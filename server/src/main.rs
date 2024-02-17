use std::{
    collections::HashMap,
    net::TcpListener,
    thread::{JoinHandle, ThreadId},
};
mod codec;
mod command;
mod connection;
mod db;
mod error;
mod serializable;
mod streamext;
mod value;

use db::DB;
use error::Error;
use value::Value;

use crate::connection::Connection;

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

#[cfg(test)]
mod test {
    use std::{net::TcpListener, thread::spawn};

    use crate::{Result, Server};
    use redis::{self, Commands};

    #[test]
    fn can_get_after_set() -> Result<()> {
        let port = find_open_port();
        let server_port = port;
        spawn(move || {
            let mut server = Server::new(
                TcpListener::bind(format!("redis://localhost:{}", server_port)).unwrap(),
            );
            server.start().unwrap();
        });

        let client_handle = spawn(move || {
            let cl = redis::Client::open(format!("redis://localhost:{}", port)).unwrap();
            let mut c = cl.get_connection().unwrap();
            let () = c.set("foo", "bar").unwrap();
            let value = c.get::<&str, String>("foo").unwrap();
            assert_eq!(value, "bar");
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
