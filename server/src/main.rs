use tokio::net::TcpListener;
use tracing::info;
mod bytestr;
mod connection;
mod protocol;
mod server;

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    tracing_subscriber::fmt::init();
    let listener = TcpListener::bind("0.0.0.0:6543").await?;
    info!("Listening on port 6543");
    server::run_server(listener).await?;
    Ok(())
}
