use std::io;

#[derive(Debug)]
pub enum BadMessageError {
    InvalidLength(String),
    Utf8(std::string::FromUtf8Error),
    InvalidCommand(String),
    /**
     * First argument is the error message sent to the client.
     * Must be a simple string (i.e. no newlines)
     * Second argument is only used by the server for debugging
     */
    Generic(String, String),
}
#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    BadMessage(BadMessageError),
    UnexpectedStartOfValue(char),
}
impl Error {
    pub fn generic<S: Into<String>, S2: Into<String>>(s: S, internal: S2) -> Error {
        let string: String = s.into();
        assert!(
            !string.contains("\r") && !string.contains("\n"),
            "Generic error strings must not contain newlines"
        );
        Error::BadMessage(BadMessageError::Generic(string, internal.into()))
    }
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}
