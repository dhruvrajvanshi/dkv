use thiserror::Error;
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::bytestr::ByteStr;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Parse error: {0}")]
    InvalidMessage(String),
}
pub type Result<T> = std::result::Result<T, ParseError>;
pub async fn parse_array<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Vec<ByteStr>> {
    expect_char(reader, b'*').await?;
    let len = parse_len(reader).await?;
    let mut arr = Vec::with_capacity(len);
    for _ in 0..len {
        arr.push(parse_bulk_string(reader).await?);
    }
    Ok(arr)
}

pub async fn parse_bulk_string<R: AsyncRead + Unpin>(reader: &mut R) -> Result<ByteStr> {
    expect_char(reader, b'$').await?;
    let len = parse_len(reader).await?;
    let mut buf = vec![0; len];
    reader.read_exact(&mut buf).await?;
    expect_char(reader, b'\r').await?;
    expect_char(reader, b'\n').await?;
    Ok(ByteStr::from(buf))
}

async fn expect_char<R: AsyncRead + Unpin>(reader: &mut R, expected: u8) -> Result<u8> {
    let c = reader.read_u8().await?;
    if c != expected {
        Err(ParseError::InvalidMessage(format!(
            "Expected '{}', found '{}'",
            expected as char, c as char
        )))
    } else {
        Ok(c)
    }
}

pub async fn parse_len<R: AsyncRead + Unpin>(reader: &mut R) -> Result<usize> {
    let mut buf = String::new();
    loop {
        let char = reader.read_u8().await?;
        if char == b'\r' {
            expect_char(reader, b'\n').await?;
            break;
        } else {
            buf.push(char as char);
        }
    }
    Ok(buf
        .parse()
        .map_err(|_| ParseError::InvalidMessage("Invalid length".to_string()))?)
}

pub async fn write_error_string<W: AsyncWrite + Unpin>(
    writer: &mut W,
    message: &str,
) -> tokio::io::Result<()> {
    writer.write_all(b"-ERR ").await?;
    writer.write_all(message.as_bytes()).await?;
    writer.write_all(b"\r\n").await?;
    Ok(())
}

pub async fn write_simple_string<W: AsyncWrite + Unpin>(
    writer: &mut W,
    message: &str,
) -> tokio::io::Result<()> {
    writer.write_all(b"+").await?;
    writer.write_all(message.as_bytes()).await?;
    writer.write_all(b"\r\n").await?;
    Ok(())
}
