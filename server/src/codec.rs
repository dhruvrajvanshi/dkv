use std::{
    collections::HashMap,
    io::{self, Read, Write},
};

use crate::{
    error::{BadMessageError, Error},
    value::Value,
};

pub type Result<T> = std::result::Result<T, Error>;

pub fn read<T: Read>(stream: &mut T) -> Result<Value> {
    let mut buf = [0];
    stream.read_exact(&mut buf)?;
    match buf[0] {
        b'$' => Ok(Value::String(read_bulk_string_tail(stream)?)),
        b'_' => {
            expect_newline(stream)?;
            Ok(Value::Null)
        }
        b'-' => {
            expect_newline(stream)?;
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
        b'*' => {
            let len = parse_length(stream)?;
            let mut values = vec![];
            for _ in 0..len {
                values.push(read(stream)?);
            }
            Ok(Value::Array(values))
        }
        b'%' => {
            let len = parse_length(stream)?;
            let mut map = HashMap::new();
            for _ in 0..len {
                let key = read_bulk_string(stream)?;
                let value = read(stream)?;
                map.insert(key, value);
            }
            Ok(Value::Map(map))
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

pub fn write<T: Write>(value: &Value, stream: &mut T) -> io::Result<()> {
    match value {
        Value::String(s) => {
            write_bulk_string(stream, s.as_str())?;
        }
        Value::Null => {
            stream.write(b"_\r\n")?;
        }
        Value::Array(values) => {
            write!(stream, "*{}\r\n", values.len())?;
            for value in values {
                write(value, stream)?;
            }
        }
        Value::Map(map) => {
            write!(stream, "%{}\r\n", map.len())?;
            for (key, value) in map {
                write_bulk_string(stream, key.as_str())?;
                write(value, stream)?;
            }
        }
    }
    Ok(())
}

fn parse_length<T: Read>(stream: &mut T) -> Result<usize> {
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
    let len = String::from_utf8(len).map_err(|it| Error::BadMessage(BadMessageError::Utf8(it)))?;
    len.parse::<usize>()
        .map_err(|_| Error::BadMessage(BadMessageError::InvalidLength(len)))
}

fn write_bulk_string<T: Write>(stream: &mut T, s: &str) -> io::Result<()> {
    write!(stream, "${}\r\n", s.len())?;
    stream.write(s.as_bytes())?;
    stream.write(b"\r\n")?;
    Ok(())
}
fn read_bulk_string_tail<T: Read>(stream: &mut T) -> Result<String> {
    let len = parse_length(stream)?;
    let mut value = vec![0; len];
    stream.read_exact(&mut value)?;
    expect_newline(stream)?;
    String::from_utf8(value).map_err(|it| Error::BadMessage(BadMessageError::Utf8(it)))
}

fn read_bulk_string<T: Read>(stream: &mut T) -> Result<String> {
    let mut buf = [0];
    stream.read_exact(&mut buf)?;
    if buf[0] != b'$' {
        return Err(Error::generic("Expected a string", format!("{:?}", buf[0])));
    }
    read_bulk_string_tail(stream)
}

#[cfg(test)]
mod test {

    use crate::Command;

    use super::*;
    #[test]
    fn can_read_and_write_map() {
        let mut map = std::collections::HashMap::new();
        map.insert("hello".to_string(), Value::String("world".to_string()));
        let mut buf = vec![];
        write(&Value::Map(map), &mut buf).unwrap();

        dbg!(String::from_utf8_lossy(&buf));
        let value = read(&mut &buf[..]).expect("Foo");
        if let Value::Map(map) = value {
            assert_eq!(map.len(), 1);
            assert_eq!(
                map.get("hello").unwrap(),
                &Value::String("world".to_string())
            );
        } else {
            panic!("Expected a map");
        }
    }

    #[test]
    fn can_parse_bulk_string() -> Result<()> {
        let input = b"$5\r\nhello\r\n";
        let result = read(&mut &input[..])?;
        assert_eq!(Value::String("hello".to_string()), result);
        Ok(())
    }

    #[test]
    fn can_write_bulk_string() -> Result<()> {
        let mut output: Vec<u8> = vec![];
        let value = Value::String("hello".to_string());
        write(&value, &mut output)?;
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
        let result = read(&mut &input[..])?;
        assert_eq!(Value::from("OK"), result);
        Ok(())
    }

    #[test]
    fn can_read_and_write_nil() -> Result<()> {
        let input = b"_\r\n";
        let result = read(&mut &input[..])?;
        assert_eq!(Value::Null, result);

        let mut output: Vec<u8> = vec![];
        write(&Value::Null, &mut output)?;
        assert_eq!(output, b"_\r\n");
        Ok(())
    }

    #[test]
    fn can_read_and_write_arrays() -> Result<()> {
        let input = b"*3\r\n$3\r\nfoo\r\n$3\r\nbar\r\n_\r\n";
        let result = read(&mut &input[..])?;
        assert_eq!(
            Value::Array(vec![Value::from("foo"), Value::from("bar"), Value::Null]),
            result
        );
        Ok(())
    }
}
