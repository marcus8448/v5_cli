use std::mem::size_of;
use std::time::{Duration, SystemTime};

use crc::{Crc, CRC_16_XMODEM};
use log::{debug, error, warn};

use crate::buffer::{OwnedBuffer, RawWrite};
use crate::connection::{Nack, SerialConnection};

pub mod competition;
pub mod filesystem;
pub mod system;

const CRC16: Crc<u16> = Crc::<u16>::new(&CRC_16_XMODEM);

const PACKET_HEADER: &[u8; 4] = &[0xc9, 0x36, 0xb8, 0x47];
const RESPONSE_HEADER: [u8; 2] = [0xAA, 0x55];
const EXT_PACKET_ID: u8 = 0x56;
const TIMEOUT: Duration = Duration::from_millis(500);

pub struct Brain {
    connection: Box<dyn SerialConnection + Send>
}

impl Brain {
    pub fn new(connection: Box<dyn SerialConnection + Send>) -> Self {
        Self { connection }
    }

    pub fn packet(&mut self, content_len: usize, packet_id: u8) -> Packet {
        Packet::new(packet_id, content_len, self)
    }

    pub async fn send_raw_packet(&mut self, data: &[u8]) -> Result<(), std::io::Error> {
        assert_eq!(CRC16.checksum(data), 0);

        self.connection.clear().await?;
        self.connection.write_all(data).await?;
        self.connection.flush().await?;
        Ok(())
    }

    pub async fn find_packet_header(&mut self) -> Result<bool, std::io::Error> {
        let mut value = 0;
        let mut i = 0;
        let time = SystemTime::now();
        loop {
            if value == RESPONSE_HEADER[i] {
                i += 1;
                if i == RESPONSE_HEADER.len() {
                    break;
                }
            } else if i > 0 {
                i = 0;
                continue;
            }

            match self.connection.try_read_one().await {
                Ok(v) => value = v,
                Err(_) => {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                    value = 0;
                    if SystemTime::now()
                        .duration_since(time)
                        .unwrap_or(Duration::ZERO)
                        > TIMEOUT {
                        return Ok(false);
                    }
                }
            }
        }
        debug!(
            "response took {}ms",
            SystemTime::now().duration_since(time).unwrap().as_millis()
        );
        Ok(true)
    }

    pub async fn receive_raw_packet(&mut self, id: u8) -> Result<OwnedBuffer, std::io::Error> {
        match self.find_packet_header().await {
            Ok(true) => {}
            Ok(false) => {
                return Err(std::io::ErrorKind::TimedOut.into())
            }
            _ => {
                return Err(std::io::ErrorKind::UnexpectedEof.into())
            }
        };

        let mut payload = Vec::with_capacity(64);
        payload.extend_from_slice(&RESPONSE_HEADER);

        let mut metadata = [0_u8; 2];

        self.connection.read(&mut metadata).await?;
        let command = metadata[0];
        let mut len: usize = metadata[1] as usize;

        payload.extend_from_slice(&metadata);

        if len & 0b1000_0000 == 0b1000_0000 {
            let val = self.connection.try_read_one().await?;
            len = ((len & 0b0111_1111) << 8) + val as usize;
            payload.push(val);
        }

        let start = payload.len();
        payload.resize(start + len, 0_u8);

        self.connection.read(&mut payload[start..]).await?;

        assert_eq!(command, EXT_PACKET_ID);
        assert_eq!(id, payload[start]);
        assert_eq!(CRC16.checksum(&payload), 0);

        if let Ok(nack) = Nack::try_from(payload[start + 1]) {
            error!("NACK: {:?} ({})", &nack, payload[start + 1]);
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "NACK"));
        }

        Ok(OwnedBuffer::new(payload.into_boxed_slice(), start + 2))
    }

    pub async fn send_simple(
        &mut self,
        id: u8
    ) -> Result<OwnedBuffer, std::io::Error> {
        let mut buffer = [0_u8; 4 + 1 + /*CRC*/ size_of::<u16>()];
        buffer[0..PACKET_HEADER.len()].copy_from_slice(PACKET_HEADER);
        buffer[PACKET_HEADER.len()] = id;

        self.connection.clear().await?;
        self.connection.write_all(&buffer).await?;
        self.connection.flush().await?;

        let time = SystemTime::now();
        match self.find_packet_header().await {
            Ok(true) => {}
            Ok(false) => {
                return Err(std::io::ErrorKind::TimedOut.into())
            }
            _ => {
                return Err(std::io::ErrorKind::UnexpectedEof.into())
            }
        };

        debug!(
            "response took {}ms",
            SystemTime::now().duration_since(time).unwrap().as_millis()
        );

        let mut payload = Vec::with_capacity(64);
        payload.extend_from_slice(&RESPONSE_HEADER);

        let mut metadata = [0_u8; 2];

        self.connection.read(&mut metadata).await?;
        let command = self.connection.try_read_one().await?;
        let len = self.connection.try_read_one().await? as usize;

        payload.extend_from_slice(&metadata);

        let start = payload.len();
        payload.resize(start + len, 0_u8);

        self.connection.read(&mut payload[start..]).await?;

        assert_eq!(command, id);

        Ok(OwnedBuffer::new(payload.into_boxed_slice(), start + 1))
    }
}

pub struct Packet<'a> {
    packet_id: u8,
    buffer: Box<[u8]>,
    pos: usize,
    brain: &'a mut Brain
}

impl<'a> Packet<'a> {
    pub fn new(packet_id: u8, content_len: usize, brain: &'a mut Brain) -> Self {
        assert!(content_len < 0b1000_0000_0000_0000_u16 as usize);
        let meta_len = /*header*/ PACKET_HEADER.len() + /*ext id*/ 1 + /*command id*/  1 + if /*len*/ content_len < 0x80 { 1 } else { 2 };
        let size = meta_len + content_len + /*CRC*/ size_of::<u16>();

        let mut buffer = Self { packet_id, buffer: vec![0_u8; size].into_boxed_slice(), pos: 0, brain };

        buffer.write_raw(PACKET_HEADER);

        buffer.write_u8(EXT_PACKET_ID);
        buffer.write_u8(packet_id);

        if content_len >= 0b1000_0000 {
            buffer.write_u8((content_len >> 8 | 0b1000_0000) as u8);
            buffer.write_u8((content_len & 0xFF) as u8);
        } else {
            buffer.write_u8(content_len as u8);
        }

        buffer
    }

    pub async fn send(mut self) -> Result<OwnedBuffer, std::io::Error> {
        assert_eq!(self.buffer.len() - size_of::<u16>(), self.pos);

        self.write_raw(&CRC16.checksum(&self.buffer[..self.pos]).to_be_bytes());
        let mut failed = 0;
        loop {
            self.brain.send_raw_packet(&self.buffer).await?;
            match self.brain.receive_raw_packet(self.packet_id).await {
                Ok(data) => return Ok(data),
                Err(err) if err.kind() == std::io::ErrorKind::TimedOut => {},
                Err(err) => return Err(err)
            };
            failed += 1;
            warn!("Failed to send packet {} time(s)", failed)
        }
    }
}

impl<'a> RawWrite for Packet<'a> {
    fn write_u8(&mut self, value: u8) {
        self.buffer[self.pos..self.pos + size_of::<u8>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<u8>();
    }

    fn write_i8(&mut self, value: i8) {
        self.buffer[self.pos..self.pos + size_of::<i8>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<i8>();
    }

    fn write_u16(&mut self, value: u16) {
        self.buffer[self.pos..self.pos + size_of::<u16>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<u16>();
    }

    fn write_i16(&mut self, value: i16) {
        self.buffer[self.pos..self.pos + size_of::<i16>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<i16>();
    }

    fn write_u32(&mut self, value: u32) {
        self.buffer[self.pos..self.pos + size_of::<u32>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<u32>();
    }

    fn write_i32(&mut self, value: i32) {
        self.buffer[self.pos..self.pos + size_of::<i32>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<i32>();
    }

    fn write_u64(&mut self, value: u64) {
        self.buffer[self.pos..self.pos + size_of::<u64>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<u64>();
    }

    fn write_i64(&mut self, value: i64) {
        self.buffer[self.pos..self.pos + size_of::<i64>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<i64>();
    }

    fn write_u128(&mut self, value: u128) {
        self.buffer[self.pos..self.pos + size_of::<u128>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<u128>();
    }

    fn write_i128(&mut self, value: i128) {
        self.buffer[self.pos..self.pos + size_of::<i128>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<i128>();
    }

    fn write_f32(&mut self, value: f32) {
        self.buffer[self.pos..self.pos + size_of::<f32>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<f32>();
    }

    fn write_f64(&mut self, value: f64) {
        self.buffer[self.pos..self.pos + size_of::<f64>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<f64>();
    }

    fn write_raw(&mut self, slice: &[u8]) {
        self.buffer[self.pos..self.pos + slice.len()].copy_from_slice(slice);
        self.pos += slice.len();
    }

    fn write_str(&mut self, string: &str, target_len: usize) {
        assert!(string.len() < target_len);
        self.buffer[self.pos..self.pos + string.len()].copy_from_slice(string.as_bytes());
        self.pos += target_len;
    }

    fn pad(&mut self, amount: usize) {
        self.pos += amount; // zero-initialized
        assert!(self.pos <= self.buffer.len())
    }

    fn get_data(self) -> Box<[u8]> {
        self.buffer
    }
}
