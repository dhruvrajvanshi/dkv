use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
};

use crate::bytestr::ByteStr;

#[derive(Clone)]
pub struct DB {
    data: Arc<RwLock<HashMap<ByteStr, Value>>>,
    next_subscription_id: Arc<Mutex<usize>>,
    subscriptions_by_channel: Arc<Mutex<HashMap<ByteStr, Vec<Subscription>>>>,
    subscriptions: Arc<Mutex<HashMap<Subscription, Arc<dyn Fn(Message) + Send + Sync + 'static>>>>,
}

pub type Result<T> = tokio::io::Result<T>;

impl DB {
    pub fn new() -> DB {
        DB {
            data: Arc::new(RwLock::new(HashMap::new())),
            next_subscription_id: Arc::new(Mutex::new(0)),
            subscriptions_by_channel: Arc::new(Mutex::new(HashMap::new())),
            subscriptions: Arc::new(Mutex::new(HashMap::new())),
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

    pub async fn del(&self, keys: &[ByteStr]) -> Result<i64> {
        let mut data = self.data.write().unwrap();
        let mut deleted_count = 0_i64;
        for key in keys {
            if data.remove(key).is_some() {
                deleted_count += 1;
            }
        }
        Ok(deleted_count)
    }

    pub fn subscribe<F>(&self, channel: impl Into<ByteStr>, f: F) -> Subscription
    where
        F: Fn(Message) + Send + Sync + 'static,
    {
        let subscription = self.next_subscription();
        let mut subs_by_channel = self.subscriptions_by_channel.lock().unwrap();
        let channel = channel.into();
        if let Some(subs) = subs_by_channel.get_mut(&channel) {
            subs.push(subscription);
        } else {
            subs_by_channel.insert(channel.into(), vec![subscription]);
        }
        let mut subscriptions = self.subscriptions.lock().unwrap();
        subscriptions.insert(subscription, Arc::new(f));
        subscription
    }

    pub async fn publish(
        &self,
        channel: impl Into<ByteStr>,
        message: impl Into<ByteStr>,
    ) -> Result<()> {
        let subs_by_channel = self.subscriptions_by_channel.lock().unwrap();
        let channel: ByteStr = channel.into();
        let message: ByteStr = message.into();
        let empty = vec![];
        let subs = subs_by_channel.get(&channel).unwrap_or(&empty);
        for sub in subs {
            let subs = self.subscriptions.lock().unwrap();
            if let Some(callback) = subs.get(sub) {
                let callback = callback.clone();
                callback(Message {
                    channel: channel.clone(),
                    value: message.clone(),
                })
            }
        }
        Ok(())
    }

    pub fn unsubscribe(&self, subscription: Subscription) {
        let mut subs = self.subscriptions.lock().unwrap();
        subs.remove(&subscription);
        // TODO: Remove from subscriptions_by_channel
    }

    fn next_subscription(&self) -> Subscription {
        let mut next_id = self.next_subscription_id.lock().unwrap();
        *next_id += 1;
        Subscription(*next_id)
    }
}
pub struct Message {
    channel: ByteStr,
    value: ByteStr,
}

#[derive(Eq, PartialEq, Debug, Hash, Copy, Clone)]
struct Subscription(usize);

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

#[cfg(test)]
mod test {
    use std::sync::Mutex;

    use super::*;

    #[tokio::test]
    async fn test_pubsub() -> Result<()> {
        let db = DB::new();

        let messages1 = Arc::new(Mutex::new(vec![]));
        let token1 = {
            let messages1 = messages1.clone();

            db.subscribe("channel1", move |message| {
                let mut messages = messages1.lock().unwrap();
                messages.push(message.value);
            })
        };
        let messages2 = Arc::new(Mutex::new(vec![]));
        let token2 = {
            let messages2 = messages2.clone();
            db.subscribe("channel1", move |message| {
                let mut messages = messages2.lock().unwrap();
                messages.push(message.value);
            })
        };

        db.publish("channel1", "message 1").await?;
        db.publish("channel1", "message 2").await?;

        db.unsubscribe(token1);
        db.unsubscribe(token2);

        db.publish("channel1", "message 3").await?;

        let messages1 = messages1.lock().unwrap();
        assert_eq!(
            *messages1,
            vec![ByteStr::from("message 1"), ByteStr::from("message 2")]
        );
        let messages2 = messages2.lock().unwrap();
        assert_eq!(
            *messages2,
            vec![ByteStr::from("message 1"), ByteStr::from("message 2")]
        );
        Ok(())
    }
}
