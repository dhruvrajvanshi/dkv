use tokio::net::TcpListener;
use tracing::info;

use crate::{connection::handle_connection, db::DB};

pub async fn run_server(listener: TcpListener) -> tokio::io::Result<()> {
    info!("Starting server loop");
    let mut connection_id: usize = 0;
    let db = DB::new();
    loop {
        let (socket, _) = listener.accept().await?;
        connection_id += 1;
        let connection_id = connection_id;
        info!("Accepted connection from {:?}", socket.peer_addr()?);
        let db = db.clone();
        tokio::spawn(async move {
            match handle_connection(socket, connection_id, db).await {
                Ok(()) => info!("Connection({connection_id}) closed"),
                Err(e) => {
                    info!("Connection({connection_id}) closed with error: {:?}", e)
                }
            };
        });
    }
}
