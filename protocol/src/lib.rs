use std::io::{self, Read, Write};
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

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Value {
    String(String),
    Null,
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
                Self::expect_newline(stream)?;
                let value = String::from_utf8(value)
                    .map_err(|it| Error::BadMessage(BadMessageError::Utf8(it)))?;
                let value = Value::String(value);
                Ok(value)
            }
            b'_' => {
                Self::expect_newline(stream)?;
                Ok(Value::Null)
            }
            b'-' => {
                Self::expect_newline(stream)?;
                Ok(Value::Null)
            }
            b'+' => {
                let mut value = vec![];
                let mut b = [0];
                loop {
                    stream.read_exact(&mut b)?;
                    if b[0] == b'\r' {
                        break;
                    }
                    value.push(b[0]);
                }
                stream.read_exact(&mut b)?;
                assert!(b[0] == b'\n');
                let value = String::from_utf8(value)
                    .map_err(|it| Error::BadMessage(BadMessageError::Utf8(it)))?;
                Ok(Value::String(value))
            }
            c => Err(Error::UnexpectedStartOfValue(c as char)),
        }
    }

    fn expect_newline<T: Read>(stream: &mut T) -> Result<()> {
        let mut b = [0];
        stream.read_exact(&mut b)?;
        assert!(b[0] == b'\r');
        stream.read_exact(&mut b)?;
        assert!(b[0] == b'\n');
        Ok(())
    }

    pub fn write<T: Write>(&self, stream: &mut T) -> Result<()> {
        match self {
            Value::String(s) => {
                write!(stream, "${}\r\n", s.len())?;
                stream.write(s.as_bytes())?;
                stream.write(b"\r\n")?;
            }
            Value::Null => {
                stream.write(b"_\r\n")?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum BadMessageError {
    InvalidLength(String),
    Utf8(std::string::FromUtf8Error),
    InvalidCommand(String),
}
#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    BadMessage(BadMessageError),
    UnexpectedStartOfValue(char),
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, PartialEq)]
pub enum Command {
    Set(Value, Value),
    Get(Value),
}
impl Command {
    pub fn read<T: Read>(stream: &mut T) -> Result<Command> {
        let command = Value::read(stream)?;
        match command {
            Value::String(s) => match s.as_str() {
                "SET" => Ok(Command::Set(Value::read(stream)?, Value::read(stream)?)),
                "GET" => Ok(Command::Get(Value::read(stream)?)),
                _ => Err(Error::BadMessage(BadMessageError::InvalidLength(s))),
            },
            Value::Null => Err(Error::BadMessage(BadMessageError::InvalidCommand(
                String::from("-"),
            ))),
        }
    }

    pub fn write<T: Write>(&self, stream: &mut T) -> Result<()> {
        match self {
            Command::Set(key, value) => {
                Value::from("SET").write(stream)?;
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
        let command = Command::Set(Value::from("key"), Value::from("value"));
        command.write(&mut output)?;

        let read_command = Command::read(&mut &output[..])?;
        assert_eq!(read_command, command);
        Ok(())
    }

    #[test]
    fn can_read_simple_strings() -> Result<()> {
        let input = b"+OK\r\n";
        let result = Value::read(&mut &input[..])?;
        assert_eq!(Value::from("OK"), result);
        Ok(())
    }

    #[test]
    fn can_read_and_write_nil() -> Result<()> {
        let input = b"_\r\n";
        let result = Value::read(&mut &input[..])?;
        assert_eq!(Value::Null, result);

        let mut output: Vec<u8> = vec![];
        Value::Null.write(&mut output)?;
        assert_eq!(output, b"_\r\n");
        Ok(())
    }
}
