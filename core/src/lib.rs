pub extern crate clap;
pub extern crate crc;
pub extern crate log;
pub extern crate serialport;
pub extern crate time;

use std::sync::OnceLock;

use tokio::runtime::Handle;

pub mod buffer;
pub mod connection;
pub mod error;
pub mod packet;
pub mod plugin;

pub static TOKIO_RUNTIME: OnceLock<Handle> = OnceLock::new();
