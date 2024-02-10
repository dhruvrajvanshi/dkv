use std::{
    collections::HashMap,
    io::{Read, Write},
    net::TcpListener,
};
mod codec;
mod command;
mod error;
mod value;

use command::Command;
use error::{BadMessageError, Error};
use value::Value;

fn main() -> Result<()> {
    let mut server = Server::new(TcpListener::bind("0.0.0.0:6543")?);
    println!("Listening on port 6543");
    server.start()?;
    Ok(())
}

pub struct Server {
    listener: TcpListener,
    map: HashMap<String, Value>,
}
type Result<T> = codec::Result<T>;
impl Server {
    pub fn new(listener: TcpListener) -> Server {
        Server {
            listener,
            map: HashMap::new(),
        }
    }

    pub fn start(&mut self) -> Result<()> {
        for stream in self.listener.incoming() {
            let mut stream = stream?;
            dbg!("Accepted new connection");
            Connection::new(&mut self.map, &mut stream).handle()?;
            dbg!("Handled connection");
        }
        Ok(())
    }
}

pub struct Connection<'a, T: Write + Read> {
    map: &'a mut HashMap<String, Value>,
    stream: T,
}
fn to_simple_string(e: Error) -> String {
    match e {
        // there's no guarantee that io::Error contains characters that are safe
        // to send as part of a simple string, so we'll just send a generic error
        // Besides, this is treated as a server error, not client error.
        Error::Io(_) => String::from("Internal server error"),
        Error::BadMessage(BadMessageError::InvalidCommand(_)) => String::from("Invalid command"),
        Error::BadMessage(BadMessageError::InvalidLength(_)) => {
            String::from("Invalid length for a bulk string")
        }
        Error::BadMessage(BadMessageError::Generic(s, _)) => s,
        Error::BadMessage(BadMessageError::Utf8(_)) => String::from("Invalid UTF-8"),
        Error::UnexpectedStartOfValue(c) => {
            format!("Unexpected start of value: {}", c)
        }
    }
}
enum ConnectionResult {
    Exit,
    Continue,
}
impl<T: Write + Read> Connection<'_, T> {
    pub fn new<'a>(map: &'a mut HashMap<String, Value>, stream: T) -> Connection<'a, T> {
        Connection { map, stream }
    }
    pub fn handle(&mut self) -> std::io::Result<()> {
        loop {
            match self._handle() {
                Ok(ConnectionResult::Continue) => {}
                Ok(ConnectionResult::Exit) => break,
                Err(e) => {
                    eprintln!("Error: {:?}", e);
                    write!(self.stream, "-ERROR: {}\r\n", to_simple_string(e))?;
                    ()
                }
            }
        }
        Ok(())
    }
    fn _handle(&mut self) -> Result<ConnectionResult> {
        let command = Command::read(&mut self.stream)?;
        Ok(match command {
            Command::Set(key, value) => {
                self.map.insert(key, value);
                Self::write_simple_string(&mut self.stream, "OK")?;
                ConnectionResult::Continue
            }
            Command::Get(key) => {
                let value = self.map.get(&key).map_or(Value::Null, |v| v.clone());
                value.write(&mut self.stream)?;
                ConnectionResult::Continue
            }
        })
    }

    fn write_simple_string(stream: &mut T, s: &str) -> Result<()> {
        write!(stream, "+{}\r\n", s)?;
        Ok(())
    }
}
