use dkv_protocol::{self, Value};
use std::net::TcpStream;

pub type Result<T> = dkv_protocol::Result<T>;

fn main() -> Result<()> {
    let mut djs = Client::new("localhost:6543");
    djs.set(Value::from("key"), Value::from("value"))?;
    let value = djs.get(Value::from("key"))?;
    assert_eq!(value, Value::from("value"));
    Ok(())
}
pub struct Client {
    address: String,
}
impl Client {
    pub fn new<T: Into<String>>(address: T) -> Client {
        Client {
            address: address.into(),
        }
    }

    pub fn set(&mut self, key: Value, value: Value) -> Result<()> {
        self.with_connection(|stream| {
            Value::from("SET").write(stream)?;
            key.write(stream)?;
            value.write(stream)?;
            assert_eq!(Value::read(stream)?, Value::from("OK"));
            Ok(())
        })
    }
    pub fn get(&mut self, key: Value) -> Result<Value> {
        self.with_connection(|stream| -> Result<Value> {
            Value::from("GET").write(stream)?;
            key.write(stream)?;
            let result = Value::read(stream)?;
            Ok(result)
        })
    }

    fn with_connection<F, T>(&mut self, f: F) -> Result<T>
    where
        F: FnOnce(&mut TcpStream) -> Result<T>,
    {
        let mut stream = TcpStream::connect(&self.address)?;
        let result = f(&mut stream);
        result
    }
}
