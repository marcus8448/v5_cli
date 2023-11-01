use std::fmt::{Display, Formatter};

use crate::brain::Brain;
use crate::connection::bluetooth::DualSubscribedBluetoothConnection;

mod bluetooth;
mod daemon;
mod serial;

#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Nack {
    General = 0xFF,
    InvalidCrc = 0xCE,
    PayloadTooSmall = 0xD0,
    TransferSizeTooLarge = 0xD1,
    CrcError = 0xD2,
    ProgramFileError = 0xD3,
    UninitializedTransfer = 0xD4,
    InvalidInitialization = 0xD5,
    NonPaddedData = 0xD6,
    UnexpectedPacketAddress = 0xD7,
    LengthMismatch = 0xD8,
    NonExistentDirectory = 0xD9,
    FileIndexFull = 0xDA,
    FileExists = 0xDB,
}

impl TryFrom<u8> for Nack {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0xFF => Ok(Self::General),
            0xCE => Ok(Self::InvalidCrc),
            0xD0 => Ok(Self::PayloadTooSmall),
            0xD1 => Ok(Self::TransferSizeTooLarge),
            0xD2 => Ok(Self::CrcError),
            0xD3 => Ok(Self::ProgramFileError),
            0xD4 => Ok(Self::UninitializedTransfer),
            0xD5 => Ok(Self::InvalidInitialization),
            0xD6 => Ok(Self::NonPaddedData),
            0xD7 => Ok(Self::UnexpectedPacketAddress),
            0xD8 => Ok(Self::LengthMismatch),
            0xD9 => Ok(Self::NonExistentDirectory),
            0xDA => Ok(Self::FileIndexFull),
            0xDB => Ok(Self::FileExists),
            _ => Err(()),
        }
    }
}

impl Display for Nack {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub enum RobotConnectionOptions {
    Serial {
        port: Option<String>,
    },

    Bluetooth {
        mac_address: Option<String>,
        pin: Option<String>,
    },
    Daemon {
        user_port: u16,
        system_port: u16,
    },
}

pub async fn connect_to_brain(
    options: RobotConnectionOptions,
) -> Result<Brain, crate::error::ConnectionError> {
    match options {
        RobotConnectionOptions::Serial { port } => {
            let (system, _) = serial::find_ports(port)?;
            Ok(Brain::new(Box::new(serial::open_connection(system).await?)))
        }
        RobotConnectionOptions::Bluetooth { mac_address, pin } => {
            match bluetooth::connect_to_robot(mac_address, pin).await {
                Ok((peripheral, characteristics)) => Ok(Brain::new(Box::new(
                    DualSubscribedBluetoothConnection::create(
                        characteristics.tx_data,
                        characteristics.rx_data,
                        peripheral,
                    )
                    .await,
                ))),
                Err(err) => Err(err),
            }
        }
        RobotConnectionOptions::Daemon { system_port, .. } => Ok(Brain::new(Box::new(
            daemon::open_connection(system_port).await?,
        ))),
    }
}

pub async fn connect_to_user(
    options: RobotConnectionOptions,
) -> Result<Box<dyn RobotConnection + Send>, crate::error::ConnectionError> {
    match options {
        RobotConnectionOptions::Serial { port } => {
            let (_, user) = serial::find_ports(port)?;
            Ok(Box::new(serial::open_connection(user).await?))
        }
        RobotConnectionOptions::Bluetooth { mac_address, pin } => {
            match bluetooth::connect_to_robot(mac_address, pin).await {
                Ok((peripheral, characteristics)) => Ok(Box::new(
                    DualSubscribedBluetoothConnection::create(
                        characteristics.tx_user,
                        characteristics.rx_user,
                        peripheral,
                    )
                    .await,
                )),
                Err(err) => Err(err),
            }
        }
        RobotConnectionOptions::Daemon { user_port, .. } => {
            Ok(Box::new(daemon::open_connection(user_port).await?))
        }
    }
}

pub async fn connect_to_all(
    options: RobotConnectionOptions,
) -> Result<
    (
        Box<dyn RobotConnection + Send>,
        Box<dyn RobotConnection + Send>,
    ),
    crate::error::ConnectionError,
> {
    match options {
        RobotConnectionOptions::Serial { port } => {
            let (system, user) = serial::find_ports(port)?;
            Ok((
                Box::new(serial::open_connection(system).await?),
                Box::new(serial::open_connection(user).await?),
            ))
        }
        RobotConnectionOptions::Bluetooth { mac_address, pin } => {
            match bluetooth::connect_to_robot(mac_address, pin).await {
                Ok((peripheral, characteristics)) => Ok((
                    Box::new(
                        DualSubscribedBluetoothConnection::create(
                            characteristics.tx_data,
                            characteristics.rx_data,
                            peripheral.clone(),
                        )
                        .await,
                    ),
                    Box::new(
                        DualSubscribedBluetoothConnection::create(
                            characteristics.tx_user,
                            characteristics.rx_user,
                            peripheral,
                        )
                        .await,
                    ),
                )),
                Err(err) => Err(err),
            }
        }
        RobotConnectionOptions::Daemon {
            user_port,
            system_port,
        } => Ok((
            Box::new(daemon::open_connection(system_port).await?),
            Box::new(daemon::open_connection(user_port).await?),
        )),
    }
}

#[async_trait::async_trait]
pub trait RobotConnection {
    fn get_target_packet_alignment(&self) -> u16;

    async fn hint_begin_packet(&mut self) -> std::io::Result<()>;
    async fn hint_end_packet(&mut self) -> std::io::Result<()>;

    async fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()>;
    async fn flush(&mut self) -> std::io::Result<()>;

    async fn clear(&mut self) -> std::io::Result<()>;
    async fn try_read(&mut self, buf: &mut [u8]) -> std::io::Result<usize>;
    async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<()>;
    async fn try_read_one(&mut self) -> std::io::Result<u8>;
}
