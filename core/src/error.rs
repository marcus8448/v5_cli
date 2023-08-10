use std::fmt::Debug;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConnectionError {
    #[error("no v5 device found!")]
    DeviceNotFound,
    #[error("no bluetooth adapters found! Is bluetooth on?")]
    NoBluetoothAdapters,
    #[error("bluetooth error")]
    BluetoothError(#[from] btleplug::Error),
    #[error("invalid PIN")]
    InvalidPIN
}

#[derive(Error, Debug)]
pub enum CommandError {
    #[error("invalid subcommand")]
    InvalidSubcommand,
    #[error("missing argument `{0}`")]
    InvalidArgument(&'static str),
    #[error("robot connection error")]
    ConnectionError(#[from] ConnectionError),
    #[error("robot communications error")]
    CommunicationError(#[from] std::io::Error),
    #[error("robot communications parsing error")]
    ParseError(#[from] ParseError)
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("missing key `{0}`")]
    MissingKey(&'static str),
    #[error("invalid name `{0}`")]
    InvalidName(String),
    #[error("invalid id {0}")]
    InvalidId(u32)
}

impl From<ParseError> for std::io::Error {
    fn from(value: ParseError) -> Self {
        match value {
            ParseError::MissingKey(key) => {
                std::io::Error::new(std::io::ErrorKind::NotFound, key)
            }
            ParseError::InvalidName(name) => {
                std::io::Error::new(std::io::ErrorKind::InvalidData, name)
            }
            ParseError::InvalidId(id) => {
                std::io::Error::new(std::io::ErrorKind::InvalidData, id.to_string())
            }
        }
    }
}