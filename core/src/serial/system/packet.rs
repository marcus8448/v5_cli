use crate::serial::system::Connection;
use crate::serial::CRC16;
use std::io::{Error, ErrorKind, Result, Write};
use std::mem::size_of;

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
    CopyScreenData = 0x28,
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

pub struct Packet<'a> {
    connection: &'a mut Connection,
    data_len: u16,
    pos: usize,
    data: Box<[u8]>,
}

impl<'a> Packet<'a> {
    pub(crate) fn create(connection: &'a mut Connection, id: PacketId, data_len: usize) -> Self {
        let mut data = vec![0_u8; 4 + 1 + 1 + (if data_len > 0x80 { 2 } else { 1 }) + data_len + 2]
            .into_boxed_slice(); // 4 byte header, 1 byte id, 1 byte command, 1-2 byte length, arbitrary data, 2 byte CRC
        data[..4].copy_from_slice(PACKET_HEADER);
        data[4] = EXT_PACKET_ID;
        data[5] = id.id();
        Packet {
            connection,
            data_len: data_len as u16,
            pos: if data_len > 0x80 { 8 } else { 7 },
            data,
        }
    }

    pub fn write_u8(&mut self, value: u8) -> Result<()> {
        self.data[self.pos..self.pos + size_of::<u8>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<u8>();
        Ok(())
    }

    pub fn write_i8(&mut self, value: i8) -> Result<()> {
        self.data[self.pos..self.pos + size_of::<i8>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<i8>();
        Ok(())
    }

    pub fn write_u16(&mut self, value: u16) -> Result<()> {
        self.data[self.pos..self.pos + size_of::<u16>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<u16>();
        Ok(())
    }

    pub fn write_i16(&mut self, value: i16) -> Result<()> {
        self.data[self.pos..self.pos + size_of::<i16>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<i16>();
        Ok(())
    }

    pub fn write_u32(&mut self, value: u32) -> Result<()> {
        self.data[self.pos..self.pos + size_of::<u32>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<u32>();
        Ok(())
    }

    pub fn write_i32(&mut self, value: i32) -> Result<()> {
        self.data[self.pos..self.pos + size_of::<i32>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<i32>();
        Ok(())
    }

    pub fn write_u64(&mut self, value: u64) -> Result<()> {
        self.data[self.pos..self.pos + size_of::<u64>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<u64>();
        Ok(())
    }

    pub fn write_i64(&mut self, value: i64) -> Result<()> {
        self.data[self.pos..self.pos + size_of::<i64>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<i64>();
        Ok(())
    }

    pub fn write_u128(&mut self, value: u128) -> Result<()> {
        self.data[self.pos..self.pos + size_of::<u128>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<u128>();
        Ok(())
    }

    pub fn write_i128(&mut self, value: i128) -> Result<()> {
        self.data[self.pos..self.pos + size_of::<i128>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<i128>();
        Ok(())
    }

    pub fn write_f32(&mut self, value: f32) -> Result<()> {
        self.data[self.pos..self.pos + size_of::<f32>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<f32>();
        Ok(())
    }

    pub fn write_f64(&mut self, value: f64) -> Result<()> {
        self.data[self.pos..self.pos + size_of::<f64>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<f64>();
        Ok(())
    }

    pub fn write(&mut self, slice: &[u8]) -> Result<()> {
        self.data[self.pos..self.pos + slice.len()].copy_from_slice(slice);
        self.pos += slice.len();
        Ok(())
    }

    pub fn write_str(&mut self, string: &str, target_len: u16) -> Result<()> {
        assert!(string.is_ascii());
        assert!(!string.contains('\0'));
        assert!(string.len() < target_len as usize); // < because of the null terminator
        self.data[self.pos..self.pos + string.len()].copy_from_slice(string.as_bytes());
        self.pos += string.len() + 1;
        self.data[self.pos - 1] = 0; // null terminator
        Ok(())
    }

    pub fn write_padded_str(&mut self, string: &str, target_len: u16) -> Result<()> {
        assert!(string.is_ascii());
        assert!(!string.contains('\0'));
        assert!(string.len() < target_len as usize);
        self.data[self.pos..self.pos + string.len()].copy_from_slice(string.as_bytes());
        self.pos += string.len() + 1;
        self.data[self.pos - 1] = 0; // null terminator
        self.pad((target_len - 1) - string.len() as u16)?;
        Ok(())
    }

    pub fn pad(&mut self, amount: u16) -> Result<()> {
        for x in 0..amount {
            self.data[self.pos + x as usize] = 0;
        }
        self.pos += amount as usize;
        Ok(())
    }

    pub fn send(mut self) -> Result<PacketResponse> {
        let len = self.data.len();
        let data_len = self.data_len;
        if data_len < 0x80 {
            self.data[6] = (data_len as u8).to_le();
        } else {
            self.data[6] = ((data_len >> 8 | 0x80) as u8).to_le();
            self.data[7] = ((data_len & 0xff) as u8).to_le();
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
        assert_eq!(sent_command, payload[data_start as usize]);
        assert_eq!(CRC16.checksum(&payload), 0);

        println!("recieved data: {:?}", &payload);

        if let Some(nack) = Nack::maybe_find(payload[(data_start + 1) as usize]) {
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
}
