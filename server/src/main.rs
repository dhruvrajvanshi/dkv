use dkv_protocol::Result;
use std::net::TcpListener;

fn main() -> Result<()> {
    let mut server = dkv::Server::new(TcpListener::bind("0.0.0.0:6543")?);
    println!("Listening on port 6543");
    server.start()?;
    Ok(())
}

pub mod dkv {
    use std::net::{TcpListener, TcpStream};

    use dkv_protocol::Command;
    pub struct Server {
        listener: TcpListener,
    }
    type Result<T> = dkv_protocol::Result<T>;
    impl Server {
        pub fn new(listener: TcpListener) -> Server {
            Server { listener }
        }

        pub fn start(&mut self) -> Result<()> {
            for stream in self.listener.incoming() {
                println!("Accepted new connection");
                let command = self.read_command(&mut stream?)?;
                dbg!(command);
                todo!()
            }
            Ok(())
        }

        fn read_command(&self, stream: &mut TcpStream) -> Result<Command> {
            Command::read(stream)
        }
    }
}
