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
    Integer(i64),
    Map(HashMap<String, Value>),
    Null,
}
impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}
impl From<&String> for Value {
    fn from(s: &String) -> Self {
        Value::String(s.clone())
    }
}
impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}
impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::Integer(i)
    }
}

#[macro_export]
macro_rules! dkv_array {
    ($($x:expr),*) => {
        Value::Array(vec![$($x.into()),*])
    };
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn dkv_array_macro() {
        assert_eq!(
            dkv_array!("a", "b"),
            Value::Array(vec![Value::from("a"), Value::from("b")])
        );
    }
}
