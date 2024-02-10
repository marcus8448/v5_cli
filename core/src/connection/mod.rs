use std::fmt::{Display, Formatter};
use std::mem::size_of;

use crc::{Crc, CRC_16_XMODEM};

use crate::brain::Brain;
use crate::buffer::ReceivingBuffer;
use crate::connection::bluetooth::BluetoothConnection;
use crate::error::CommunicationError;

mod bluetooth;
pub mod daemon;
mod serial;

pub(crate) const CRC16: Crc<u16> = Crc::<u16>::new(&CRC_16_XMODEM);

pub(crate) const PACKET_HEADER: [u8; 4] = [0xc9, 0x36, 0xb8, 0x47];
pub(crate) const RESPONSE_HEADER: [u8; 2] = [0xAA, 0x55];

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
        port: u16,
    },
}

pub async fn connect_to_brain(
    options: RobotConnectionOptions,
) -> Result<Brain, crate::error::ConnectionError> {
    match options {
        RobotConnectionOptions::Serial { port } => {
            let (system, user) = serial::find_ports(port)?;
            Ok(Brain::new(Box::new(
                serial::open_connection(system, user).await?,
            )))
        }
        RobotConnectionOptions::Bluetooth { mac_address, pin } => {
            match bluetooth::connect_to_robot(mac_address, pin).await {
                Ok((peripheral, characteristics)) => Ok(Brain::new(Box::new(
                    BluetoothConnection::create(
                        characteristics.rx_data,
                        characteristics.tx_data,
                        characteristics.rx_user,
                        characteristics.tx_user,
                        peripheral,
                    )
                    .await,
                ))),
                Err(err) => Err(err),
            }
        }
        RobotConnectionOptions::Daemon { port } => {
            Ok(Brain::new(Box::new(daemon::open_connection(port).await?)))
        }
    }
}

pub struct Packet<'a> {
    packet_id: u8,
    buffer: Box<[u8]>,
    pos: usize,
    brain: &'a mut Brain,
}

impl<'a> Packet<'a> {
    pub fn new(packet_id: u8, content_len: usize, connection: &'a mut Brain) -> Self {
        assert!(content_len < 0b1000_0000_0000_0000_u16 as usize);
        let meta_len = /*header*/ PACKET_HEADER.len() + /*ext id*/ 1 + /*command id*/  1 + if /*len*/ content_len < 0x80 { 1 } else { 2 };
        let size = meta_len + content_len + /*CRC*/ size_of::<u16>();

        let mut buffer = Self {
            packet_id,
            buffer: vec![0_u8; size].into_boxed_slice(),
            pos: 0,
            brain: connection,
        };

        buffer.write_raw(&PACKET_HEADER);

        buffer.write_u8(0x56); //ext packet id
        buffer.write_u8(packet_id);

        if content_len >= 0b1000_0000 {
            buffer.write_u8((content_len >> 8 | 0b1000_0000) as u8);
            buffer.write_u8((content_len & 0xFF) as u8);
        } else {
            buffer.write_u8(content_len as u8);
        }

        buffer
    }

    pub async fn send(mut self) -> Result<ReceivingBuffer, CommunicationError> {
        assert_eq!(self.buffer.len() - size_of::<u16>(), self.pos);

        self.write_raw(&CRC16.checksum(&self.buffer[..self.pos]).to_be_bytes());
        self.brain.connection.send_packet(&self.buffer).await
    }
}

impl<'a> Packet<'a> {
    pub fn write_u8(&mut self, value: u8) {
        self.buffer[self.pos..self.pos + size_of::<u8>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<u8>();
    }

    pub fn write_i8(&mut self, value: i8) {
        self.buffer[self.pos..self.pos + size_of::<i8>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<i8>();
    }

    pub fn write_u16(&mut self, value: u16) {
        self.buffer[self.pos..self.pos + size_of::<u16>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<u16>();
    }

    pub fn write_i16(&mut self, value: i16) {
        self.buffer[self.pos..self.pos + size_of::<i16>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<i16>();
    }

    pub fn write_u32(&mut self, value: u32) {
        self.buffer[self.pos..self.pos + size_of::<u32>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<u32>();
    }

    pub fn write_i32(&mut self, value: i32) {
        self.buffer[self.pos..self.pos + size_of::<i32>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<i32>();
    }

    pub fn write_u64(&mut self, value: u64) {
        self.buffer[self.pos..self.pos + size_of::<u64>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<u64>();
    }

    pub fn write_i64(&mut self, value: i64) {
        self.buffer[self.pos..self.pos + size_of::<i64>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<i64>();
    }

    pub fn write_u128(&mut self, value: u128) {
        self.buffer[self.pos..self.pos + size_of::<u128>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<u128>();
    }

    pub fn write_i128(&mut self, value: i128) {
        self.buffer[self.pos..self.pos + size_of::<i128>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<i128>();
    }

    pub fn write_f32(&mut self, value: f32) {
        self.buffer[self.pos..self.pos + size_of::<f32>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<f32>();
    }

    pub fn write_f64(&mut self, value: f64) {
        self.buffer[self.pos..self.pos + size_of::<f64>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<f64>();
    }

    pub fn write_raw(&mut self, slice: &[u8]) {
        self.buffer[self.pos..self.pos + slice.len()].copy_from_slice(slice);
        self.pos += slice.len();
    }

    pub fn write_str(&mut self, string: &str, target_len: usize) {
        assert!(string.len() < target_len);
        self.buffer[self.pos..self.pos + string.len()].copy_from_slice(string.as_bytes());
        self.pos += target_len;
    }

    pub fn pad(&mut self, amount: usize) {
        self.pos += amount; // zero-initialized
        assert!(self.pos <= self.buffer.len())
    }
}

#[async_trait::async_trait]
pub trait RobotConnection: Send {
    fn get_max_packet_size(&self) -> u16;

    async fn send_simple(&mut self, id: u8) -> Result<ReceivingBuffer, CommunicationError> {
        let mut buffer = [0_u8; 4 /*header*/ + 1 /*id*/ + /*CRC*/ size_of::<u16>()];
        buffer[0..4].copy_from_slice(&PACKET_HEADER);
        buffer[4] = id;
        let crc = CRC16
            .checksum(&buffer[..buffer.len() - size_of::<u16>()])
            .to_le_bytes();
        buffer[5..].copy_from_slice(&crc);

        return self.send_packet(&buffer).await;
    }

    async fn claim_exclusive(&mut self) -> Result<(), CommunicationError> {
        Ok(())
    }

    async fn unclaim_exclusive(&mut self) -> Result<(), CommunicationError> {
        Ok(())
    }
    async fn send_packet(&mut self, data: &[u8]) -> Result<ReceivingBuffer, CommunicationError>;
    async fn write_serial(&mut self, data: &[u8]) -> Result<usize, CommunicationError>;
    async fn read_serial(&mut self, data: &mut [u8]) -> Result<usize, CommunicationError>;

    async fn reset(&mut self) -> Result<(), CommunicationError>;

    async fn shutdown(&mut self) -> Result<(), CommunicationError>;
}
