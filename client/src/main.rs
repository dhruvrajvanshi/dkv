use dkv_protocol::{Result, Value};
use std::net::TcpStream;

fn main() -> dkv::Result<()> {
    let stream = TcpStream::connect("localhost:6543")?;
    let mut djs = dkv::Client::new(stream);
    djs.put(b"key", b"value")?;
    let value = djs.get(b"key")?;
    assert_eq!(value, Some(b"value".into()));
    return Ok(());
}
mod dkv {
    use dkv_protocol::{self, Value};
    use std::io::{self, Read, Write};
    use std::net::TcpStream;

    pub type Result<T> = dkv_protocol::Result<T>;
    pub struct Client {
        stream: TcpStream,
    }
    impl Client {
        pub fn new(stream: TcpStream) -> Client {
            Client { stream }
        }

        pub fn put(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
            self.stream.write(b"PUT ")?;
            self.write_bulk_string(key)?;
            self.write_bulk_string(value)?;
            return Ok(());
        }
        pub fn get(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>> {
            self.stream.write(b"GET ")?;
            self.write_bulk_string(key)?;
            return self.read_get_result();
        }

        fn write_bulk_string(&mut self, value: &[u8]) -> Result<()> {
            write!(self.stream, "${}\r\n", value.len())?;
            self.stream.write(value)?;
            self.stream.write(b"\r\n")?;
            return Ok(());
        }

        fn read_get_result(&mut self) -> Result<Option<Vec<u8>>> {
            let value = Value::read(&mut self.stream)?;
            match value {
                Value::String(value) => Ok(Some(value.into_bytes())),
                _ => panic!("Invalid value"),
            }
        }
    }
}
