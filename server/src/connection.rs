use tokio::net::TcpStream;
use tracing::{debug, error};

use crate::protocol;

pub async fn handle_connection(socket: TcpStream, _connection_id: usize) -> tokio::io::Result<()> {
    let (reader, mut writer) = tokio::io::split(socket);
    let mut reader = tokio::io::BufReader::new(reader);
    loop {
        debug!("handle_connection: Loop");
        match protocol::parse_array(&mut reader).await {
            Ok(arr) => {
                debug!("Received command: {:?}", arr);
                protocol::write_error_string(&mut writer, "Unimplemented").await?;
            }
            Err(e) => {
                error!("Error parsing message: {:?}", e);
                break;
            }
        }
    }
    Ok(())
}
