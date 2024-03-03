use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::bytestr::ByteStr;

#[derive(Clone)]
pub struct DB {
    data: Arc<RwLock<HashMap<ByteStr, Value>>>,
}

pub type Result<T> = tokio::io::Result<T>;

impl DB {
    pub fn new() -> DB {
        DB {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get(&self, key: &ByteStr) -> Result<Option<Value>> {
        self.view(|m| m.get(key).cloned())
    }

    pub async fn set(&self, key: &ByteStr, value: ByteStr) -> Result<Option<Value>> {
        self.mutate(|m| m.insert(key.clone(), Value::String(value)))
    }

    pub async fn flush_all(&self) -> Result<()> {
        self.mutate(|m| m.clear())
    }

    fn mutate<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&mut HashMap<ByteStr, Value>) -> R,
    {
        let mut data = self.data.write().unwrap();
        Ok(f(&mut data))
    }

    fn view<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&HashMap<ByteStr, Value>) -> R,
    {
        let data = self.data.read().unwrap();
        Ok(f(&data))
    }
}

#[derive(Clone, Debug)]
pub enum Value {
    String(ByteStr),
    List(Vec<ByteStr>),
    Hash(HashMap<ByteStr, ByteStr>),
}
