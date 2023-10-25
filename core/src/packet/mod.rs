use std::fmt::Debug;
use std::mem::size_of;

use crate::buffer::{OwnedBuffer, RawRead, RawWrite};
use crate::connection::{Brain, CRC16};
use crate::packet::PacketType::Custom;

pub mod competition;
pub mod filesystem;
pub mod system;

const PACKET_HEADER: &[u8; 4] = &[0xc9, 0x36, 0xb8, 0x47];

const EXT_PACKET_ID: u8 = 0x56;

#[repr(u8)]
pub enum PacketType {
    ONE,
    Custom { id: u8, size: u16 }
}

impl Into<u8> for PacketType {
    fn into(self) -> u8 {
        self.get_id()
    }
}

impl PacketType {
    pub fn get_id(&self) -> u8 {
        match self {
            PacketType::ONE => 0x1,
            Custom { id, .. } => *id
        }
    }
}

pub struct PacketBuf<'a> {
    packet_type: PacketType,
    buffer: Box<[u8]>,
    pos: u16,
    brain: &'a mut Brain
}

impl<'a> PacketBuf<'a> {
    pub fn new(packet_type: PacketType, content_len: u16, brain: &'a mut Brain) -> Self {
        let meta_len = /*header*/ PACKET_HEADER.len() + /*ext id*/ 1 + /*command id*/  1 + if /*len*/ content_len < 0x80 { 1 } else { 2 };
        let size = meta_len + content_len as usize + /*CRC*/ size_of::<u16>();

        let id = packet_type.get_id();
        let mut buffer = Self { packet_type, buffer: vec![0_u8; size].into_boxed_slice(), pos: 0, brain };

        buffer.write_raw(PACKET_HEADER);

        buffer.write_u8(EXT_PACKET_ID);
        buffer.write_u8(id);

        if content_len < 0x80 {
            buffer.write_u8(content_len as u8);
        } else {
            buffer.write_u8((content_len >> 8 | 0x80) as u8);
            buffer.write_u8((content_len & 0xff) as u8);
        }

        buffer
    }

    pub async fn send(mut self) -> Result<OwnedBuffer, std::io::Error> {
        self.write_raw(&CRC16.checksum(&self.buffer[..self.pos as usize]).to_be_bytes());
        loop {
            self.brain.send_raw_packet(&self.buffer).await?;
            match self.brain.receive_raw_packet(self.packet_type.get_id()).await {
                Ok(data) => return Ok(data),
                Err(err) if err.kind() == std::io::ErrorKind::TimedOut => continue,
                Err(err) => return Err(err)
            };
        }
    }
}

impl<'a> RawWrite for PacketBuf<'a> {
    fn write_u8(&mut self, value: u8) {
        self.buffer[self.pos as usize..self.pos as usize + size_of::<u8>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<u8>() as u16;
    }

    fn write_i8(&mut self, value: i8) {
        self.buffer[self.pos as usize..self.pos as usize + size_of::<i8>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<i8>() as u16;
    }

    fn write_u16(&mut self, value: u16) {
        self.buffer[self.pos as usize..self.pos as usize + size_of::<u16>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<u16>() as u16;
    }

    fn write_i16(&mut self, value: i16) {
        self.buffer[self.pos as usize..self.pos as usize + size_of::<i16>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<i16>() as u16;
    }

    fn write_u32(&mut self, value: u32) {
        self.buffer[self.pos as usize..self.pos as usize + size_of::<u32>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<u32>() as u16;
    }

    fn write_i32(&mut self, value: i32) {
        self.buffer[self.pos as usize..self.pos as usize + size_of::<i32>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<i32>() as u16;
    }

    fn write_u64(&mut self, value: u64) {
        self.buffer[self.pos as usize..self.pos as usize + size_of::<u64>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<u64>() as u16;
    }

    fn write_i64(&mut self, value: i64) {
        self.buffer[self.pos as usize..self.pos as usize + size_of::<i64>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<i64>() as u16;
    }

    fn write_u128(&mut self, value: u128) {
        self.buffer[self.pos as usize..self.pos as usize + size_of::<u128>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<u128>() as u16;
    }

    fn write_i128(&mut self, value: i128) {
        self.buffer[self.pos as usize..self.pos as usize + size_of::<i128>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<i128>() as u16;
    }

    fn write_f32(&mut self, value: f32) {
        self.buffer[self.pos as usize..self.pos as usize + size_of::<f32>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<f32>() as u16;
    }

    fn write_f64(&mut self, value: f64) {
        self.buffer[self.pos as usize..self.pos as usize + size_of::<f64>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<f64>() as u16;
    }

    fn write_raw(&mut self, slice: &[u8]) {
        self.buffer[self.pos as usize..self.pos as usize + slice.len()].copy_from_slice(slice);
        self.pos += slice.len() as u16;
    }

    fn write_str(&mut self, string: &str, target_len: usize) {
        assert!(string.len() < target_len);
        self.buffer[self.pos as usize..self.pos as usize + string.len()].copy_from_slice(string.as_bytes());
        self.pos += target_len as u16;
    }

    fn pad(&mut self, amount: usize) {
        self.pos += amount as u16; // zero-initialized
    }

    fn get_data(self) -> Box<[u8]> {
        self.buffer
    }
}

enum Test {
    JAK = 0x0
}

#[async_trait::async_trait]
pub trait Packet<const ID: u8>: Debug {
    type Response;

    fn send_len(&self) -> usize;

    fn is_simple(&self) -> bool {
        false
    }

    fn write_buffer(&self, buffer: &mut dyn RawWrite) -> std::io::Result<()>;

    fn read_response(
        &self,
        buffer: &mut dyn RawRead,
        len: usize,
    ) -> std::io::Result<Self::Response>;
}
