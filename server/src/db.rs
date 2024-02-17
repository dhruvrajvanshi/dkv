use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::value::Value;

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
    pub fn get(&self, key: &str) -> Value {
        self.db_impl
            .clone()
            .lock()
            .unwrap()
            .map
            .get(key)
            .map_or(Value::Null, |v| v.clone())
    }
    pub fn flush_all(&self) {
        self.db_impl.clone().lock().unwrap().map.clear();
    }

    pub fn set(&mut self, key: String, value: Value) {
        self.db_impl.clone().lock().unwrap().map.insert(key, value);
    }
}

struct DBImpl {
    map: HashMap<String, Value>,
}
