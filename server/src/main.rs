use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
};
use tracing::{debug, error, info};
mod protocol;

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    tracing_subscriber::fmt::init();
    let listener = TcpListener::bind("0.0.0.0:6543").await?;
    info!("Listening on port 6543");
    run_server(listener).await?;
    Ok(())
}

async fn run_server(listener: TcpListener) -> tokio::io::Result<()> {
    info!("Starting server loop");
    let mut connection_id: usize = 0;
    loop {
        let (socket, _) = listener.accept().await?;
        connection_id += 1;
        let connection_id = connection_id;
        info!("Accepted connection from {:?}", socket.peer_addr()?);
        tokio::spawn(async move {
            match handle_connection(socket, connection_id).await {
                Ok(()) => info!("Connection({connection_id}) closed"),
                Err(e) => {
                    info!("Connection({connection_id}) closed with error: {:?}", e)
                }
            };
        });
    }
}

async fn handle_connection(socket: TcpStream, _connection_id: usize) -> tokio::io::Result<()> {
    let (reader, mut writer) = tokio::io::split(socket);
    let mut reader = tokio::io::BufReader::new(reader);
    loop {
        debug!("handle_connection: Loop");
        match protocol::parse_array(&mut reader).await {
            Ok(arr) => {
                debug!("Received command: {:?}", arr);
                writer.write_all(b"+OK\r\n").await?;
            }
            Err(e) => {
                error!("Error parsing message: {:?}", e);
                break;
            }
        }
    }
    Ok(())
}
