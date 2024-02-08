use std::io::Result;
use std::net::TcpListener;

fn main() -> Result<()> {
    let mut server = dkv::Server::new(TcpListener::bind("0.0.0.0:6543")?);
    println!("Listening on port 6543");
    server.start()?;
    Ok(())
}

pub mod dkv {
    use std::io::{self, Read};
    use std::net::{TcpListener, TcpStream};
    pub struct Server {
        listener: TcpListener,
    }
    type Result<T> = io::Result<T>;
    impl Server {
        pub fn new(listener: TcpListener) -> Server {
            Server { listener }
        }

        pub fn start(&mut self) -> Result<()> {
            for stream in self.listener.incoming() {
                println!("Accepted new connection");
                let command = self.read_command(&mut stream?)?;
                todo!()
            }
            Ok(())
        }

        fn read_command(&self, stream: &mut TcpStream) -> Result<Command> {
            let mut buf = [0, 0, 0];
            stream.read_exact(&mut buf)?;
            match &buf {
                b"PUT" => {
                    todo!("PUT")
                }
                b"GET" => {
                    todo!("GET")
                }
                b => {
                    dbg!(String::from_utf8_lossy(b));
                    todo!("Invalid command")
                }
            }
        }
    }

    pub enum Command {
        Put(Vec<u8>, Vec<u8>),
        Get(Vec<u8>),
    }
}
