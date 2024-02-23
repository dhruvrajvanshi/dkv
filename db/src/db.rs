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
                subscribers: HashMap::new(),
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

    fn with_impl<T>(&self, f: impl FnOnce(&mut DBImpl) -> T) -> T {
        let mut db_impl = self.db_impl.lock().unwrap();
        f(&mut db_impl)
    }

    pub fn publish(&self, channel: &str, value: &str) {
        let subscribers =
            self.with_impl(|db| db.subscribers.get(channel).unwrap_or(&vec![]).clone());
        // Subscriber functions may run for a long time, so we don't want to hold the lock
        // while they run
        for subscriber in subscribers {
            subscriber(channel, value)
        }
    }

    pub fn subscribe(&self, channel: &str, f: impl Fn(&str, &str) + Send + Sync + 'static) {
        self.with_impl(move |db| {
            if !db.subscribers.contains_key(channel) {
                db.subscribers.insert(channel.to_string(), vec![]);
            }
            db.subscribers.get_mut(channel).unwrap().push(Arc::new(f));
        })
    }
}

struct DBImpl {
    map: HashMap<String, Value>,
    subscribers: HashMap<String, Vec<Arc<dyn Fn(&str, &str) + Send + Sync + 'static>>>,
}

#[cfg(test)]
mod test {
    use std::thread::spawn;

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

    #[test]
    fn should_allow_pub_sub() {
        let db = DB::new();
        let db_publisher = db.clone();
        let count = Arc::new(Mutex::new(0));
        {
            let count = count.clone();
            db.subscribe("channel", move |_, message| {
                *count.clone().lock().unwrap() += 1;
                let count = *count.clone().lock().unwrap();
                assert_eq!(message, format!("message{}", count));
            });
        }
        let publisher = spawn(move || {
            let db = db_publisher;
            db.publish("channel", "message1");
            db.publish("channel", "message2");
            db.publish("channel", "message3");
            println!("Published messages");

            drop(db);
        });

        publisher.join().unwrap();
        assert_eq!(3, *count.lock().unwrap());
    }
}
