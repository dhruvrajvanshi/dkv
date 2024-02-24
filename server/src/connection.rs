use std::{
    collections::HashMap,
    io::{self, Write},
    net::TcpStream,
    sync::mpsc,
};

use crate::{
    codec::{self, write_bulk_string},
    command::Command,
    error::{BadMessageError, Error},
    serializable::{Deserializable, Serializable},
    value::Value,
    Result,
};

use crate::command::make_command_docs;
use db::DB;
use dkv_db as db;

#[derive(Debug, Copy, Clone)]
enum Protocol {
    RESP2,
    RESP3,
}

pub struct Connection {
    db: DB,
    tcp_stream: TcpStream,
    protocol: Protocol,
}
impl Connection {
    pub fn new(db: DB, stream: TcpStream) -> Connection {
        Connection {
            db,
            tcp_stream: stream,
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
                    self.write_error(&to_simple_string(e))?;
                }
            }
        }
        Ok(())
    }
    fn read_command(&mut self) -> Result<Command> {
        Command::read(&mut self.tcp_stream)
    }

    fn _handle(&mut self) -> Result<()> {
        let command = self.read_command()?;
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
                self.db.set(key, db::Value::from(value));
                self.write_simple_string("OK")?;
            }
            Command::Get(key) => match self.db.get_optional(&key) {
                Some(value @ db::Value::String(_)) => {
                    self.write(&value)?;
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
                        self.write_value(&Value::Map(command_docs))?;
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
                        self.write_value(value)?;
                    } else {
                        todo!("Unimplement CONFIG GET {:?}", args[1])
                    }
                    self.write_value(&Value::Map(HashMap::new()))?
                } else {
                    todo!("Unimplement CONFIG {:?}", args[0])
                }
            }
            Command::Ping(s) => self.write_value(&Value::from(s))?,
            Command::FlushAll => {
                self.db.flush_all();
                self.write_simple_string("OK")?;
            }
            Command::Del(key) => {
                let num_keys_deleted = self.db.del(&key);
                self.write_value(&Value::Integer(num_keys_deleted as i64))?
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
                    Found(String),
                    NotFound,
                    WrongType,
                }
                let result = self.db.view(&key, |v| match v {
                    Some(db::Value::Hash(m)) => {
                        if let Some(value) = m.get(&field) {
                            R::Found(value.clone())
                        } else {
                            R::NotFound
                        }
                    }
                    _ => R::WrongType,
                });
                match result {
                    R::Found(value) => self.write_bulk_string(&value)?,
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
                    Some(db::Value::Hash(m)) => {
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
                        self.db.set(key, db::Value::Hash(map));
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
                let map = match self.db.get_optional(&key) {
                    Some(db::Value::Hash(m)) => m,
                    _ => HashMap::new(),
                };
                match self.protocol {
                    Protocol::RESP2 => {
                        let mut values = vec![];
                        for (k, v) in &map {
                            values.push(k.as_str());
                            values.push(v.as_str());
                        }
                        println!("WRITE_ARRAY: {:?}", values);
                        self.write_array(values.as_slice())?;
                    }
                    Protocol::RESP3 => {
                        println!("WRITE_HASH: {:?}", map);
                        self.write(&db::Value::Hash(map))?;
                    }
                }
            }
            Command::HLen(ref key) => {
                let len = self.db.view(key, |v| match v {
                    Some(db::Value::Hash(m)) => m.len() as i64,
                    _ => 0,
                });
                self.write_value(&Value::Integer(len))?;
            }
            Command::HExists { ref key, ref field } => {
                let exists = self.db.view(key, |v| match v {
                    Some(db::Value::Hash(m)) => m.contains_key(field),
                    _ => false,
                });
                let result = if exists {
                    Value::Integer(1)
                } else {
                    Value::Integer(0)
                };
                self.write_value(&result)?;
            }
            Command::Subscribe(channels) => {
                self.handle_subscribe(channels)?;
            }
            Command::Publish(channel, message) => {
                self.db.publish(&channel, &message);
                self.write_value(&Value::Integer(1))?;
            }
            Command::Unsubscribe(_) => {
                self.write_error("Unsubscribe called outside of a subscription connection")?;
            }
        }
        Ok(())
    }

    fn handle_subscribe(&mut self, channels: Vec<String>) -> Result<()> {
        let mut subscriptions_by_channel = HashMap::new();
        let (send_value, recv_value) = mpsc::channel();
        for channel in channels {
            let send_value = send_value.clone();
            let sub = self.db.subscribe(&channel, move |message| {
                // deliberately ignore error because if we're unable to send a value
                // that just means that the client has disconnected by calling
                // unsubscribe
                let _ = send_value.send(Value::Array(vec![
                    Value::from("message"),
                    Value::from(message.channel.to_string()),
                    Value::from(message.value.to_string()),
                ]));
            });
            subscriptions_by_channel.insert(channel, sub);
        }

        loop {
            if let Ok(value) = recv_value.try_recv() {
                self.write_value(&value)?;
            }
            let command = self.try_read_command()?;
            match command {
                Some(Command::Unsubscribe(channels)) => {
                    for channel in channels {
                        if let Some(sub) = subscriptions_by_channel.remove(&channel) {
                            self.db.unsubscribe(sub);
                        }
                        self.write_value(&Value::Array(vec![
                            Value::from("unsubscribe"),
                            Value::from(channel),
                            // TODO
                            Value::from(subscriptions_by_channel.len() as i64),
                        ]))?;
                    }
                    if subscriptions_by_channel.is_empty() {
                        break;
                    }
                }
                Some(_) => {
                    self.write_error("Only unsubscribe commands can be sent after SUBSCRIBE")?
                }
                None => continue,
            }
        }

        Ok(())
    }

    /// Try to read a command from the tcp stream, but return None
    /// in case the call would block.
    fn try_read_command(&mut self) -> Result<Option<Command>> {
        self.tcp_stream.set_nonblocking(true)?;

        let value = match Command::read(&mut self.tcp_stream) {
            Ok(command) => Ok(Some(command)),
            Err(Error::Io(ref e)) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e),
        };
        self.tcp_stream.set_nonblocking(true)?;
        value
    }

    fn write_error(&mut self, s: &str) -> io::Result<()> {
        write!(self.tcp_stream, "-ERROR: {}\r\n", s)?;
        Ok(())
    }

    fn write_value(&mut self, value: &Value) -> io::Result<()> {
        value.write(&mut self.tcp_stream)?;
        Ok(())
    }

    fn write(&mut self, value: &impl Serializable) -> io::Result<()> {
        value.write(&mut self.tcp_stream)
    }

    fn write_bulk_string(&mut self, value: &str) -> io::Result<()> {
        codec::write_bulk_string(&mut self.tcp_stream, value)
    }

    fn write_array(&mut self, values: &[&str]) -> io::Result<()> {
        codec::write_bulk_string_array(&mut self.tcp_stream, values)
    }

    /// RESP2 doesn't have a Null representation
    /// instead, it uses a bulk string/array with -1 length
    /// depending on context
    fn write_null_response(&mut self) -> io::Result<()> {
        match self.protocol {
            Protocol::RESP2 => {
                write!(self.tcp_stream, "$-1\r\n")?;
            }
            Protocol::RESP3 => self.write_value(&Value::Null)?,
        }
        Ok(())
    }

    fn write_simple_string(&mut self, value: &str) -> Result<()> {
        write!(self.tcp_stream, "+{}\r\n", value)?;
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
impl Serializable for db::Value {
    fn write(&self, stream: &mut impl std::io::Write) -> std::io::Result<()> {
        use db::Value;
        match self {
            Value::String(s) => {
                write!(stream, "+{}\r\n", s)?;
            }
            Value::List(list) => {
                write!(stream, "*{}\r\n", list.len())?;
                for item in list {
                    write_bulk_string(stream, item)?;
                }
            }
            Value::Hash(map) => {
                write!(stream, "%{}\r\n", map.len())?;
                for (key, value) in map {
                    write_bulk_string(stream, key.as_str())?;
                    write_bulk_string(stream, value.as_str())?;
                }
            }
        }
        Ok(())
    }
}
