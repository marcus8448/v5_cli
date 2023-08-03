pub mod competition;
pub mod filesystem;
pub mod system;

use std::io;
use std::io::{Error, ErrorKind};
use crc::{Crc, CRC_16_XMODEM, CRC_32_BZIP2};
use crate::buffer::{ReadBuffer, WriteBuffer};

const PACKET_HEADER_LENGTH: usize = 4;
const PACKET_HEADER: &[u8; PACKET_HEADER_LENGTH] = &[0xc9, 0x36, 0xb8, 0x47];
const RESPONSE_HEADER: [u8; 2] = [0xAA, 0x55];
const EXT_PACKET_ID: u8 = 0x56;

pub trait Packet<const ID: u8> {
    type Response;

    fn get_size(&self) -> usize;

    fn write_buffer(&self, buffer: &mut dyn WriteBuffer) -> io::Result<()>;

    fn read_response(&self, buffer: &mut dyn ReadBuffer) -> io::Result<Self::Response>;

    fn is_simple() -> bool {
        false
    }
}

#[repr(u8)]
#[derive(Debug)]
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

pub fn send(mut self) -> io::Result<PacketResponse> {
    let len = self.data.len();
    if LENGTH < 0x80 { //fixme: simple
        self.data[6] = (LENGTH as u8).to_le();
    } else {
        self.data[6] = ((LENGTH >> 8 | 0x80) as u8).to_le();
        self.data[7] = ((LENGTH & 0xff) as u8).to_le();
    }

    let sum = &CRC16.checksum(&self.data[..self.pos]).to_le_bytes();
    self.data[self.pos] = sum[1];
    self.data[self.pos + 1] = sum[0];
    self.pos += 2;
    assert_eq!(self.pos, len);

    println!("sent: {:?}", self.data);
    let x = self.connection.raw.write(&self.data)?;
    assert_eq!(x, self.data.len());
    self.connection.flush()?;

    let sent_command = self.data[5];
    let mut payload = Vec::from(self.data);
    payload.clear();
    payload.resize(4, 0_u8);

    loop {
        println!("awaiting response");
        self.connection.raw.read_exact(&mut payload[0..1]).unwrap();
        println!("a{}", payload[0]);
        if payload[0] != RESPONSE_HEADER[0] {
            continue;
        }
        self.connection.raw.read_exact(&mut payload[1..2]).unwrap();
        println!("b{}", payload[1]);
        if payload[1] != RESPONSE_HEADER[1] {
            continue;
        }
        break;
    }
    println!("received header");

    self.connection.raw.read_exact(&mut payload[2..3]).unwrap();
    let command = payload[2];
    println!("command: {}", command);
    self.connection.raw.read_exact(&mut payload[3..4]).unwrap();
    let mut len: u16 = payload[3] as u16;
    let data_start: usize;
    if command == EXT_PACKET_ID && len & 0x80 == 0x80 {
        payload.push(0);
        self.connection.raw.read_exact(&mut payload[4..5]).unwrap();
        len = ((len & 0x7f) << 8) + payload[4] as u16;

        data_start = 5;
    } else {
        data_start = 4;
    }
    println!("length: {}", len);
    payload.resize(payload.len() + len as usize, 0_u8);
    self.connection
        .raw
        .read_exact(&mut payload[data_start..])
        .unwrap();
    assert_eq!(command, EXT_PACKET_ID);
    assert_eq!(sent_command, payload[data_start]);
    assert_eq!(CRC16.checksum(&payload), 0);

    println!("recieved data: {:?}", &payload);

    if let Some(nack) = Nack::maybe_find(payload[(data_start + 1)]) {
        return Err(Error::new(
            ErrorKind::Unsupported,
            format!("NACK: {:?}", nack),
        ));
    }

    //todo: check length

    let data_end = payload.len() - 2;

    Ok(PacketResponse {
        command,
        payload,
        data_start: data_start + 2, // real command, NACK
        data_end,
    })
}

pub const CRC16: Crc<u16> = Crc::<u16>::new(&CRC_16_XMODEM);
pub const CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_BZIP2);
