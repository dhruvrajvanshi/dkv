mod codec;
mod error;
mod value;

use std::io::{self, Read, Write};

pub use codec::Result;
pub use error::BadMessageError;
pub use error::Error;
pub use value::Value;

pub struct LoggingStream<T: Write + Read> {
    stream: T,
}
impl<T: Write + Read> LoggingStream<T> {
    pub fn new(stream: T) -> LoggingStream<T> {
        LoggingStream { stream }
    }
}
impl<T: Write + Read> Write for LoggingStream<T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        println!("w: {}", String::from_utf8_lossy(buf));
        self.stream.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}
impl<T: Write + Read> Read for LoggingStream<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.stream.read(buf)?;
        println!("r: {}", String::from_utf8_lossy(&buf[..n]));
        Ok(n)
    }
}

#[derive(Debug, PartialEq)]
pub enum Command {
    Set(String, Value),
    Get(String),
}
impl Command {
    pub fn read<T: Read>(stream: &mut T) -> codec::Result<Command> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_parse_bulk_string() -> Result<()> {
        let input = b"$5\r\nhello\r\n";
        let result = codec::read(&mut &input[..])?;
        assert_eq!(Value::String("hello".to_string()), result);
        Ok(())
    }

    #[test]
    fn can_write_bulk_string() -> Result<()> {
        let mut output: Vec<u8> = vec![];
        let value = Value::String("hello".to_string());
        codec::write(&value, &mut output)?;
        assert_eq!(output, b"$5\r\nhello\r\n");
        Ok(())
    }

    #[test]
    fn can_read_and_write_commands() -> Result<()> {
        let mut output: Vec<u8> = vec![];
        let command = Command::Set("key".to_owned(), Value::from("value"));
        command.write(&mut output)?;

        let read_command = Command::read(&mut &output[..])?;
        assert_eq!(read_command, command);
        Ok(())
    }

    #[test]
    fn can_read_simple_strings() -> Result<()> {
        let input = b"+OK\r\n";
        let result = codec::read(&mut &input[..])?;
        assert_eq!(Value::from("OK"), result);
        Ok(())
    }

    #[test]
    fn can_read_and_write_nil() -> Result<()> {
        let input = b"_\r\n";
        let result = codec::read(&mut &input[..])?;
        assert_eq!(Value::Null, result);

        let mut output: Vec<u8> = vec![];
        codec::write(&Value::Null, &mut output)?;
        assert_eq!(output, b"_\r\n");
        Ok(())
    }

    #[test]
    fn can_read_and_write_arrays() -> Result<()> {
        let input = b"*3\r\n$3\r\nfoo\r\n$3\r\nbar\r\n_\r\n";
        let result = codec::read(&mut &input[..])?;
        assert_eq!(
            Value::Array(vec![Value::from("foo"), Value::from("bar"), Value::Null]),
            result
        );
        Ok(())
    }
}
