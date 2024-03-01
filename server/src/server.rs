use std::{
    net::TcpListener,
    sync::{Arc, Mutex},
};

use dkv_db::DB;

use crate::{
    codec,
    connection::{Connection, HandleIfReadyResult},
};

pub struct Server {
    listener: TcpListener,
    db: DB,
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
        let connections: Arc<Mutex<Vec<Connection>>> = Arc::new(Mutex::new(vec![]));
        let h_connections = connections.clone();
        let handle = std::thread::spawn(move || loop {
            let connections = h_connections.clone();
            for connection in connections.lock().unwrap().iter_mut() {
                match connection.handle_if_ready() {
                    Ok(HandleIfReadyResult::Yield) => {}
                    Ok(HandleIfReadyResult::Ok) => {}
                    Ok(HandleIfReadyResult::Disconnect) => {
                        // TODO: remove from connections
                    }
                    Err(e) => {
                        eprintln!("Error handling connection: {:?}", e);
                    }
                }
            }
        });
        for stream in self.listener.incoming() {
            let db = self.db.clone();
            let stream = stream?;
            connections
                .clone()
                .lock()
                .unwrap()
                .push(Connection::new(db, stream));
        }
        handle.join().unwrap();
        Ok(())
    }
}
