use dkv_protocol::{self, Value};
use std::net::TcpStream;

type Result<T> = dkv_protocol::Result<T>;

fn main() -> Result<()> {
    let stream = TcpStream::connect("localhost:6543")?;
    let mut djs = Client::new(stream);
    djs.put(Value::from("key"), Value::from("value"))?;
    let value = djs.get(Value::from("key"))?;
    assert_eq!(value, Value::from("value"));
    Ok(())
}
pub struct Client {
    stream: TcpStream,
}
impl Client {
    pub fn new(stream: TcpStream) -> Client {
        Client { stream }
    }

    pub fn put(&mut self, key: Value, value: Value) -> Result<()> {
        self.write(Value::from("PUT"))?;
        self.write(key)?;
        self.write(value)?;
        return Ok(());
    }
    pub fn get(&mut self, key: Value) -> Result<Value> {
        self.write(Value::from("GET"))?;
        self.write(key)?;
        return self.read_get_result();
    }

    fn write(&mut self, value: Value) -> Result<()> {
        value.write(&mut self.stream)
    }

    fn read_get_result(&mut self) -> Result<Value> {
        Value::read(&mut self.stream)
    }
}
