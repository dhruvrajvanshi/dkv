use std::io::Read;

use crate::{
    codec::Result,
    serializable::{Deserializable, Serializable},
    Error, Value,
};

#[derive(Debug, PartialEq)]
pub enum Command {
    Set(String, Value),
    Get(String),
    Del(String),
    Exists(String),
    Command(Vec<Value>),
    Config(Vec<Value>),
    Ping(String),
    FlushAll,
    ClientSetInfo(String, String),
    Rename(String, String),
    HSet {
        key: String,
        field: String,
        value: Value,
    },
    HGet {
        key: String,
        field: String,
    },
    Hello(String),
}

impl Deserializable for Command {
    type Error = Error;
    fn read(stream: &mut impl Read) -> Result<Self> {
        let command = Value::read(stream)?;
        match command {
            Value::Array(values) => {
                if values.is_empty() {
                    return Err(Error::generic(
                        "Empty array is not a valid command",
                        "".to_string(),
                    ));
                }

                let command = &values[0];
                match command {
                    Value::String(s) => match s.as_str().to_uppercase().as_str() {
                        "SET" => {
                            if values.len() != 3 {
                                return Err(Error::generic(
                                    "SET command must have 2 arguments",
                                    "",
                                ));
                            }
                            if let Value::String(v) = values[1].clone() {
                                Ok(Command::Set(v, values[2].clone()))
                            } else {
                                Err(Error::generic(
                                    "First argument of a set command must be a string",
                                    "",
                                ))
                            }
                        }
                        "GET" => {
                            if values.len() != 2 {
                                return Err(Error::generic(
                                    "GET command must have 1 argument".to_string(),
                                    "",
                                ));
                            }
                            if let Value::String(v) = values[1].clone() {
                                Ok(Command::Get(v))
                            } else {
                                Err(Error::generic(
                                    "First argument of a get command must be a string",
                                    "",
                                ))
                            }
                        }
                        "DEL" => match &values[1..] {
                            [Value::String(s)] => Ok(Command::Del(s.clone())),
                            _ => Err(Error::generic(
                                "DEL command must have 1 argument",
                                (values.len() - 1).to_string(),
                            )),
                        },
                        "COMMAND" => {
                            let args = values[1..].to_vec();
                            Ok(Command::Command(args))
                        }
                        "CLIENT" => match &values[1..] {
                            [Value::String(ref subcommand), Value::String(key), Value::String(value)] => {
                                match subcommand.as_str() {
                                    "SETINFO" => {
                                        Ok(Command::ClientSetInfo(key.clone(), value.clone()))
                                    }
                                    _ => Err(Error::generic(
                                        "Invalid client subcommand",
                                        format!("{:?}", subcommand),
                                    )),
                                }
                            }
                            _ => Err(Error::generic(
                                "Invalid client command",
                                format!("{:?}", command),
                            )),
                        },
                        "CONFIG" => {
                            let args = values[1..].to_vec();

                            Ok(Command::Config(args))
                        }
                        "PING" => match &values[1..] {
                            [Value::String(s)] => Ok(Command::Ping(s.clone())),
                            [] => Ok(Command::Ping("PONG".to_string())),
                            _ => Err(Error::generic(
                                "PING command must have 0 or 1 arguments",
                                (values.len() - 1).to_string(),
                            )),
                        },
                        "FLUSHALL" => Ok(Command::FlushAll),
                        "RENAME" => match &values[1..] {
                            [Value::String(old_key), Value::String(new_key)] => {
                                Ok(Command::Rename(old_key.clone(), new_key.clone()))
                            }
                            _ => Err(Error::generic(
                                "RENAME command must have 2 string arguments",
                                (values.len() - 1).to_string(),
                            )),
                        },
                        "HSET" => match &values[1..] {
                            [Value::String(key), field, value] => {
                                let field = match field.clone() {
                                    Value::String(field) => field,
                                    Value::Integer(field) => field.to_string(),
                                    _ => return Err(Error::generic("HSET field must be a string or number", ""))
                                };
                                Ok(Command::HSet { key: key.clone(), field, value: value.clone() })
                            }
                            _ => Err(Error::generic("HSET must be called with 2 string arguments and 1 string or number argument", ""))
                        },
                        "HGET" => match &values[1..] {
                            [Value::String(key), field] => {
                                let field = match field.clone() {
                                    Value::String(field) => field,
                                    Value::Integer(field) => field.to_string(),
                                    _ => return Err(Error::generic("HSET field must be a string or number", ""))
                                };
                                Ok(Command::HGet { key: key.clone(), field })
                            }
                            _ => Err(Error::generic("HGET must be called with 2 string arguments", ""))
                        }
                        "HELLO" => match &values[1..] {
                            [Value::String(version)] => {
                                Ok(Command::Hello(version.clone()))
                            }
                            _ => Err(Error::generic("HELLO must be called with 1 string argument", ""))
                        }
                        "EXISTS" => match &values[1..] {
                            [Value::String(key)] => {
                                Ok(Command::Exists(key.clone()))
                            }
                            _ => Err(Error::generic("EXISTS must be called with 1 string argument", ""))
                        }
                        c => Err(Error::generic("Invalid command", c)),
                    },
                    _ => Err(Error::generic("Command must be a string", "")),
                }
            }
            _ => Err(Error::generic("Command must be an array", "")),
        }
    }
}
impl Serializable for Command {
    fn write(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        use Command as c;
        use Command::{Config, Get, Ping, Set};
        match self {
            Set(key, value) => {
                Value::Array(vec![Value::from("SET"), Value::from(key), value.clone()])
                    .write(writer)
            }
            Get(key) => Value::Array(vec![Value::from("GET"), Value::from(key)]).write(writer),
            Command::Command(args) => {
                let mut values = vec![Value::from("COMMAND")];
                values.extend(args.clone());
                Value::Array(values).write(writer)
            }
            Config(args) => {
                let mut values = vec![Value::from("CONFIG")];
                values.extend(args.clone());
                Value::Array(values).write(writer)
            }
            Ping(s) => Value::Array(vec![Value::from("PING"), Value::from(s)]).write(writer),
            Command::FlushAll => Value::Array(vec![Value::from("FLUSHALL")]).write(writer),
            Command::Del(key) => {
                Value::Array(vec![Value::from("DEL"), Value::from(key)]).write(writer)
            }
            c::ClientSetInfo(key, value) => Value::Array(vec![
                Value::from("CLIENT"),
                Value::from("SETINFO"),
                Value::from(key),
                Value::from(value),
            ])
            .write(writer),
            c::Rename(old_key, new_key) => Value::Array(vec![
                Value::from("RENAME"),
                Value::from(old_key),
                Value::from(new_key),
            ])
            .write(writer),
            c::HSet { key, field, value } => Value::Array(vec![
                Value::from("HSET"),
                Value::from(key),
                Value::from(field),
                value.clone(),
            ])
            .write(writer),
            c::HGet { key, field } => Value::Array(vec![
                Value::from("HGET"),
                Value::from(key),
                Value::from(field),
            ])
            .write(writer),
            c::Hello(version) => {
                Value::Array(vec![Value::from("HELLO"), Value::from(version)]).write(writer)
            }
            c::Exists(key) => {
                Value::Array(vec![Value::from("EXISTS"), Value::from(key)]).write(writer)
            }
        }
    }
}

pub fn make_command_docs() -> std::collections::HashMap<String, Value> {
    let mut map = std::collections::HashMap::new();
    let mut set_map = std::collections::HashMap::new();
    set_map.insert("summary".to_owned(), Value::from("Set a key to a value"));
    map.insert("SET".to_string(), Value::Map(set_map));
    let mut get_map = std::collections::HashMap::new();
    get_map.insert("summary".to_owned(), Value::from("Get a value by key"));
    map.insert("GET".to_string(), Value::Map(get_map));
    map
}
