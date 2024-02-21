use std::net::TcpListener;
mod codec;
mod command;
mod connection;
mod db;
mod error;
mod serializable;
mod streamext;
mod thread_pool;
mod value;

use db::DB;
use error::Error;
use thread_pool::ThreadPool;
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
    thread_pool: ThreadPool,
}

type Result<T> = codec::Result<T>;
impl Server {
    pub fn new(listener: TcpListener) -> Server {
        Server {
            listener,
            db: DB::new(),
            thread_pool: ThreadPool::new(1),
        }
    }

    pub fn start(&mut self) -> Result<()> {
        for stream in self.listener.incoming() {
            let stream = stream?;
            let db = self.db.clone();
            self.thread_pool.submit(move || {
                dbg!("Accepted new connection");
                let (reader, writer) = streamext::split(stream);
                Connection::new(db, reader, writer).handle().unwrap();
                dbg!("Handled connection");
            });
        }
        Ok(())
    }
}
