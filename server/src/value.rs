use std::collections::HashMap;

use crate::codec;

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

    pub fn write<T: std::io::Write>(&self, stream: &mut T) -> std::io::Result<()> {
        codec::write(self, stream)
    }

    pub fn read<T: std::io::Read>(stream: &mut T) -> codec::Result<Value> {
        codec::read(stream)
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }
}
