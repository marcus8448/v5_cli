use std::fmt::{Debug, Display, Formatter};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Generic(&'static str),
    External(Box<dyn std::error::Error + Send + Sync>),
    InvalidId(u8),
    InvalidName(String),
    Unknown,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Generic(str) => write!(f, "{}", str),
            Error::External(ext) => Display::fmt(ext, f),
            Error::InvalidId(id) => write!(f, "Invalid id: {}", id),
            Error::InvalidName(name) => write!(f, "Invalid name: {}", name),
            Error::Unknown => write!(f, "Unknown"),
        }
    }
}

impl std::error::Error for Error {}

impl From<btleplug::Error> for Error {
    fn from(value: btleplug::Error) -> Self {
        Error::External(Box::new(value))
    }
}

impl From<Error> for std::io::Error {
    fn from(value: Error) -> Self {
        std::io::Error::new(std::io::ErrorKind::InvalidData, value)
    }
}

