use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

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

#[derive(Clone)]
pub struct DB {
    db_impl: Arc<Mutex<DBImpl>>,
}

impl DB {
    pub fn new() -> DB {
        DB {
            db_impl: Arc::new(Mutex::new(DBImpl {
                map: HashMap::new(),
            })),
        }
    }

    pub fn get_optional(&self, key: &str) -> Option<Value> {
        self.db_impl.clone().lock().unwrap().map.get(key).cloned()
    }
    pub fn exists(&self, key: &str) -> bool {
        self.db_impl.clone().lock().unwrap().map.contains_key(key)
    }
    pub fn flush_all(&self) {
        self.db_impl.clone().lock().unwrap().map.clear();
    }

    pub fn set(&self, key: String, value: Value) {
        self.db_impl.clone().lock().unwrap().map.insert(key, value);
    }

    pub fn del(&self, key: &str) -> u64 {
        self.db_impl
            .clone()
            .lock()
            .unwrap()
            .map
            .remove(key)
            .is_some() as u64
    }

    pub fn view<T>(&self, key: &str, f: impl FnOnce(Option<&Value>) -> T) -> T {
        let m = self.db_impl.clone();
        let m = m.lock().unwrap();
        let m = &m.map;
        let value = m.get(key);
        f(value)
    }

    pub fn mutate<T>(&self, key: &str, f: impl FnOnce(Option<&mut Value>) -> T) -> T {
        let m = self.db_impl.clone();
        let mut m = m.lock().unwrap();
        let m = &mut m.map;
        let value = m.get_mut(key);
        f(value)
    }
}

struct DBImpl {
    map: HashMap<String, Value>,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_allow_reading_without_copying() {
        let db = DB::new();
        db.set("key".to_string(), Value::String("value".to_string()));
        let len = db.view("key", |value: Option<&Value>| match value {
            Some(Value::String(value)) => value.len(),
            _ => panic!(),
        });
        assert_eq!(len, "value".len());
    }
}
