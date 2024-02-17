use std::{
    collections::HashMap,
    io::{Read, Write},
};

use crate::{
    codec,
    serializable::{Deserializable, Serializable},
};

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    String(String),
    Array(Vec<Value>),
    Map(HashMap<String, Value>),
    Null,
}
impl Value {
    pub fn from<S: Into<String>>(value: S) -> Value {
        Value::String(value.into())
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }
}

impl Serializable for Value {
    fn write(&self, writer: &mut impl Write) -> std::io::Result<()> {
        codec::write(self, writer)
    }
}
impl Deserializable for Value {
    type Error = crate::Error;
    fn read(stream: &mut impl Read) -> std::result::Result<Self, Self::Error> {
        codec::read(stream)
    }
}
