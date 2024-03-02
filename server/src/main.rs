use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
};
use tracing::{debug, error, info};

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
        match parse::parse_array(&mut reader).await {
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

mod parse {
    use thiserror::Error;
    use tokio::io::{self, AsyncRead, AsyncReadExt};

    #[derive(Error, Debug)]
    pub enum ParseError {
        #[error("I/O error: {0}")]
        Io(#[from] io::Error),
        #[error("Parse error: {0}")]
        InvalidMessage(String),
    }
    pub type Result<T> = std::result::Result<T, ParseError>;

    pub async fn parse_array<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Vec<String>> {
        expect_char(reader, b'*').await?;
        let len = parse_len(reader).await?;
        let mut arr = Vec::with_capacity(len);
        for _ in 0..len {
            arr.push(parse_bulk_string(reader).await?);
        }
        Ok(arr)
    }

    pub async fn parse_bulk_string<R: AsyncRead + Unpin>(reader: &mut R) -> Result<String> {
        expect_char(reader, b'$').await?;
        let len = parse_len(reader).await?;
        let mut buf = vec![0; len];
        reader.read_exact(&mut buf).await?;
        let buf = String::from_utf8(buf)
            .map_err(|_| ParseError::InvalidMessage("Invalid UTF-8 in bulk string".to_string()))?;
        expect_char(reader, b'\r').await?;
        expect_char(reader, b'\n').await?;
        Ok(buf)
    }

    async fn expect_char<R: AsyncRead + Unpin>(reader: &mut R, expected: u8) -> Result<u8> {
        let c = reader.read_u8().await?;
        if c != expected {
            Err(ParseError::InvalidMessage(format!(
                "Expected '{}', found '{}'",
                expected as char, c as char
            )))
        } else {
            Ok(c)
        }
    }

    pub async fn parse_len<R: AsyncRead + Unpin>(reader: &mut R) -> Result<usize> {
        let mut buf = String::new();
        loop {
            let char = reader.read_u8().await?;
            if char == b'\r' {
                expect_char(reader, b'\n').await?;
                break;
            } else {
                buf.push(char as char);
            }
        }
        Ok(buf
            .parse()
            .map_err(|_| ParseError::InvalidMessage("Invalid length".to_string()))?)
    }
}
