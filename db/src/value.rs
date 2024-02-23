use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    List(Vec<String>),
    Hash(HashMap<String, String>),
}
impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_owned())
    }
}
impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}
impl From<Vec<String>> for Value {
    fn from(s: Vec<String>) -> Self {
        Value::List(s)
    }
}
impl From<HashMap<String, String>> for Value {
    fn from(s: HashMap<String, String>) -> Self {
        Value::Hash(s)
    }
}
