use std::{
    collections::HashMap,
    io::{Read, Write},
    net::TcpListener,
    thread::{JoinHandle, ThreadId},
};
mod codec;
mod command;
mod db;
mod error;
mod streamext;
mod value;

use command::Command;
use db::DB;
use error::{BadMessageError, Error};
use value::Value;

use crate::command::make_command_docs;

fn main() -> Result<()> {
    let mut server = Server::new(TcpListener::bind("0.0.0.0:6543")?);
    println!("Listening on port 6543");
    server.start()?;
    Ok(())
}

pub struct Server {
    listener: TcpListener,
    db: DB,
}
enum HandleCommand {
    Start(JoinHandle<()>),
    Stop(ThreadId),
    Drain,
}
type Result<T> = codec::Result<T>;
impl Server {
    pub fn new(listener: TcpListener) -> Server {
        Server {
            listener,
            db: DB::new(),
        }
    }

    pub fn start(&mut self) -> Result<()> {
        let (handle_sender, handle_receiver) = std::sync::mpsc::channel::<HandleCommand>();
        let handle_manager = std::thread::spawn(move || {
            let mut handles = HashMap::new();
            loop {
                match handle_receiver.recv().unwrap() {
                    HandleCommand::Start(handle) => {
                        let thread_id = handle.thread().id();
                        handles.insert(thread_id, handle);
                    }
                    HandleCommand::Stop(thread_id) => {
                        let handle = handles.remove(&thread_id).unwrap();
                        handle.join().unwrap();
                        handles.remove(&thread_id);
                    }
                    HandleCommand::Drain => break,
                }
            }
            for (_, handle) in handles {
                handle.join().unwrap();
            }
        });
        for stream in self.listener.incoming() {
            let (reader, writer) = streamext::split(stream?);
            let db = self.db.clone();
            let s = handle_sender.clone();
            let handle = std::thread::spawn(move || {
                dbg!("Accepted new connection");
                Connection::new(db, reader, writer).handle().unwrap();
                dbg!("Handled connection");
                s.send(HandleCommand::Stop(std::thread::current().id()))
                    .unwrap();
            });
            handle_sender.send(HandleCommand::Start(handle)).unwrap();
        }
        handle_sender.send(HandleCommand::Drain).unwrap();
        handle_manager.join().unwrap();
        Ok(())
    }
}

pub struct Connection<R: Read, W: Write> {
    db: DB,
    reader: R,
    writer: W,
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
