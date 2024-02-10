use std::io::{Read, Write};

use crate::{
    codec::{self, Result},
    Error, Value,
};

#[derive(Debug, PartialEq)]
pub enum Command {
    Set(String, Value),
    Get(String),
}
impl Command {
    pub fn read<T: Read>(stream: &mut T) -> Result<Command> {
        let command = codec::read(stream)?;
        match command {
            Value::Array(values) => {
                if values.len() == 0 {
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
                                return Err(Error::generic(
                                    "First argument of a set command must be a string",
                                    "",
                                ));
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
                        c => Err(Error::generic("Invalid command", c)),
                    },
                    _ => Err(Error::generic("Command must be a string", "")),
                }
            }
            _ => Err(Error::generic("Command must be an array", "")),
        }
    }

    pub fn write<T: Write>(&self, stream: &mut T) -> Result<()> {
        match self {
            Command::Set(key, value) => {
                codec::write(
                    &Value::Array(vec![
                        Value::from("SET"),
                        Value::String(key.clone()),
                        value.clone(),
                    ]),
                    stream,
                )?;
            }
            Command::Get(key) => {
                let array = Value::Array(vec![Value::from("GET"), Value::String(key.clone())]);
                codec::write(&array, stream)?;
            }
        }
        Ok(())
    }
}
