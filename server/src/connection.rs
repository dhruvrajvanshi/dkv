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
            Command::Rename(key, value) => match self.db.rename(&key, value).await? {
                db::RenameResult::Renamed => self.write_simple_string("OK").await?,
                db::RenameResult::KeyNotFound => self.write_error_string("no such key").await?,
            },
            Command::Del(_) => {
                protocol::write_error_string(&mut self.writer, "Unimplemented").await?;
            }
            Command::Exists(ref keys) => {
                let count = self.db.count(keys).await?;
                self.write_integer(count as i64).await?
            }
            Command::HGet { ref key, ref field } => {
                enum R {
                    Ok(Option<ByteStr>),
                    WrongType,
                }
                let r = self
                    .db
                    .view_key(key, |value| match value {
                        Some(db::Value::Hash(h)) => R::Ok(h.get(field).cloned()),
                        Some(_) => R::WrongType,
                        None => R::Ok(None),
                    })
                    .await;
                match r {
                    R::Ok(Some(value)) => self.write_bulk_string(&value).await?,
                    R::Ok(None) => self.write_null_response().await?,
                    R::WrongType => self.write_error_string("WRONGTYPE").await?,
                }
            }
            Command::HExists { ref key, ref field } => {
                let r = self
                    .db
                    .view_key(key, |value| match value {
                        Some(db::Value::Hash(h)) => {
                            if h.contains_key(field) {
                                1
                            } else {
                                0
                            }
                        }
                        Some(_) => 0,
                        None => 0,
                    })
                    .await;
                self.write_integer(r).await?;
            }
            Command::HGetAll(ref key) => {
                let entries = if let Some(db::Value::Hash(mut m)) = self.db.get(key).await? {
                    m.drain().collect()
                } else {
                    vec![]
                };
                match self.protocol_version {
                    ProtocolVersion::RESP2 => {
                        let mut output = vec![];
                        for (key, value) in entries {
                            output.push(key);
                            output.push(value);
                        }
                        protocol::write_bulk_string_array(&mut self.writer, &output).await?;
                    }
                    ProtocolVersion::RESP3 => {
                        let mut output = vec![];
                        for (key, value) in entries {
                            output.push(["field".into(), key.into()]);
                            output.push(["value".into(), value.into()]);
                        }
                        protocol::write_map(&mut self.writer, &output).await?;
                    }
                }
            }
            Command::HSet { key, field, value } => match self.db.hset(key, field, value).await? {
                HSetResult::Ok(_) => self.write_integer(1).await?,
                HSetResult::NotAMap => self.write_error_string("WRONGTYPE").await?,
            },
            Command::HLen(key) => {
                let r = self
                    .db
                    .view_key(&key, |value| match value {
                        Some(db::Value::Hash(h)) => h.len(),
                        Some(_) => 0,
                        None => 0,
                    })
                    .await;
                self.write_integer(r as i64).await?;
            }
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
    Rename(ByteStr, ByteStr),
    Del(ByteStr),
    Exists(Vec<ByteStr>),
    HSet {
        key: ByteStr,
        field: ByteStr,
        value: ByteStr,
    },
    HGet {
        key: ByteStr,
        field: ByteStr,
    },
    HExists {
        key: ByteStr,
        field: ByteStr,
    },
    HGetAll(ByteStr),
    HLen(ByteStr),
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
        match parts[0].to_ascii_uppercase().deref() {
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
                if parts.len() < 2 {
                    err("EXISTS requires at least 1 argument")
                } else {
                    Ok(Command::Exists(parts[1..].to_vec()))
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
            b"HGET" => {
                if parts.len() != 3 {
                    err("HGET requires 2 arguments")
                } else {
                    Ok(Command::HGet {
                        key: parts[1].clone(),
                        field: parts[2].clone(),
                    })
                }
            }
            b"HEXISTS" => {
                if parts.len() != 3 {
                    err("HEXISTS requires 2 arguments")
                } else {
                    Ok(Command::HExists {
                        key: parts[1].clone(),
                        field: parts[2].clone(),
                    })
                }
            }
            b"HGETALL" => {
                if parts.len() != 2 {
                    err("HGETALL requires 1 argument")
                } else {
                    Ok(Command::HGetAll(parts[1].clone()))
                }
            }
            b"HLEN" => {
                if parts.len() != 2 {
                    err("HLEN requires 1 argument")
                } else {
                    Ok(Command::HLen(parts[1].clone()))
                }
            }
            b"FLUSHALL" => {
                if parts.len() != 1 {
                    err("FLUSHALL requires 0 arguments")
                } else {
                    Ok(Command::FlushAll)
                }
            }
            b"RENAME" => {
                if parts.len() != 3 {
                    err("RENAME requires 2 arguments")
                } else {
                    Ok(Command::Rename(parts[1].clone(), parts[2].clone()))
                }
            }
            command => err(format!(
                "Unknown command: {:?}",
                String::from_utf8_lossy(command)
            )),
        }
    }
}
