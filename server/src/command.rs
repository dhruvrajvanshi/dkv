use std::io::Read;

use crate::{
    codec::{self, Result},
    Error, Value,
};

#[derive(Debug, PartialEq)]
pub enum Command {
    Set(String, Value),
    Get(String),
    Command(Vec<Value>),
    Config(Vec<Value>),
}
impl Command {
    pub fn read<T: Read>(stream: &mut T) -> Result<Command> {
        let command = codec::read(stream)?;
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
                    Value::String(s) => match s.as_str() {
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
                        "COMMAND" => {
                            let args = values[1..].to_vec();
                            Ok(Command::Command(args))
                        }
                        "CONFIG" => {
                            let args = values[1..].to_vec();

                            Ok(Command::Config(args))
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
