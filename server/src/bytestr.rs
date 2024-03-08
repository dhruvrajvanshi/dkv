use core::fmt;
use std::{
    fmt::{Debug, Formatter},
    ops::Deref,
};

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct ByteStr(Vec<u8>);
impl From<&str> for ByteStr {
    fn from(s: &str) -> Self {
        ByteStr(s.as_bytes().to_vec())
    }
}

impl From<&[u8]> for ByteStr {
    fn from(s: &[u8]) -> Self {
        ByteStr(s.to_vec())
    }
}
impl From<Vec<u8>> for ByteStr {
    fn from(v: Vec<u8>) -> Self {
        ByteStr(v)
    }
}

impl Deref for ByteStr {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Debug for ByteStr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", String::from_utf8_lossy(self.deref()))
    }
}
