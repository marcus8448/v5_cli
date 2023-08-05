use std::fmt::Debug;
use std::io::Write;
use std::mem::size_of;

use crc::{Crc, CRC_16_XMODEM};

use crate::buffer::{FixedReadBuffer, ReadBuffer, WriteBuffer};
use crate::connection::SerialConnection;
use crate::error::Result;

pub mod competition;
pub mod filesystem;
pub mod system;

pub const CRC16: Crc<u16> = Crc::<u16>::new(&CRC_16_XMODEM);

const PACKET_HEADER: &[u8; 4] = &[0xc9, 0x36, 0xb8, 0x47];
const RESPONSE_HEADER: [u8; 2] = [0xAA, 0x55];
const EXT_PACKET_ID: u8 = 0x56;

pub trait Packet<const ID: u8>: Debug {
    type Response;

    fn send_len(&self) -> usize;

    fn is_simple() -> bool {
        false
    }

    fn write_buffer(&self, buffer: &mut dyn WriteBuffer) -> std::io::Result<()>;

    fn read_response(
        &self,
        buffer: &mut dyn ReadBuffer,
        len: usize,
    ) -> std::io::Result<Self::Response>;

    fn send(&mut self, connection: &mut Box<dyn SerialConnection>) -> Result<Self::Response> {
        dbg!(&self);
        let len = self.send_len();
        let mut buffer =
            Vec::with_capacity(4 + 1 + 1 + if len < 0x80 { 1 } else { 2 } + len + size_of::<u16>());
        buffer.write_raw(PACKET_HEADER);

        if Self::is_simple() {
            buffer.write_u8(ID);
        } else {
            buffer.write_u8(EXT_PACKET_ID);
            buffer.write_u8(ID);

            if len < 0x80 {
                println!("normal size {}", len);
                buffer.write_u8(len as u8);
            } else {
                println!("pack size {}", len);
                buffer.write_u8((len >> 8 | 0x80) as u8);
                buffer.write_u8((len & 0xff) as u8);
            }

            let i = buffer.len();

            self.write_buffer(&mut buffer)?;
            let j = buffer.len();
            println!("Act size: {}", j - i);

            buffer.write_raw(&CRC16.checksum(&buffer).to_be_bytes());
        }

        // println!("sending: {:02X?}", &buffer);
        connection.write_all(&buffer)?;
        connection.flush()?;

        let mut buf = [0_u8; 1];

        loop {
            println!("Searching for header");
            connection.read_exact(&mut buf)?;

            if buf[0] != RESPONSE_HEADER[0] {
                continue;
            }
            connection.read_exact(&mut buf)?;
            if buf[0] != RESPONSE_HEADER[1] {
                continue;
            }
            break;
        }

        let mut payload = Vec::with_capacity(64);
        payload.extend_from_slice(&RESPONSE_HEADER);

        let mut metadata = [0_u8; 2];

        connection.read_exact(&mut metadata)?;
        let command = metadata[0];
        let mut len: usize = metadata[1] as usize;

        payload.extend_from_slice(&metadata);

        if !Self::is_simple() && len & 0x80 != 0 {
            connection.read_exact(&mut buf)?;
            len = ((len & 0x7f) << 8) + buf[0] as usize;
            payload.push(buf[0]);
        }

        let start = payload.len();
        payload.reserve(len);
        payload.resize(start + len, 0_u8);

        connection.read_exact(&mut payload[start..])?;

        // println!(
        //     "received data ({}): {:02X?}",
        //     len - if Self::is_simple() { 1 } else { 4 },
        //     &payload
        // );

        if Self::is_simple() {
            assert_eq!(command, ID);
            Ok(self.read_response(&mut FixedReadBuffer::new(&payload[start + 1..]), len - 1)?)
        } else {
            assert_eq!(command, EXT_PACKET_ID);
            assert_eq!(ID, payload[start]);
            assert_eq!(CRC16.checksum(&payload), 0);

            if let Some(nack) = Nack::maybe_find(payload[start + 1]) {
                println!("NACK: {:?} ({})", &nack, payload[start + 1]);
                return Err(crate::error::Error::Generic("NACK"));
            }
            Ok(self.read_response(
                &mut FixedReadBuffer::new(&payload[start + 2..payload.len() - 2]),
                len - 4,
            )?)
        }
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
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

impl Nack {
    pub fn maybe_find(id: u8) -> Option<Self> {
        match id {
            0xFF => Some(Self::General),
            0xCE => Some(Self::InvalidCrc),
            0xD0 => Some(Self::PayloadTooSmall),
            0xD1 => Some(Self::TransferSizeTooLarge),
            0xD2 => Some(Self::CrcError),
            0xD3 => Some(Self::ProgramFileError),
            0xD4 => Some(Self::UninitializedTransfer),
            0xD5 => Some(Self::InvalidInitialization),
            0xD6 => Some(Self::NonPaddedData),
            0xD7 => Some(Self::UnexpectedPacketAddress),
            0xD8 => Some(Self::LengthMismatch),
            0xD9 => Some(Self::NonExistentDirectory),
            0xDA => Some(Self::FileIndexFull),
            0xDB => Some(Self::FileExists),
            _ => None,
        }
    }
}
