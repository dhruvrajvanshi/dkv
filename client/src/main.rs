use dkv_protocol::{self, Value};
use std::{
    io::{Read, Write},
    net::TcpStream,
};

type Result<T> = dkv_protocol::Result<T>;

fn main() -> Result<()> {
    let stream = TcpStream::connect("localhost:6543")?;
    let mut djs = Client::new(stream);
    djs.set(Value::from("key"), Value::from("value"))?;
    let stream = TcpStream::connect("localhost:6543")?;
    let mut djs = Client::new(stream);
    let value = djs.get(Value::from("key"))?;
    assert_eq!(value, Value::from("value"));
    Ok(())
}
pub struct Client<T: Write + Read> {
    stream: T,
}
impl<T: Write + Read> Client<T> {
    pub fn new(stream: T) -> Client<T> {
        Client { stream }
    }

    pub fn set(&mut self, key: Value, value: Value) -> Result<()> {
        self.write(Value::from("SET"))?;
        self.write(key)?;
        self.write(value)?;
        assert_eq!(Value::read(&mut self.stream)?, Value::from("OK"));
        return Ok(());
    }
    pub fn get(&mut self, key: Value) -> Result<Value> {
        self.write(Value::from("GET"))?;
        self.write(key)?;
        let result = Value::read(&mut self.stream)?;
        Ok(result)
    }

    fn write(&mut self, value: Value) -> Result<()> {
        value.write(&mut self.stream)
    }
}
