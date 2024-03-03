use std::ops::Deref;

use thiserror::Error;
use tokio::net::TcpStream;
use tracing::{debug, error};

use crate::{
    bytestr::ByteStr,
    db::{self, HSetResult, DB},
    protocol,
};

pub async fn handle_connection(
    socket: TcpStream,
    connection_id: usize,
    db: DB,
) -> tokio::io::Result<()> {
    let mut conn = Connection::new(socket, connection_id, db);
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
    debug!("handle_connection: Done");
    Ok(())
}

struct Connection {
    id: usize,
    reader: tokio::io::BufReader<tokio::io::ReadHalf<TcpStream>>,
    writer: tokio::io::WriteHalf<TcpStream>,
    protocol_version: ProtocolVersion,
    db: DB,
}
#[derive(Debug)]
enum ProtocolVersion {
    RESP2,
    RESP3,
}
enum TickResult {
    Continue,
    Close,
}
impl Connection {
    fn new(socket: TcpStream, id: usize, db: DB) -> Self {
        let (reader, writer) = tokio::io::split(socket);
        let reader = tokio::io::BufReader::new(reader);
        Connection {
            id,
            reader,
            writer,
            protocol_version: ProtocolVersion::RESP2,
            db,
        }
    }
    async fn tick(&mut self) -> tokio::io::Result<TickResult> {
        Ok(match protocol::parse_array(&mut self.reader).await {
            Ok(arr) => {
                debug!("Received command: {:?}", arr);
                match Command::from(arr) {
                    Ok(command) => self.handle_command(command).await?,
                    Err(e) => {
                        error!("Error parsing command: {:?}", e);
                        protocol::write_error_string(&mut self.writer, "Invalid command").await?;
                    }
                }
                TickResult::Continue
            }
            Err(protocol::ParseError::Io(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                TickResult::Close
            }
            Err(e) => {
                error!("Error parsing message: {:?}", e);
                TickResult::Close
            }
        })
    }

    async fn handle_command(&mut self, command: Command) -> tokio::io::Result<()> {
        match command {
            Command::Hello(name) => {
                if name.deref().eq(b"2") {
                    self.protocol_version = ProtocolVersion::RESP2;
                    self.write_simple_string("OK").await?;
                } else if name.deref().eq(b"3") {
                    self.protocol_version = ProtocolVersion::RESP3;
                    protocol::write_map(
                        &mut self.writer,
                        &[
                            ["id".into(), (self.id as i64).into()],
                            ["proto".into(), 3.into()],
                            ["mode".into(), "standalone".into()],
                            ["role".into(), "master".into()],
                        ],
                    )
                    .await?;
                } else {
                    self.write_error_string("Invalid protocol version").await?;
                }
            }
            Command::ClientSetInfo(_, _) => {
                protocol::write_simple_string(&mut self.writer, "OK").await?;
            }
            Command::Get(key) => {
                match self.db.get(&key).await? {
                    Some(db::Value::String(value)) => self.write_bulk_string(&value).await?,
                    None => self.write_null_response().await?,
                    Some(_) => {
                        self.write_error_string("WRONGTYPE").await?;
                    }
                }
                protocol::write_error_string(&mut self.writer, "Unimplemented").await?;
            }
            Command::Set(ref key, value) => {
                self.db.set(key, value).await?;
                self.write_simple_string("OK").await?
            }
            Command::Del(_) => {
                protocol::write_error_string(&mut self.writer, "Unimplemented").await?;
            }
            Command::Exists(_) => {
                protocol::write_error_string(&mut self.writer, "Unimplemented").await?;
            }
            Command::HSet { key, field, value } => match self.db.hset(key, field, value).await? {
                HSetResult::Ok(_) => self.write_integer(1).await?,
                HSetResult::NotAMap => self.write_error_string("WRONGTYPE").await?,
            },
            Command::FlushAll => {
                self.db.flush_all().await?;
                self.write_simple_string("OK").await?;
            }
        }
        Ok(())
    }

    async fn write_error_string(&mut self, message: &str) -> tokio::io::Result<()> {
        protocol::write_error_string(&mut self.writer, message).await
    }

    async fn write_simple_string(&mut self, message: &str) -> tokio::io::Result<()> {
        protocol::write_simple_string(&mut self.writer, message).await
    }

    async fn write_bulk_string(&mut self, value: &ByteStr) -> tokio::io::Result<()> {
        protocol::write_bulk_string(&mut self.writer, value).await
    }
    async fn write_null_response(&mut self) -> tokio::io::Result<()> {
        debug!("Writing null response: {:?}", self.protocol_version);
        match self.protocol_version {
            ProtocolVersion::RESP2 => protocol::write_nil_reply(&mut self.writer).await,
            ProtocolVersion::RESP3 => protocol::write_null(&mut self.writer).await,
        }
    }

    async fn write_integer(&mut self, value: i64) -> tokio::io::Result<()> {
        protocol::write_integer(&mut self.writer, value).await
    }
}

enum Command {
    Hello(ByteStr),
    ClientSetInfo(ByteStr, ByteStr),
    Get(ByteStr),
    Set(ByteStr, ByteStr),
    Del(ByteStr),
    Exists(ByteStr),
    HSet {
        key: ByteStr,
        field: ByteStr,
        value: ByteStr,
    },
    FlushAll,
}

#[derive(Error, Debug)]
enum DecodeError {
    #[error("Invalid message: {0}")]
    UnparsableCommand(String),
}

impl Command {
    fn from(parts: Vec<ByteStr>) -> std::result::Result<Self, DecodeError> {
        fn err(s: impl Into<String>) -> std::result::Result<Command, DecodeError> {
            std::result::Result::Err(DecodeError::UnparsableCommand(s.into()))
        }
        match parts[0].deref() {
            b"CLIENT" => {
                if parts.len() < 2 {
                    err("CLIENT requires at least 1 argument")
                } else {
                    match parts[1].deref() {
                        b"SETINFO" => {
                            if parts.len() != 4 {
                                err("CLIENT SETINFO requires 2 arguments")
                            } else {
                                Ok(Command::ClientSetInfo(parts[2].clone(), parts[3].clone()))
                            }
                        }
                        _ => err(format!("Unknown CLIENT subcommand: {:?}", parts[1])),
                    }
                }
            }
            b"HELLO" => {
                if parts.len() != 2 {
                    err("HELLO requires 1 argument")
                } else {
                    Ok(Command::Hello(parts[1].clone()))
                }
            }
            b"GET" => {
                if parts.len() != 2 {
                    err("GET requires 1 argument")
                } else {
                    Ok(Command::Get(parts[1].clone()))
                }
            }
            b"SET" => {
                if parts.len() != 3 {
                    err("SET requires 2 arguments")
                } else {
                    Ok(Command::Set(parts[1].clone(), parts[2].clone()))
                }
            }
            b"DEL" => {
                if parts.len() != 2 {
                    err("DEL requires 1 argument")
                } else {
                    Ok(Command::Del(parts[1].clone()))
                }
            }
            b"EXISTS" => {
                if parts.len() != 2 {
                    err("EXISTS requires 1 argument")
                } else {
                    Ok(Command::Exists(parts[1].clone()))
                }
            }
            b"HSET" => {
                if parts.len() != 4 {
                    err("HSET requires 3 arguments")
                } else {
                    Ok(Command::HSet {
                        key: parts[1].clone(),
                        field: parts[2].clone(),
                        value: parts[3].clone(),
                    })
                }
            }
            b"FLUSHALL" => {
                if parts.len() != 1 {
                    err("FLUSHALL requires 0 arguments")
                } else {
                    Ok(Command::FlushAll)
                }
            }
            command => err(format!(
                "Unknown command: {:?}",
                String::from_utf8_lossy(command)
            )),
        }
    }
}
