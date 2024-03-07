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
        Ok(self.view(|m| m.get(key).cloned()).await)
    }

    pub async fn hset(&self, key: ByteStr, field: ByteStr, value: ByteStr) -> Result<HSetResult> {
        self.mutate(|m| match m.get(&key) {
            Some(Value::Hash(h)) => {
                let mut h = h.clone();
                let result = h.insert(field, value);
                m.insert(key, Value::Hash(h));
                Ok(HSetResult::Ok(result.map(|it| Value::String(it))))
            }
            None => {
                let mut h = HashMap::new();
                h.insert(field, value);
                m.insert(key, Value::Hash(h));
                Ok(HSetResult::Ok(None))
            }
            Some(_) => Ok(HSetResult::NotAMap),
        })
        .await
    }

    pub async fn set(&self, key: &ByteStr, value: ByteStr) -> Result<Option<Value>> {
        Ok(self
            .mutate(|m| m.insert(key.clone(), Value::String(value)))
            .await)
    }

    pub async fn rename(&self, key: &ByteStr, new_key: ByteStr) -> Result<RenameResult> {
        Ok(self
            .mutate(|m| -> RenameResult {
                if let Some(value) = m.remove(key) {
                    m.insert(new_key, value);
                    RenameResult::Renamed
                } else {
                    RenameResult::KeyNotFound
                }
            })
            .await)
    }

    pub async fn count(&self, keys: &[ByteStr]) -> Result<usize> {
        Ok(self
            .view(|m| keys.iter().filter(|k| m.contains_key(k)).count())
            .await)
    }

    pub async fn flush_all(&self) -> Result<()> {
        let result = self.mutate(|m| m.clear());
        Ok(result.await)
    }

    async fn mutate<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut HashMap<ByteStr, Value>) -> R,
    {
        let mut data = self.data.write().unwrap();
        f(&mut data)
    }

    async fn view<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&HashMap<ByteStr, Value>) -> R,
    {
        let data = self.data.read().unwrap();
        f(&data)
    }

    pub async fn view_key<F, R>(&self, key: &ByteStr, f: F) -> R
    where
        F: FnOnce(Option<&Value>) -> R,
    {
        let data = self.data.read().unwrap();
        f(data.get(key))
    }
}

#[derive(Clone, Debug)]
pub enum Value {
    String(ByteStr),
    List(Vec<ByteStr>),
    Hash(HashMap<ByteStr, ByteStr>),
}

pub enum HSetResult {
    Ok(Option<Value>),
    NotAMap,
}
pub enum RenameResult {
    Renamed,
    KeyNotFound,
}
