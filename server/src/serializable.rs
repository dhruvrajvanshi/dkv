use std::io::{Read, Write};

pub trait Serializable {
    fn write(&self, stream: &mut impl Write) -> std::io::Result<()>;
}
pub trait Deserializable: Sized {
    type Error;
    fn read(stream: &mut impl Read) -> std::result::Result<Self, Self::Error>;
}
