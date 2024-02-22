use std::{
    collections::HashMap,
    io::{self, Read, Write},
};

use crate::{
    command::Command,
    db::DB,
    dkv_array,
    error::{BadMessageError, Error},
    serializable::{Deserializable, Serializable},
    value::Value,
    Result,
};

use crate::command::make_command_docs;

#[derive(Debug, Copy, Clone)]
enum Protocol {
    RESP2,
    RESP3,
}

pub struct Connection<R: Read, W: Write> {
    db: DB,
    reader: R,
    writer: W,
    protocol: Protocol,
}
impl<R: Read, W: Write> Connection<R, W> {
    pub fn new(db: DB, reader: R, writer: W) -> Connection<R, W> {
        Connection {
            db,
            reader,
            writer,
            protocol: Protocol::RESP2,
        }
    }
    pub fn handle(&mut self) -> std::io::Result<()> {
        loop {
            match self._handle() {
                Ok(()) => {}
                Err(Error::Io(ref e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    // When redis-cli quits, it just closes the connection,
                    // which means when we try to read the next command, we get
                    // an UnexpectedEof error. We should just break the loop to
                    // handle this case.
                    break;
                }
                Err(e) => {
                    eprintln!("Error: {:?}", e);
                    write!(self.writer, "-ERROR: {}\r\n", to_simple_string(e))?;
                }
            }
        }
        Ok(())
    }
    fn _handle(&mut self) -> Result<()> {
        let command = Command::read(&mut self.reader)?;
        match command {
            Command::Hello(version) => {
                if version == "3" {
                    self.protocol = Protocol::RESP3;
                    let mut map = HashMap::new();
                    {
                        let mut put = |k, v| {
                            map.insert(String::from(k), v);
                        };
                        put("server", Value::from("dkv"));
                        put("version", Value::from("0.1.0"));
                        put("proto", Value::Integer(3));
                        put("id", Value::Integer(10));
                        put("mode", Value::from("standalone"));
                        put("role", Value::from("master"));
                        put("modules", Value::Array(vec![]));
                    }

                    self.write_value(&Value::Map(map))?;
                } else {
                    self.write_error("Invalid protocol version")?;
                }
            }
            Command::Set(key, value) => {
                self.db.set(key, Value::from(value));
                self.write_simple_string("OK")?;
            }
            Command::Get(key) => match self.db.get_optional(&key) {
                Some(value @ Value::String(_)) => {
                    value.write(&mut self.writer)?;
                }
                Some(_) => self.write_error("WRONGTYPE")?,
                None => {
                    self.write_null_response()?;
                }
            },
            Command::Command(args) => {
                if args[0].clone().as_str() == "DOCS" {
                    let subcommand = args.get(1);
                    if subcommand.is_none() {
                        let command_docs = make_command_docs();
                        Value::Map(command_docs).write(&mut self.writer)?;
                    } else {
                        todo!("COMMAND DOCS is not implement for subcommands yet")
                    }
                } else {
                    todo!("Unimplement COMMAND {:?}", args[0])
                }
            }
            Command::Config(args) => {
                if args[0] == "GET" {
                    if let Some(key) = args.get(1) {
                        let config = get_default_config();
                        let default_reply = Value::Map(HashMap::new());
                        if !config.contains_key(key.as_str()) {
                            println!("invalid config key: {:?}", key);
                        }
                        let value = config.get(key.as_str()).unwrap_or(&default_reply);
                        value.write(&mut self.writer)?;
                    } else {
                        todo!("Unimplement CONFIG GET {:?}", args[1])
                    }
                    Value::Map(HashMap::new()).write(&mut self.writer)?
                } else {
                    todo!("Unimplement CONFIG {:?}", args[0])
                }
            }
            Command::Ping(s) => Value::from(s).write(&mut self.writer)?,
            Command::FlushAll => {
                self.db.flush_all();
                self.write_simple_string("OK")?;
            }
            Command::Del(key) => {
                let num_keys_deleted = self.db.del(&key);
                Value::Integer(num_keys_deleted as i64).write(&mut self.writer)?
            }
            Command::ClientSetInfo(_, _) => {
                self.write_simple_string("OK")?;
            }
            Command::Rename(old_key, new_key) => {
                match self.db.get_optional(&old_key) {
                    Some(value) => {
                        self.db.set(new_key, value);
                        self.db.del(&old_key);
                        self.write_simple_string("OK")?;
                    }
                    None => {
                        self.write_error("NO_SUCH_KEY")?;
                    }
                };
            }
            Command::HGet { key, field } => {
                enum R {
                    Found(Value),
                    NotFound,
                    WrongType,
                }
                let result = self.db.view(&key, |v| match v {
                    Some(Value::Map(m)) => {
                        if let Some(value) = m.get(&field) {
                            R::Found(value.clone())
                        } else {
                            R::NotFound
                        }
                    }
                    _ => R::WrongType,
                });
                match result {
                    R::Found(value) => self.write_value(&value)?,
                    R::NotFound => self.write_null_response()?,
                    R::WrongType => self.write_error("WRONG_TYPE")?,
                }
            }
            Command::HSet { key, field, value } => {
                enum R {
                    NewMap,
                    Mutated,
                    WrongKey,
                }
                let result = self.db.mutate(&key, |v| match v {
                    None => R::NewMap,
                    Some(Value::Map(m)) => {
                        m.insert(field.clone(), value.clone());
                        R::Mutated
                    }

                    Some(_) => R::WrongKey,
                });

                match result {
                    R::Mutated => self.write_value(&Value::Integer(1))?,
                    R::NewMap => {
                        let mut map = HashMap::new();
                        map.insert(field, value);
                        self.db.set(key, Value::Map(map));
                        self.write_value(&Value::Integer(1))?
                    }
                    R::WrongKey => self.write_error("WRONG_KEY")?,
                }
            }
            Command::Exists(key) => {
                let exists = self.db.exists(&key);
                if exists {
                    self.write_value(&Value::Integer(1))?;
                } else {
                    self.write_value(&Value::Integer(0))?;
                }
            }
            Command::HGetAll(key) => {
                if let Some(Value::Map(m)) = self.db.get_optional(&key) {
                    match self.protocol {
                        Protocol::RESP2 => {
                            let mut values = vec![];
                            for (k, v) in m {
                                values.push(Value::String(k));
                                values.push(v);
                            }
                            self.write_value(&Value::Array(values))?;
                        }
                        Protocol::RESP3 => self.write_value(&Value::Map(m))?,
                    }
                } else {
                    match self.protocol {
                        Protocol::RESP2 => self.write_value(&dkv_array![])?,
                        Protocol::RESP3 => self.write_value(&Value::Map(HashMap::new()))?,
                    }
                }
            }
            Command::HLen(ref key) => {
                let len = self.db.view(key, |v| match v {
                    Some(Value::Map(m)) => m.len() as i64,
                    _ => 0,
                });
                self.write_value(&Value::Integer(len))?;
            }
            Command::HExists { ref key, ref field } => {
                let exists = self.db.view(key, |v| match v {
                    Some(Value::Map(m)) => m.contains_key(field),
                    _ => false,
                });
                let result = if exists {
                    Value::Integer(1)
                } else {
                    Value::Integer(0)
                };
                self.write_value(&result)?;
            }
        }
        Ok(())
    }

    fn write_error(&mut self, s: &str) -> Result<()> {
        write!(self.writer, "-ERROR: {}\r\n", s)?;
        Ok(())
    }

    fn write_value(&mut self, value: &Value) -> io::Result<()> {
        value.write(&mut self.writer)?;
        Ok(())
    }

    /// RESP2 doesn't have a Null representation
    /// instead, it uses a bulk string/array with -1 length
    /// depending on context
    fn write_null_response(&mut self) -> io::Result<()> {
        match self.protocol {
            Protocol::RESP2 => {
                write!(self.writer, "$-1\r\n")?;
            }
            Protocol::RESP3 => self.write_value(&Value::Null)?,
        }
        Ok(())
    }

    fn write_simple_string(&mut self, value: &str) -> Result<()> {
        write!(&mut self.writer, "+{}\r\n", value)?;
        Ok(())
    }
}

fn get_default_config() -> HashMap<&'static str, Value> {
    let mut config = HashMap::new();
    config.insert("save", Value::from("3600 1 300 100 60 10000"));
    config.insert("appendonly", Value::from("no"));
    config.insert("bind", Value::from("localhost"));
    config
}

fn to_simple_string(e: Error) -> String {
    match e {
        // there's no guarantee that io::Error contains characters that are safe
        // to send as part of a simple string, so we'll just send a generic error
        // Besides, this is treated as a server error, not client error.
        Error::Io(_) => String::from("Internal server error"),
        Error::BadMessage(BadMessageError::InvalidCommand(_)) => String::from("Invalid command"),
        Error::BadMessage(BadMessageError::InvalidLength(_)) => {
            String::from("Invalid length for a bulk string")
        }
        Error::BadMessage(BadMessageError::Generic(s, _)) => s,
        Error::BadMessage(BadMessageError::Utf8(_)) => String::from("Invalid UTF-8"),
        Error::UnexpectedStartOfValue(c) => {
            format!("Unexpected start of value: {}", c)
        }
    }
}

#[cfg(test)]
mod test {
    use std::vec;

    use crate::codec;

    use super::*;
    #[test]
    fn handles_ping() {
        let input = b"*1\r\n$4\r\nPING\r\n";
        let mut output = vec![];
        let mut conn = Connection::new(DB::new(), &input[..], &mut output);
        conn.handle().unwrap();

        let value = codec::read(&mut &output[..]).unwrap();

        assert_eq!(value, Value::from("PONG"));
    }

    #[test]
    fn handles_ping_with_argument() {
        let input = b"*2\r\n$4\r\nPING\r\n$4\r\nPING\r\n";
        let mut output = vec![];
        let mut conn = Connection::new(DB::new(), &input[..], &mut output);
        conn.handle().unwrap();

        let value = codec::read(&mut &output[..]).unwrap();

        assert_eq!(value, Value::from("PING"));
    }
}
