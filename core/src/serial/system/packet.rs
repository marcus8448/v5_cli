use crate::serial::system::Connection;
use crate::serial::CRC16;
use std::io::{Error, ErrorKind, Result, Write};

const PACKET_HEADER: &[u8; 4] = &[0xc9, 0x36, 0xb8, 0x47];
const RESPONSE_HEADER: [u8; 2] = [0xAA, 0x55];
const EXT_PACKET_ID: u8 = 0x56;

#[repr(u8)]
pub enum PacketId {
    GetSystemVersion = 0xA4,
    FileTransferChannel = 0x10,
    FileTransferInitialize = 0x11,
    FileTransferComplete = 0x12,
    FileTransferWrite = 0x13,
    FileTransferRead = 0x14,
    SetFileTransferLink = 0x15,
    GetDirectoryCount = 0x16,
    GetFileMetadataByIndex = 0x17,
    ExecuteProgram = 0x18,
    GetFileMetadataByName = 0x19,
    GetProduct = 0x21,
    GetSystemStatus = 0x22,
    SetProgramFileMetadata = 0x1A,
    DeleteFile = 0x1B,
    GetFileSlot = 0x1C,
    GetKernelVariable = 0x2E,
    SetKernelVariable = 0x2F,

    ManageCompetition = 0xC1,
}

impl PacketId {
    fn id(self) -> u8 {
        return self as u8;
    }
}

pub struct PacketResponse {
    command: u8,
    payload: Vec<u8>,
    data_start: usize,
    data_end: usize,
}

impl PacketResponse {
    pub fn get_command(&self) -> u8 {
        self.command
    }

    pub fn get_full_payload(&self) -> &[u8] {
        &self.payload
    }

    pub fn get_data(&self) -> &[u8] {
        &self.payload[self.data_start..self.data_end]
    }
}

#[repr(u8)]
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

pub trait Packet<'a> {
    fn create(connection: &'a mut Connection, id: PacketId) -> Self;

    fn write_u8(&mut self, value: u8) -> Result<()>;
    fn write_i8(&mut self, value: i8) -> Result<()>;
    fn write_u16(&mut self, value: u16) -> Result<()>;
    fn write_i16(&mut self, value: i16) -> Result<()>;
    fn write_u32(&mut self, value: u32) -> Result<()>;
    fn write_i32(&mut self, value: i32) -> Result<()>;
    fn write_u64(&mut self, value: u64) -> Result<()>;
    fn write_i64(&mut self, value: i64) -> Result<()>;
    fn write_u128(&mut self, value: u128) -> Result<()>;
    fn write_i128(&mut self, value: i128) -> Result<()>;
    fn write_f32(&mut self, value: u32) -> Result<()>;
    fn write_f64(&mut self, value: u64) -> Result<()>;
    fn write(&mut self, slice: &[u8]) -> Result<()>;
    fn write_str(&mut self, string: &str, target_len: u16) -> Result<()>;
    fn write_padded_str(&mut self, string: &str, target_len: u16) -> Result<()>;
    fn pad(&mut self, amount: u16) -> Result<()>;
    fn send(self) -> Result<PacketResponse>;
}

pub struct BasicPacket<'a> {
    connection: &'a mut Connection,
}

pub struct ExtendedPacket<'a> {
    connection: &'a mut Connection,
    data: Vec<u8>,
}

impl<'a> Packet<'a> for BasicPacket<'a> {
    fn create(connection: &'a mut Connection, id: PacketId) -> Self {
        connection.raw.write(PACKET_HEADER).unwrap();
        connection
            .raw
            .write(std::slice::from_ref(&id.id()))
            .unwrap();
        BasicPacket { connection }
    }

    fn write_u8(&mut self, value: u8) -> Result<()> {
        self.connection.raw.write(&value.to_le_bytes())?;
        Ok(())
    }

    fn write_i8(&mut self, value: i8) -> Result<()> {
        self.connection.raw.write(&value.to_le_bytes())?;
        Ok(())
    }

    fn write_u16(&mut self, value: u16) -> Result<()> {
        self.connection.raw.write(&value.to_le_bytes())?;
        Ok(())
    }

    fn write_i16(&mut self, value: i16) -> Result<()> {
        self.connection.raw.write(&value.to_le_bytes())?;
        Ok(())
    }

    fn write_u32(&mut self, value: u32) -> Result<()> {
        self.connection.raw.write(&value.to_le_bytes())?;
        Ok(())
    }

    fn write_i32(&mut self, value: i32) -> Result<()> {
        self.connection.raw.write(&value.to_le_bytes())?;
        Ok(())
    }

    fn write_u64(&mut self, value: u64) -> Result<()> {
        self.connection.raw.write(&value.to_le_bytes())?;
        Ok(())
    }

    fn write_i64(&mut self, value: i64) -> Result<()> {
        self.connection.raw.write(&value.to_le_bytes())?;
        Ok(())
    }

    fn write_u128(&mut self, value: u128) -> Result<()> {
        self.connection.raw.write(&value.to_le_bytes())?;
        Ok(())
    }

    fn write_i128(&mut self, value: i128) -> Result<()> {
        self.connection.raw.write(&value.to_le_bytes())?;
        Ok(())
    }

    fn write_f32(&mut self, value: u32) -> Result<()> {
        self.connection.raw.write(&value.to_le_bytes())?;
        Ok(())
    }

    fn write_f64(&mut self, value: u64) -> Result<()> {
        self.connection.raw.write(&value.to_le_bytes())?;
        Ok(())
    }

    fn write(&mut self, slice: &[u8]) -> Result<()> {
        self.connection.raw.write(slice)?;
        Ok(())
    }

    fn write_str(&mut self, string: &str, target_len: u16) -> Result<()> {
        assert!(string.is_ascii());
        assert!(string.len() < target_len as usize);
        self.connection.raw.write(string.as_bytes())?;
        self.connection.raw.write(std::slice::from_ref(&0))?; // null terminator
        Ok(())
    }

    fn write_padded_str(&mut self, string: &str, target_len: u16) -> Result<()> {
        assert!(string.is_ascii());
        assert!(!string.contains('\0'));
        assert!(string.len() < target_len as usize);
        self.connection.raw.write(string.as_bytes())?;
        self.connection.raw.write(std::slice::from_ref(&0))?; // null terminator
        self.pad((target_len - 1) - string.len() as u16)?;
        Ok(())
    }

    fn pad(&mut self, amount: u16) -> Result<()> {
        let zero = std::slice::from_ref(&0_u8);
        for _ in 0..amount {
            self.connection.raw.write(zero)?;
        }
        Ok(())
    }

    fn send(self) -> Result<PacketResponse> {
        self.connection.flush()?;

        let mut payload = Vec::new();
        payload.reserve(4);
        loop {
            self.connection.raw.read_exact(&mut payload[0..1]).unwrap();
            if payload[0] != RESPONSE_HEADER[0] {
                continue;
            }
            self.connection.raw.read_exact(&mut payload[1..2]).unwrap();
            if payload[1] != RESPONSE_HEADER[1] {
                continue;
            }
            break;
        }

        self.connection.raw.read_exact(&mut payload[2..3]).unwrap();
        let command = payload[2];
        self.connection.raw.read_exact(&mut payload[3..4]).unwrap();
        let mut len: u16 = payload[3] as u16;
        let data_start: usize;
        if command == EXT_PACKET_ID && len & 0x80 == 0x80 {
            self.connection.raw.read_exact(&mut payload[4..5]).unwrap();
            len = ((len & 0x7f) << 8) + payload[4] as u16;

            data_start = payload.len();
            payload.reserve(len as usize);
            self.connection.raw.read_exact(&mut payload[5..]).unwrap();
        } else {
            data_start = payload.len();
            payload.reserve(len as usize);
            self.connection.raw.read_exact(&mut payload[4..]).unwrap();
        }
        let data_end = payload.len();
        Ok(PacketResponse {
            command,
            payload,
            data_start,
            data_end,
        })
    }
}

impl<'a> ExtendedPacket<'a> {
    pub(crate) fn create_sized(connection: &'a mut Connection, id: PacketId, size: u16) -> Self {
        let mut vec = Vec::new();
        vec.reserve((4 + 1 + 1 + 2 + size + 2) as usize); // 4 byte header, 1 byte id, 1 byte command, 2 byte length, arbitrary data, 2 byte CRC
        vec.extend_from_slice(PACKET_HEADER);
        vec.push(EXT_PACKET_ID);
        vec.push(id.id());
        ExtendedPacket {
            connection,
            data: vec,
        }
    }
}

impl<'a> Packet<'a> for ExtendedPacket<'a> {
    fn create(connection: &'a mut Connection, id: PacketId) -> Self {
        ExtendedPacket::create_sized(connection, id, 64)
    }

    fn write_u8(&mut self, value: u8) -> Result<()> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn write_i8(&mut self, value: i8) -> Result<()> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn write_u16(&mut self, value: u16) -> Result<()> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn write_i16(&mut self, value: i16) -> Result<()> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn write_u32(&mut self, value: u32) -> Result<()> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn write_i32(&mut self, value: i32) -> Result<()> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn write_u64(&mut self, value: u64) -> Result<()> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn write_i64(&mut self, value: i64) -> Result<()> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn write_u128(&mut self, value: u128) -> Result<()> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn write_i128(&mut self, value: i128) -> Result<()> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn write_f32(&mut self, value: u32) -> Result<()> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn write_f64(&mut self, value: u64) -> Result<()> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn write(&mut self, slice: &[u8]) -> Result<()> {
        self.data.extend_from_slice(slice);
        Ok(())
    }

    fn write_str(&mut self, string: &str, target_len: u16) -> Result<()> {
        assert!(string.is_ascii());
        assert!(!string.contains('\0'));
        assert!(string.len() < target_len as usize);
        self.data.extend_from_slice(string.as_bytes());
        self.data.push(0); // null terminator
        Ok(())
    }

    fn write_padded_str(&mut self, string: &str, target_len: u16) -> Result<()> {
        assert!(string.is_ascii());
        assert!(!string.contains('\0'));
        assert!(string.len() < target_len as usize);
        self.data.extend_from_slice(string.as_bytes());
        self.data.push(0); // null terminator
        self.pad((target_len - 1) - string.len() as u16)?;
        Ok(())
    }

    fn pad(&mut self, amount: u16) -> Result<()> {
        for _ in 0..amount {
            self.data.push(0);
        }
        Ok(())
    }

    fn send(mut self) -> Result<PacketResponse> {
        self.data.splice(6..6, self.data.len().to_le_bytes());
        self.connection.raw.write(&self.data)?;
        self.connection
            .raw
            .write(&CRC16.checksum(&self.data).to_le_bytes())?;
        self.connection.flush()?;

        let sent_command = self.data[5];

        let mut payload = self.data;
        payload.clear();
        payload.reserve(5);

        loop {
            self.connection.raw.read_exact(&mut payload[0..1]).unwrap();
            if payload[0] != RESPONSE_HEADER[0] {
                continue;
            }
            self.connection.raw.read_exact(&mut payload[1..2]).unwrap();
            if payload[1] != RESPONSE_HEADER[1] {
                continue;
            }
            break;
        }

        self.connection.raw.read_exact(&mut payload[2..3]).unwrap();
        let command = payload[2];
        self.connection.raw.read_exact(&mut payload[3..4]).unwrap();
        let mut len: u16 = payload[3] as u16;
        let data_start: usize;
        if command == EXT_PACKET_ID && len & 0x80 == 0x80 {
            self.connection.raw.read_exact(&mut payload[4..5]).unwrap();
            len = ((len & 0x7f) << 8) + payload[4] as u16;

            data_start = payload.len();
            payload.reserve(len as usize);
            self.connection.raw.read_exact(&mut payload[5..]).unwrap();
        } else {
            data_start = payload.len();
            payload.reserve(len as usize);
            self.connection.raw.read_exact(&mut payload[4..]).unwrap();
        }

        assert_eq!(command, EXT_PACKET_ID);
        assert_eq!(sent_command, payload[data_start as usize]);
        assert_eq!(CRC16.checksum(&payload), 0);

        if let Some(nack) = Nack::maybe_find(payload[(data_start + 1) as usize]) {
            return Err(Error::new(
                ErrorKind::Unsupported,
                format!("NACK: {}", nack as u8),
            ));
        }

        //todo: check length

        let data_end = payload.len() - 2;

        Ok(PacketResponse {
            command,
            payload,
            data_start,
            data_end,
        })
    }
}
