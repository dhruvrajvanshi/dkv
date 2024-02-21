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
                self.db.set(key, value);
                Self::_write_simple_string(&mut self.writer, "OK")?;
            }
            Command::Get(key) => {
                if let Some(value) = self.db.get_optional(&key) {
                    value.write(&mut self.writer)?;
                } else {
                    self.write_null_response()?;
                }
            }
            Command::Command(args) => {
                if args[0].clone().as_str() == Some("DOCS") {
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
                if let Some("GET") = args[0].as_str() {
                    if let Some(key) = args[1].as_str() {
                        let config = get_default_config();
                        let default_reply = Value::Map(HashMap::new());
                        if !config.contains_key(key) {
                            println!("invalid config key: {:?}", key);
                        }
                        let value = config.get(key).unwrap_or(&default_reply);
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
                Self::_write_simple_string(&mut self.writer, "OK")?;
            }
            Command::Del(key) => {
                let num_keys_deleted = self.db.del(&key);
                Value::Integer(num_keys_deleted as i64).write(&mut self.writer)?
            }
            Command::ClientSetInfo(_, _) => {
                Self::_write_simple_string(&mut self.writer, "OK")?;
            }
            Command::Rename(old_key, new_key) => {
                let key_exists = self.db.exists(&old_key);
                if !key_exists {
                    self.write_error("NO_SUCH_KEY")?;
                } else {
                    let value = self.db.get(&old_key);
                    self.db.set(new_key, value);
                    self.db.del(&old_key);
                    self.write_simple_string("OK")?;
                }
            }
            Command::HGet { key, field } => match self.db.get(&key) {
                Value::Map(m) => {
                    if let Some(value) = m.get(&field) {
                        self.write_value(value)?;
                    } else {
                        self.write_null_response()?;
                    }
                }
                _ => {
                    self.write_error("WRONG_TYPE")?;
                }
            },
            Command::HSet { key, field, value } => {
                let existing = self.db.get(&key);
                match existing {
                    Value::Map(mut map) => {
                        map.insert(field.clone(), value.clone());
                        self.db.set(key.clone(), Value::Map(map));
                        self.write_value(&Value::Integer(1))?
                    }
                    Value::Null => {
                        let mut map = HashMap::new();
                        map.insert(field.clone(), value.clone());
                        self.db.set(key.clone(), Value::Map(map));
                        self.write_value(&Value::Integer(1))?
                    }
                    _ => self.write_error("WRONG_KEY")?,
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
        Self::_write_simple_string(&mut self.writer, value)
    }

    fn _write_simple_string(stream: &mut W, s: &str) -> Result<()> {
        write!(stream, "+{}\r\n", s)?;
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
