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
    use std::net::TcpStream;
    use std::io::{self, Read, Write};

    pub type Result<T> = io::Result<T>;
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
            let mut buf = [0];
            self.stream.read_exact(&mut buf)?;
            match buf[0] {
                b'-' => Ok(None),
                b'$' => {
                    let mut len = vec![];
                    let mut b = [0];
                    loop {
                        self.stream.read_exact(&mut b)?;
                        if b[0] == b'\r' {
                            break;
                        }
                        len.push(b[0]);
                    }
                    self.stream.read_exact(&mut b)?;
                    assert!(b[0] == b'\n');
                    let len = String::from_utf8(len)
                        .expect("Invalid length")
                        .parse::<usize>()
                        .expect("Invalid length");
                    let mut value = vec![0; len];
                    self.stream.read_exact(&mut value)?;
                    let mut ignore = [0, 0];
                    self.stream.read_exact(&mut ignore)?;
                    assert!(&ignore == b"\r\n");
                    Ok(Some(value))
                }
                _ => Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid response",
                )),
            }
        }
    }
}
