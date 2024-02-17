use std::{
    io::{Read, Write},
    sync::Arc,
};

pub fn split<S: Read + Write>(s: S) -> (impl Read, impl Write) {
    let r = Arc::new(s);
    (ReadHalf { inner: r.clone() }, WriteHalf { inner: r })
}

struct ReadHalf<R: Read> {
    inner: Arc<R>,
}
impl<R: Read> Read for ReadHalf<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        Arc::get_mut(&mut self.inner).unwrap().read(buf)
    }
}
struct WriteHalf<W> {
    inner: Arc<W>,
}
impl<W: Write> Write for WriteHalf<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Arc::get_mut(&mut self.inner).unwrap().write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Arc::get_mut(&mut self.inner).unwrap().flush()
    }
}
