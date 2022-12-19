pub mod packet;

use std::io;
use std::io::{ErrorKind, Read, Write};
use std::time::Duration;
use chrono::{NaiveDateTime};
use serialport::SerialPort;
use packet::PacketId;
use crate::serial::system::packet::{BasicPacket, ExtendedPacket, Packet};

const RESPONSE_HEADER: [u8; 2] = [0xAA, 0x55];
const TEN_MILLIS: Duration = Duration::from_millis(10);
pub const EPOCH_TO_JAN_1_2000: i64 = 946684800;

pub struct PacketResponse {
    command: u8,
    payload: Vec<u8>
}

pub enum FileType {
    Bin
}

impl FileType {
    pub fn get_name(&self) -> &'static str {
        match self {
            _Bin => "bin"
        }
    }
    pub fn from_name(name: &str) -> Self {
        match name {
            "bin" => Self::Bin,
            _ => todo!()
        }
    }
}

#[repr(u8)]
pub enum TransferDirection {
    Upload = 1,
    Download = 2
}

#[repr(u8)]
pub enum TransferTarget {
    DDR = 0,
    Flash = 1,
    Screen = 2
}

#[repr(u16)]
pub enum Vid {
    User = 1,
    System = 15,
    Rms = 16,
    Pros = 24,
    Mw = 32
}

impl Vid {
    pub fn id(self) -> u8 {
        return self as u8;
    }

    pub fn from_id(id: u8) -> Self {
        match id {
            1 => Self::User,
            15 => Self::System,
            16 => Self::Rms,
            24 => Self::Pros,
            32 => Self::Mw,
            _ => todo!()
        }
    }
}

#[repr(u8)]
pub enum Channel {
    Pit = 0,
    Download = 1
}

pub struct Connection {
    connection: Box<dyn SerialPort>
}

pub struct Brain {
    connection: Connection
}

pub struct FileMetadata {
    vid: Vid,
    size: u32,
    addr: u32,
    crc: u32,
    file_type: String,
    timestamp: NaiveDateTime,
    name: String
}

pub struct UploadMeta {
    max_packet_size: u16,
    file_size: u32,
    crc: u32
}

impl Brain {
    pub fn new(connection: Box<dyn SerialPort>) -> Self {
        Brain {
            connection: Connection { connection }
        }
    }

    pub fn get_raw_connection(&mut self) -> &mut Connection {
        &mut self.connection
    }

    pub fn read_file_metadata(&mut self, name: &str, vid: Vid) -> io::Result<FileMetadata> {
        assert!(name.is_ascii());
        assert!(name.len() > 0);

        let mut packet = self.connection.begin_extended_sized_packet(PacketId::GetFileMetadataByName, 26);
        packet.write_u8(vid.id())?;
        packet.write(name.as_bytes())?;
        packet.pad((name.len() - 24) as u16)?;
        packet.send()?;

        let payload = self.connection.receive_packet(10000)?.payload;
        let vid = Vid::from_id(payload[0]);
        let size = u32::from_le_bytes(payload[1..5].try_into().unwrap());
        let addr = u32::from_le_bytes(payload[5..9].try_into().unwrap());
        let crc = u32::from_le_bytes(payload[9..13].try_into().unwrap());
        let file_type = String::from_utf8_lossy(payload[13..17].try_into().unwrap()).to_string();
        let timestamp = NaiveDateTime::from_timestamp_millis(u32::from_le_bytes(payload[17..21].try_into().unwrap()) as i64 * 1000).unwrap();//.add(FixedOffset::)+ EPOCH_TO_JAN_1_2000;
        let name = u32::from_le_bytes(payload[21..45].try_into().unwrap()).to_string();
        Ok(FileMetadata { vid, size, addr, crc, file_type, timestamp, name })
    }

    pub fn upload_file(&mut self, file: &[u8], remote_name: &str, display_name: &str, address: u32, crc: u32, overwrite: bool, timestamp: chrono::NaiveDateTime) -> io::Result<()> {
        let meta = self.initialize_file_transfer(TransferDirection::Upload, TransferTarget::Flash, Vid::System, overwrite, file.len() as u32, address, crc, 0b00_01_00, FileType::Bin, display_name, timestamp)?;
        assert!(meta.file_size >= file.len() as u32);
        let max_packet_size = meta.max_packet_size / 2;
        let max_packet_size = max_packet_size - (max_packet_size % 4); //padding
        for i in (0..file.len()).step_by(max_packet_size as usize) {
            let end = file.len().min(i + max_packet_size as usize);
            self.write_file_transfer_part(&file[i..end], address + i as u32).unwrap();
        }

        Ok(())
    }

    pub fn write_file_transfer_part(&mut self, slice: &[u8], address: u32) -> io::Result<()> {
        // self.connection.write_packet_header()?;
        // x.copy_from_slice(address.to_le_bytes());
        // self.connection.send_extended_packet(PacketId::FileTransferWrite, )
        // logger(__name__).debug('Sending ext 0x13 command')
        // if isinstance(payload, str):
        //     payload = payload.encode(encoding='ascii')
        // if len(payload) % 4 != 0:
        //     padded_payload = bytes([*payload, *([0] * (4 - (len(payload) % 4)))])
        // else:
        //     padded_payload = payload
        // tx_fmt = "<I{}s".format(len(padded_payload))
        // tx_payload = struct.pack(tx_fmt, addr, padded_payload)
        // ret = self._txrx_ext_packet(0x13, tx_payload, 0)
        // logger(__name__).debug('Completed ext 0x13 command')
        Ok(())
    }

    pub fn initialize_file_transfer(&mut self, direction: TransferDirection, target: TransferTarget, vid: Vid, overwrite: bool, length: u32, address: u32, crc: u32, version: u32, file_type: FileType, name: &str, timestamp: NaiveDateTime) -> io::Result<UploadMeta> {
        assert!(name.len() <= 24);
        assert!(name.len() > 0);
        let mut packet = self.connection.begin_extended_sized_packet(PacketId::FileTransferInitialize, 52);
        packet.write_u8(direction as u8)?;
        packet.write_u8(target as u8)?;
        packet.write_u8(vid as u8)?;
        packet.write_u8(overwrite as u8)?;
        packet.write_u32(length)?;
        packet.write_u32(address)?;
        packet.write_u32(crc)?;
        packet.write(&file_type.get_name().as_bytes())?;
        packet.write_u32((&timestamp.timestamp() - EPOCH_TO_JAN_1_2000) as u32)?;
        packet.write_u32(version)?;
        packet.write(name.as_bytes())?;
        packet.pad((24 - name.len()) as u16)?;
        packet.send()?;
        let response = self.connection.receive_packet(5000)?.payload;
        Ok(UploadMeta { max_packet_size: u16::from_le_bytes(response[0..2].try_into().unwrap()), file_size: u32::from_le_bytes(response[2..6].try_into().unwrap()), crc: u32::from_le_bytes(response[6..10].try_into().unwrap()) })
    }
}

impl Connection {
    pub fn new(connection: Box<dyn SerialPort>) -> Self {
        Connection {
            connection
        }
    }

    pub fn begin_packet(&mut self, id: PacketId) -> BasicPacket {
        BasicPacket::create(self, id)
    }

    pub fn begin_extended_packet(&mut self, id: PacketId) -> ExtendedPacket {
        ExtendedPacket::create(self, id)
    }

    pub fn begin_extended_sized_packet(&mut self, id: PacketId, size: u16) -> ExtendedPacket {
        ExtendedPacket::create_sized(self, id, size)
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.connection.flush()
    }

    pub fn receive_packet(&mut self, timeout_millis: u64) -> io::Result<PacketResponse> {
        let mut buf: [u8; 1] = [0];
        let start = std::time::Instant::now();
        self.connection.set_timeout(Duration::from_millis(timeout_millis)).unwrap();
        let mut success = false;
        while start.elapsed().as_millis() < timeout_millis as u128 {
            self.connection.read_exact(&mut buf).unwrap();
            if buf[0] != RESPONSE_HEADER[0] {
                continue
            }
            self.connection.read_exact(&mut buf).unwrap();
            if buf[0] != RESPONSE_HEADER[1] {
                continue
            }
            success = true;
            break
        }
        if !success {
            return Err(io::Error::new(ErrorKind::InvalidData, "Packet header not found."));
        }

        self.connection.read_exact(&mut buf).unwrap();
        let command = buf[0];
        self.connection.read_exact(&mut buf).unwrap();
        let mut len: u16 = buf[0] as u16;
        let mut payload = Vec::new();
        if command == 0x56 && len & 0x80 == 0x80 {
            self.connection.read_exact(&mut buf).unwrap();
            len = ((len & 0x7f) << 8) + buf[0] as u16;
        }
        payload.reserve(len as usize);
        self.connection.read_exact(&mut payload).unwrap();
        Ok(PacketResponse {
            command,
            payload
        })
    }

    pub fn receive_packet_raw(&mut self, timeout_millis: u64) -> io::Result<PacketResponse> {
        let mut payload = Vec::new();
        payload.reserve(4);
        let start = std::time::Instant::now();
        self.connection.set_timeout(Duration::from_millis(timeout_millis)).unwrap();
        let mut success = false;
        while start.elapsed().as_millis() < timeout_millis as u128 {
            self.connection.read_exact(&mut payload[..1]).unwrap();
            if payload[0] != RESPONSE_HEADER[0] {
                continue
            }
            self.connection.read_exact(&mut payload[..2]).unwrap();
            if payload[1] != RESPONSE_HEADER[1] {
                continue
            }
            success = true;
            break
        }
        if !success {
            return Err(io::Error::new(ErrorKind::InvalidData, "Packet header not found."));
        }

        self.connection.read_exact(&mut payload[2..4]).unwrap();
        let command = payload[2];
        let len = payload[3];
        payload.reserve(len as usize);
        self.connection.read_exact(&mut payload).unwrap();
        Ok(PacketResponse {
            command,
            payload
        })
    }
}
