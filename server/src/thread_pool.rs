use std::{
    sync::{mpsc, Arc, Mutex},
    thread::JoinHandle,
};

pub struct ThreadPool {
    sender: mpsc::Sender<Option<Task>>,
    threads: Vec<JoinHandle<()>>,
}
type Task = Box<dyn FnOnce() + Send + 'static>;
impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        let (sender, receiver) = mpsc::channel::<Option<Task>>();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut threads = Vec::with_capacity(size);
        for _ in 0..size {
            let receiver = receiver.clone();
            let thread = std::thread::spawn(move || loop {
                match receiver.lock().unwrap().recv() {
                    Ok(Some(task)) => {
                        task();
                    }
                    Ok(None) => break,
                    _ => {
                        std::thread::yield_now();
                    }
                }
            });
            threads.push(thread);
        }
        ThreadPool { sender, threads }
    }

    pub fn submit(&mut self, f: impl FnOnce() + Send + 'static) {
        self.sender.send(Some(Box::new(f))).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for _ in &self.threads {
            self.sender.send(None).unwrap();
        }
        for thread in self.threads.drain(..) {
            thread.join().unwrap();
        }
    }
}
