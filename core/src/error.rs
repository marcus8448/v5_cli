use std::array::TryFromSliceError;
use std::fmt::Debug;
use std::str::Utf8Error;
use std::time::SystemTimeError;

#[derive(Debug)]
pub enum Error {
    Generic(&'static str),
    External(Box<dyn std::error::Error>),
    InvalidId(u8),
    InvalidName(String),
    Unknown,
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::External(Box::new(error))
    }
}

impl From<TryFromSliceError> for Error {
    fn from(error: TryFromSliceError) -> Self {
        Self::External(Box::new(error))
    }
}

impl From<Utf8Error> for Error {
    fn from(error: Utf8Error) -> Self {
        Self::External(Box::new(error))
    }
}

impl From<SystemTimeError> for Error {
    fn from(error: SystemTimeError) -> Self {
        Self::External(Box::new(error))
    }
}

impl From<&'static str> for Error {
    fn from(error: &'static str) -> Self {
        Self::Generic(error)
    }
}
