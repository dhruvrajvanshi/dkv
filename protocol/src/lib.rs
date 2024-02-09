use std::io::{self, Read, Write};

#[derive(Debug, PartialEq)]
pub enum Value {
    String(String),
}
impl Value {
    pub fn from(value: &str) -> Value {
        Value::String(value.to_string())
    }

    pub fn read<T: Read>(stream: &mut T) -> Result<Value> {
        let mut buf = [0];
        stream.read_exact(&mut buf)?;
        match buf[0] {
            b'$' => {
                let mut len = vec![];
                let mut b = [0];
                loop {
                    stream.read_exact(&mut b)?;
                    if b[0] == b'\r' {
                        break;
                    }
                    len.push(b[0]);
                }
                stream.read_exact(&mut b)?;
                assert!(b[0] == b'\n');
                let len = String::from_utf8(len)
                    .map_err(|it| Error::BadMessage(BadMessageError::Utf8(it)))?;
                let len = len
                    .parse::<usize>()
                    .map_err(|_| Error::BadMessage(BadMessageError::InvalidLength(len)))?;
                let mut value = vec![0; len];
                stream.read_exact(&mut value)?;
                let mut ignore = [0, 0];
                stream.read_exact(&mut ignore)?;
                assert!(&ignore == b"\r\n", "Invalid terminator");
                Ok(Value::String(String::from_utf8(value).map_err(|it| {
                    Error::BadMessage(BadMessageError::Utf8(it))
                })?))
            }
            _ => panic!("Invalid value"),
        }
    }

    pub fn write<T: Write>(&self, stream: &mut T) -> Result<()> {
        match self {
            Value::String(s) => {
                write!(stream, "${}\r\n", s.len())?;
                stream.write(s.as_bytes())?;
                stream.write(b"\r\n")?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum BadMessageError {
    InvalidLength(String),
    Utf8(std::string::FromUtf8Error),
}
#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    BadMessage(BadMessageError),
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, PartialEq)]
pub enum Command {
    Put(Value, Value),
    Get(Value),
}
impl Command {
    pub fn read<T: Read>(stream: &mut T) -> Result<Command> {
        let command = Value::read(stream)?;
        match command {
            Value::String(s) => match s.as_str() {
                "PUT" => Ok(Command::Put(Value::read(stream)?, Value::read(stream)?)),
                "GET" => Ok(Command::Get(Value::read(stream)?)),
                _ => Err(Error::BadMessage(BadMessageError::InvalidLength(s))),
            },
        }
    }

    pub fn write<T: Write>(&self, stream: &mut T) -> Result<()> {
        match self {
            Command::Put(key, value) => {
                Value::from("PUT").write(stream)?;
                key.write(stream)?;
                value.write(stream)?;
            }
            Command::Get(key) => {
                Value::from("GET").write(stream)?;
                key.write(stream)?;
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
        let result = Value::read(&mut &input[..])?;
        assert_eq!(Value::String("hello".to_string()), result);
        Ok(())
    }

    #[test]
    fn can_write_bulk_string() -> Result<()> {
        let mut output: Vec<u8> = vec![];
        let value = Value::String("hello".to_string());
        value.write(&mut output)?;
        assert_eq!(output, b"$5\r\nhello\r\n");
        Ok(())
    }

    #[test]
    fn can_read_and_write_commands() -> Result<()> {
        let mut output: Vec<u8> = vec![];
        let command = Command::Put(Value::from("key"), Value::from("value"));
        command.write(&mut output)?;
        assert_eq!(output, b"PUT$3\r\nkey\r\n$5\r\nvalue\r\n");
        let input = b"PUT$3\r\nkey\r\n$5\r\nvalue\r\n";
        let result = Command::read(&mut &input[..])?;
        assert_eq!(command, result);
        Ok(())
    }
}
