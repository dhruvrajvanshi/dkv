use std::io::Read;

use crate::{
    codec::{read_bulk_string_array, Result},
    serializable::Deserializable,
    Error, Value,
};

#[derive(Debug, PartialEq)]
pub enum Command {
    Set(String, String),
    Get(String),
    Del(String),
    Exists(String),
    Command(Vec<String>),
    Config(Vec<String>),
    Ping(String),
    FlushAll,
    ClientSetInfo(String, String),
    Rename(String, String),
    HSet {
        key: String,
        field: String,
        value: String,
    },
    HGet {
        key: String,
        field: String,
    },
    HGetAll(String),
    HLen(String),
    HExists {
        key: String,
        field: String,
    },
    Hello(String),
    Subscribe(Vec<String>),
    Publish(String, String),
    Unsubscribe(Vec<String>),
    Quit,
}

impl Deserializable for Command {
    type Error = Error;
    fn read(stream: &mut impl Read) -> Result<Self> {
        let command = read_bulk_string_array(stream)?;
        use Command as c;
        let c: Command = match (command[0].to_uppercase().as_str(), &command[1..]) {
            ("CLIENT", [setinfo, key, value]) if setinfo.to_uppercase() == "SETINFO" => {
                Command::ClientSetInfo(key.clone(), value.clone())
            }
            ("CONFIG", args) => Command::Config(args.to_vec()),
            ("HELLO", [version]) => c::Hello(version.clone()),
            ("COMMAND", args) => c::Command(args.to_vec()),
            ("FLUSHALL", _) => c::FlushAll,
            ("PING", []) => c::Ping("PONG".into()),
            ("PING", [value]) => c::Ping(value.into()),
            ("SET", [key, value]) => c::Set(key.clone(), value.clone()),
            ("GET", [key]) => c::Get(key.clone()),
            ("DEL", [key]) => c::Del(key.clone()),
            ("RENAME", [old, new]) => c::Rename(old.clone(), new.clone()),
            ("EXISTS", [key]) => c::Exists(key.clone()),
            ("HGET", [key, field]) => c::HGet {
                key: key.clone(),
                field: field.clone(),
            },
            ("HSET", [key, field, value]) => c::HSet {
                key: key.clone(),
                field: field.clone(),
                value: value.clone(),
            },
            ("HGETALL", [key]) => c::HGetAll(key.clone()),
            ("HLEN", [key]) => c::HLen(key.clone()),
            ("HEXISTS", [key, field]) => c::HExists {
                key: key.clone(),
                field: field.clone(),
            },
            ("SUBSCRIBE", channels) => c::Subscribe(channels.to_vec()),
            ("PUBLISH", [channel, message]) => c::Publish(channel.clone(), message.clone()),
            ("UNSUBSCRIBE", channels) => c::Unsubscribe(channels.to_vec()),
            ("QUIT", []) => c::Quit,
            _ => return Err(Error::generic("Invalid command", format!("{:?}", command))),
        };
        Ok(c)
    }
}

pub fn make_command_docs() -> std::collections::HashMap<String, Value> {
    let mut map = std::collections::HashMap::new();
    let mut set_map = std::collections::HashMap::new();
    set_map.insert("summary".to_owned(), Value::from("Set a key to a value"));
    map.insert("SET".to_string(), Value::Map(set_map));
    let mut get_map = std::collections::HashMap::new();
    get_map.insert("summary".to_owned(), Value::from("Get a value by key"));
    map.insert("GET".to_string(), Value::Map(get_map));
    map
}
