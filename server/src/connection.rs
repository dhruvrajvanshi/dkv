use std::{
    collections::HashMap,
    io::{Read, Write},
};

use crate::{
    command::Command,
    db::DB,
    error::{BadMessageError, Error},
    serializable::{Deserializable, Serializable},
    value::Value,
    Result,
};

use crate::command::make_command_docs;

pub struct Connection<R: Read, W: Write> {
    db: DB,
    reader: R,
    writer: W,
}
impl<R: Read, W: Write> Connection<R, W> {
    pub fn new(db: DB, reader: R, writer: W) -> Connection<R, W> {
        Connection { db, reader, writer }
    }
    pub fn handle(&mut self) -> std::io::Result<()> {
        loop {
            match self._handle() {
                Ok(()) => {}
                Err(Error::Io(ref e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    // When redis-cli quits, it just closes the connection,
                    // which means when we try to read the next command, we get
                    // an UnexpectedEof error. We should just break the loop to
                    // handle this case.
                    break;
                }
                Err(e) => {
                    eprintln!("Error: {:?}", e);
                    write!(self.writer, "-ERROR: {}\r\n", to_simple_string(e))?;
                }
            }
        }
        Ok(())
    }
    fn _handle(&mut self) -> Result<()> {
        let command = Command::read(&mut self.reader)?;
        match command {
            Command::Set(key, value) => {
                self.db.set(key, value);
                Self::write_simple_string(&mut self.writer, "OK")?;
            }
            Command::Get(key) => {
                let value = self.db.get(&key);
                value.write(&mut self.writer)?;
            }
            Command::Command(args) => {
                if args[0].clone().as_str() == Some("DOCS") {
                    let subcommand = args.get(1);
                    if subcommand.is_none() {
                        let command_docs = make_command_docs();
                        Value::Map(command_docs).write(&mut self.writer)?;
                    } else {
                        todo!("COMMAND DOCS is not implement for subcommands yet")
                    }
                } else {
                    todo!("Unimplement COMMAND {:?}", args[0])
                }
            }
            Command::Config(args) => {
                if let Some("GET") = args[0].as_str() {
                    if let Some(key) = args[1].as_str() {
                        let config = get_default_config();
                        let default_reply = Value::Map(HashMap::new());
                        if !config.contains_key(key) {
                            println!("invalid config key: {:?}", key);
                        }
                        let value = config.get(key).unwrap_or(&default_reply);
                        value.write(&mut self.writer)?;
                    } else {
                        todo!("Unimplement CONFIG GET {:?}", args[1])
                    }
                    Value::Map(HashMap::new()).write(&mut self.writer)?
                } else {
                    todo!("Unimplement CONFIG {:?}", args[0])
                }
            }
            Command::Ping(s) => Value::from(s).write(&mut self.writer)?,
            Command::FlushAll => {
                self.db.flush_all();
                Self::write_simple_string(&mut self.writer, "OK")?;
            }
        }
        Ok(())
    }

    fn write_simple_string(stream: &mut W, s: &str) -> Result<()> {
        write!(stream, "+{}\r\n", s)?;
        Ok(())
    }
}

fn get_default_config() -> HashMap<&'static str, Value> {
    let mut config = HashMap::new();
    config.insert("save", Value::from("3600 1 300 100 60 10000"));
    config.insert("appendonly", Value::from("no"));
    config.insert("bind", Value::from("localhost"));
    config
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

#[cfg(test)]
mod test {
    use std::{
        net::{TcpListener, TcpStream},
        thread::spawn,
        vec,
    };

    use crate::{
        codec::{self, Result},
        Server,
    };

    use super::*;
    #[test]
    fn handles_ping() {
        let input = b"*1\r\n$4\r\nPING\r\n";
        let mut output = vec![];
        let mut conn = Connection::new(DB::new(), &input[..], &mut output);
        conn.handle().unwrap();

        let value = codec::read(&mut &output[..]).unwrap();

        assert_eq!(value, Value::from("PONG"));
    }

    #[test]
    fn handles_ping_with_argument() {
        let input = b"*2\r\n$4\r\nPING\r\n$4\r\nPING\r\n";
        let mut output = vec![];
        let mut conn = Connection::new(DB::new(), &input[..], &mut output);
        conn.handle().unwrap();

        let value = codec::read(&mut &output[..]).unwrap();

        assert_eq!(value, Value::from("PING"));
    }

    #[test]
    fn can_get_after_set() -> Result<()> {
        let port = find_open_port();
        let server_port = port;
        spawn(move || {
            let mut server =
                Server::new(TcpListener::bind(format!("localhost:{}", server_port)).unwrap());
            server.start().unwrap();
        });

        let client_handle = spawn(move || {
            let mut stream = TcpStream::connect(format!("localhost:{}", port)).unwrap();

            Command::Set("foo".into(), Value::from("bar"))
                .write(&mut stream)
                .unwrap();

            assert_eq!(Value::read(&mut stream).unwrap(), Value::from("OK"));

            Command::Get("foo".into()).write(&mut stream).unwrap();
            assert_eq!(Value::read(&mut stream).unwrap(), Value::from("bar"));
        });
        client_handle.join().unwrap();
        // we don't have a way to signal termination to the server yet, so we'll just let
        Ok(())
    }

    fn find_open_port() -> usize {
        let start = 7000;
        let end = 8000;
        for port in start..end {
            if TcpListener::bind(format!("localhost:{}", port)).is_ok() {
                return port;
            }
        }
        panic!("No open ports found in range {}-{}", start, end);
    }
}
