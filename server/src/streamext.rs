use std::{
    io::{Read, Write},
    sync::{Arc, Mutex},
};

pub fn split<S: Read + Write>(s: S) -> (impl Read, impl Write) {
    let r = Arc::new(Mutex::new(s));
    (ReadHalf { inner: r.clone() }, WriteHalf { inner: r })
}

struct ReadHalf<R: Read> {
    inner: Arc<Mutex<R>>,
}
impl<R: Read> Read for ReadHalf<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.lock().unwrap().read(buf)
    }
}
struct WriteHalf<W> {
    inner: Arc<Mutex<W>>,
}
impl<W: Write> Write for WriteHalf<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.lock().unwrap().write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.lock().unwrap().flush()
    }
}
