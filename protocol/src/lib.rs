use std::io::{self, Read};

#[derive(Debug, PartialEq)]
pub enum Value {
    String(String),
}
impl Value {
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

#[derive(Debug)]
pub enum Command {
    Put(Value, Value),
    Get(Value),
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
}
