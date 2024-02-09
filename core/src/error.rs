use std::fmt::Debug;

use thiserror::Error;

use crate::connection::Nack;

#[derive(Error, Debug)]
pub enum ConnectionError {
    #[error("no v5 device found!")]
    DeviceNotFound,
    #[error("no bluetooth adapters found! Is bluetooth on?")]
    NoBluetoothAdapters,
    #[error("bluetooth error: `{0}`")]
    BluetoothError(#[from] btleplug::Error),
    #[error("serial port error: `{0}`")]
    SerialPortError(#[from] tokio_serial::Error),
    #[error("serial port error: `{0}`")]
    IoError(#[from] std::io::Error),
    #[error("invalid PIN")]
    InvalidPIN,
}

#[derive(Error, Debug)]
pub enum CommunicationError {
    #[error("nack received: `{0}`")]
    NegativeAcknowledgement(Nack),
    #[error("i/o error: `{0}`")]
    IoError(#[from] std::io::Error),
    #[error("i/o error: `{0}`")]
    BtIoError(#[from] btleplug::Error),
    #[error("timed out")]
    TimedOut,
    #[error("disconnected")]
    Eof,
}

#[derive(Error, Debug)]
pub enum CommandError {
    #[error("invalid subcommand")]
    InvalidSubcommand,
    #[error("missing argument `{0}`")]
    InvalidArgument(&'static str),
    #[error("connection error: {0}")]
    ConnectionError(#[from] ConnectionError),
    #[error("communications error: {0}")]
    CommunicationError(#[from] CommunicationError),
    #[error("i/o error: `{0}`")]
    IoError(#[from] std::io::Error),
    #[error("communications parsing error: {0}")]
    ParseError(#[from] ParseError),
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("missing key `{0}`")]
    MissingKey(&'static str),
    #[error("invalid name `{0}`")]
    InvalidName(String),
    #[error("invalid id {0}")]
    InvalidId(u32),
}
