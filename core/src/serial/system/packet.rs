use crate::serial::system::Connection;
use std::io::{Result, Write};
use crate::serial::CRC16;

const PACKET_HEADER: &[u8; 4] = &[0xc9, 0x36, 0xb8, 0x47];
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
    GetFileSlot = 0x1C
}

impl PacketId {
    fn id(self) -> u8 {
        return self as u8;
    }
}

pub trait Packet<'a> {
    fn create(connection: &'a mut Connection, id: PacketId) -> Self;

    fn write_u8(&mut self, value: u8) -> Result<&mut Self>;
    fn write_i8(&mut self, value: i8) -> Result<&mut Self>;
    fn write_u16(&mut self, value: u16) -> Result<&mut Self>;
    fn write_i16(&mut self, value: i16) -> Result<&mut Self>;
    fn write_u32(&mut self, value: u32) -> Result<&mut Self>;
    fn write_i32(&mut self, value: i32) -> Result<&mut Self>;
    fn write_u64(&mut self, value: u64) -> Result<&mut Self>;
    fn write_i64(&mut self, value: i64) -> Result<&mut Self>;
    fn write_u128(&mut self, value: u128) -> Result<&mut Self>;
    fn write_i128(&mut self, value: i128) -> Result<&mut Self>;
    fn write_f32(&mut self, value: u32) -> Result<&mut Self>;
    fn write_f64(&mut self, value: u64) -> Result<&mut Self>;
    fn write(&mut self, slice: &[u8]) -> Result<&mut Self>;
    fn pad(&mut self, amount: u16) -> Result<&mut Self>;
    fn send(self) -> Result<()>;
}

pub struct BasicPacket<'a> {
    connection: &'a mut Connection
}

pub struct ExtendedPacket<'a> {
    connection: &'a mut Connection,
    data: Vec<u8>
}

impl<'a> Packet<'a> for BasicPacket<'a> {
    fn create(connection: &'a mut Connection, id: PacketId) -> Self {
        connection.connection.write(PACKET_HEADER).unwrap();
        connection.connection.write(std::slice::from_ref(&id.id())).unwrap();
        BasicPacket { connection }
    }

    fn write_u8(&mut self, value: u8) -> Result<&mut Self> {
        self.connection.connection.write(&value.to_le_bytes())?;
        Ok(self)
    }

    fn write_i8(&mut self, value: i8) -> Result<&mut Self> {
        self.connection.connection.write(&value.to_le_bytes())?;
        Ok(self)
    }

    fn write_u16(&mut self, value: u16) -> Result<&mut Self> {
        self.connection.connection.write(&value.to_le_bytes())?;
        Ok(self)
    }

    fn write_i16(&mut self, value: i16) -> Result<&mut Self> {
        self.connection.connection.write(&value.to_le_bytes())?;
        Ok(self)
    }

    fn write_u32(&mut self, value: u32) -> Result<&mut Self> {
        self.connection.connection.write(&value.to_le_bytes())?;
        Ok(self)
    }

    fn write_i32(&mut self, value: i32) -> Result<&mut Self> {
        self.connection.connection.write(&value.to_le_bytes())?;
        Ok(self)
    }

    fn write_u64(&mut self, value: u64) -> Result<&mut Self> {
        self.connection.connection.write(&value.to_le_bytes())?;
        Ok(self)
    }

    fn write_i64(&mut self, value: i64) -> Result<&mut Self> {
        self.connection.connection.write(&value.to_le_bytes())?;
        Ok(self)
    }

    fn write_u128(&mut self, value: u128) -> Result<&mut Self> {
        self.connection.connection.write(&value.to_le_bytes())?;
        Ok(self)
    }

    fn write_i128(&mut self, value: i128) -> Result<&mut Self> {
        self.connection.connection.write(&value.to_le_bytes())?;
        Ok(self)
    }

    fn write_f32(&mut self, value: u32) -> Result<&mut Self> {
        self.connection.connection.write(&value.to_le_bytes())?;
        Ok(self)
    }

    fn write_f64(&mut self, value: u64) -> Result<&mut Self> {
        self.connection.connection.write(&value.to_le_bytes())?;
        Ok(self)
    }

    fn write(&mut self, slice: &[u8]) -> Result<&mut Self> {
        self.connection.connection.write(slice)?;
        Ok(self)
    }

    fn pad(&mut self, amount: u16) -> Result<&mut Self> {
        let zero = std::slice::from_ref(&0_u8);
        for _ in 0..amount {
            self.connection.connection.write(zero)?;
        }
        Ok(self)
    }

    fn send(self) -> Result<()> {
        self.connection.flush()?;
        Ok(())
    }
}

impl<'a> ExtendedPacket<'a> {
    pub(crate) fn create_sized(connection: &'a mut Connection, id: PacketId, size: u16) -> Self {
        let mut vec = Vec::new();
        vec.reserve((4 + 1 + 1 + 2 + size + 2) as usize); // 4 byte header, 1 byte id, 1 byte command, 2 byte length, arbitrary data, 2 byte CRC
        vec.extend_from_slice(PACKET_HEADER);
        vec.push(EXT_PACKET_ID);
        vec.push(id.id());
        ExtendedPacket { connection, data: vec }
    }
}

impl<'a> Packet<'a> for ExtendedPacket<'a> {
    fn create(connection: &'a mut Connection, id: PacketId) -> Self {
        ExtendedPacket::create_sized(connection, id, 64)
    }

    fn write_u8(&mut self, value: u8) -> Result<&mut Self> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(self)
    }

    fn write_i8(&mut self, value: i8) -> Result<&mut Self> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(self)
    }

    fn write_u16(&mut self, value: u16) -> Result<&mut Self> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(self)
    }

    fn write_i16(&mut self, value: i16) -> Result<&mut Self> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(self)
    }

    fn write_u32(&mut self, value: u32) -> Result<&mut Self> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(self)
    }

    fn write_i32(&mut self, value: i32) -> Result<&mut Self> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(self)
    }

    fn write_u64(&mut self, value: u64) -> Result<&mut Self> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(self)
    }

    fn write_i64(&mut self, value: i64) -> Result<&mut Self> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(self)
    }

    fn write_u128(&mut self, value: u128) -> Result<&mut Self> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(self)
    }

    fn write_i128(&mut self, value: i128) -> Result<&mut Self> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(self)
    }

    fn write_f32(&mut self, value: u32) -> Result<&mut Self> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(self)
    }

    fn write_f64(&mut self, value: u64) -> Result<&mut Self> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(self)
    }

    fn write(&mut self, slice: &[u8]) -> Result<&mut Self> {
        self.data.extend_from_slice(slice);
        Ok(self)
    }

    fn pad(&mut self, amount: u16) -> Result<&mut Self> {
        for _ in 0..amount {
            self.data.push(0);
        }
        Ok(self)
    }

    fn send(mut self) -> Result<()> {
        self.data.splice(6..6, self.data.len().to_le_bytes());
        self.connection.connection.write(&self.data)?;
        self.connection.connection.write(&CRC16.checksum(&self.data).to_le_bytes())?;
        self.connection.flush()?;
        Ok(())
    }
}
