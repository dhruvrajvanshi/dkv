use std::io::{self, Read, Write};

use crate::{
    error::{BadMessageError, Error},
    value::Value,
};

pub type Result<T> = std::result::Result<T, Error>;

pub fn read<T: Read>(stream: &mut T) -> Result<Value> {
    let mut buf = [0];
    stream.read_exact(&mut buf)?;
    match buf[0] {
        b'$' => {
            let len = parse_length(stream)?;
            let mut value = vec![0; len];
            stream.read_exact(&mut value)?;
            expect_newline(stream)?;
            let value = String::from_utf8(value)
                .map_err(|it| Error::BadMessage(BadMessageError::Utf8(it)))?;
            let value = Value::String(value);
            Ok(value)
        }
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
