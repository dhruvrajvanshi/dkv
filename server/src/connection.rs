use tokio::net::TcpStream;
use tracing::{debug, error};

use crate::protocol;

pub async fn handle_connection(socket: TcpStream, connection_id: usize) -> tokio::io::Result<()> {
    let mut conn = Connection::new(socket, connection_id);
    loop {
        debug!("handle_connection: Loop");
        match conn.tick().await {
            Ok(TickResult::Continue) => {}
            Ok(TickResult::Close) => break,
            Err(_) => {
                break;
            }
        }
    }
    Ok(())
}

struct Connection {
    _id: usize,
    reader: tokio::io::BufReader<tokio::io::ReadHalf<TcpStream>>,
    writer: tokio::io::WriteHalf<TcpStream>,
}
enum TickResult {
    Continue,
    Close,
}
impl Connection {
    fn new(socket: TcpStream, id: usize) -> Self {
        let (reader, writer) = tokio::io::split(socket);
        let reader = tokio::io::BufReader::new(reader);
        Connection {
            _id: id,
            reader,
            writer,
        }
    }
    async fn tick(&mut self) -> tokio::io::Result<TickResult> {
        Ok(match protocol::parse_array(&mut self.reader).await {
            Ok(arr) => {
                debug!("Received command: {:?}", arr);
                protocol::write_error_string(&mut self.writer, "Unimplemented").await?;
                TickResult::Continue
            }
            Err(e) => {
                error!("Error parsing message: {:?}", e);
                TickResult::Close
            }
        })
    }
}
