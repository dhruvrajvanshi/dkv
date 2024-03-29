use std::{
    collections::HashMap,
    net::TcpListener,
    thread::{JoinHandle, ThreadId},
};

use dkv_db::DB;

use crate::{codec, connection::Connection};

pub struct Server {
    listener: TcpListener,
    db: DB,
}
enum HandleCommand {
    Start(JoinHandle<()>),
    Stop(ThreadId),
    Drain,
}
pub type Result<T> = codec::Result<T>;
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
            let db = self.db.clone();
            let s = handle_sender.clone();
            let handle = std::thread::spawn(move || {
                dbg!("Accepted new connection");
                Connection::new(db, stream.unwrap()).handle().unwrap();
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
