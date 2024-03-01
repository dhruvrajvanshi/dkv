use std::net::TcpListener;
mod codec;
mod command;
mod connection;
mod error;
mod serializable;
mod server;
mod value;

use error::Error;
use value::Value;

use crate::server::Server;

fn main() -> server::Result<()> {
    let mut server = Server::new(TcpListener::bind("0.0.0.0:6543")?);
    println!("Listening on port 6543");
    server.start()?;
    Ok(())
}
