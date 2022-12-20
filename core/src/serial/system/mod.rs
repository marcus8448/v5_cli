pub mod packet;

use std::io;
use std::io::{Write};
use std::time::Duration;
use chrono::{DateTime, FixedOffset, Local, NaiveDateTime};
use serialport::SerialPort;
use packet::{PacketId};
use crate::serial::system::packet::{BasicPacket, ExtendedPacket, Packet, PacketResponse};
use crate::serial::error::Error;
use crate::serial::error::Error::InvalidId;

const TEN_MILLIS: Duration = Duration::from_millis(10);
pub const EPOCH_MS_TO_JAN_1_2000: i64 = 946684800000;

type Result<T> = std::result::Result<T, Error>;

pub enum FileType {
    Bin,
    Ini
}

impl FileType {
    pub fn get_name(&self) -> &'static str {
        match self {
            Self::Bin => "bin",
            Self::Ini => "ini"
        }
    }
}

impl TryFrom<&str> for FileType {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "bin" => Ok(Self::Bin),
            "ini" => Ok(Self::Ini),
            _ => Err(InvalidId(255))
        }
    }
}

#[repr(u8)]
pub enum Product {
    Brain = 0x10,
    Controller = 0x11
}

impl TryFrom<u8> for Product {
    type Error = Error;

    fn try_from(id: u8) -> Result<Self> {
        match id {
            0x10 => Ok(Self::Brain),
            0x11 => Ok(Self::Controller),
            i => Err(Error::InvalidId(i))
        }
    }
}

#[repr(u8)]
pub enum UploadAction {
    Nothing = 0b0,
    Run = 0b01,
    RunScreen = 0b11,
}

impl Default for UploadAction {
    fn default() -> Self {
        Self::Nothing
    }
}

impl TryFrom<&str> for UploadAction {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self> {
        match value.to_lowercase().as_str() {
            "nothing" => Ok(Self::Nothing),
            "run" => Ok(Self::Run),
            "screen" => Ok(Self::RunScreen),
            _ => Err(InvalidId(255))
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
}

impl TryFrom<u8> for Vid {
    type Error = Error;

    fn try_from(id: u8) -> Result<Self> {
        match id {
            1 => Ok(Self::User),
            15 => Ok(Self::System),
            16 => Ok(Self::Rms),
            24 => Ok(Self::Pros),
            32 => Ok(Self::Mw),
            i => Err(Error::InvalidId(i))
        }
    }
}


#[repr(u8)]
pub enum Channel {
    Pit = 0,
    Download = 1
}

impl TryFrom<u8> for Channel {
    type Error = Error;

    fn try_from(id: u8) -> Result<Self> {
        match id {
            0 => Ok(Self::Pit),
            1 => Ok(Self::Download),
            i => Err(Error::InvalidId(i))
        }
    }
}

pub struct Connection {
    raw: Box<dyn SerialPort>
}

pub struct Brain {
    connection: Connection
}

pub struct FileMetadata {
    pub vid: Vid,
    pub size: u32,
    pub addr: u32,
    pub crc: u32,
    pub file_type: String,
    pub timestamp: DateTime<Local>,
    pub name: String
}

pub struct UploadMeta {
    max_packet_size: u16,
    file_size: u32,
    crc: u32
}

pub struct SystemVersion {
    major: u8,
    minor: u8,
    patch: u8,
    a: u8,
    b: u8,
    product: Product,
    flag: u8
}

impl Brain {
    pub fn new(connection: Box<dyn SerialPort>) -> Self {
        Brain {
            connection: Connection { raw: connection }
        }
    }

    pub fn get_raw_connection(&mut self) -> &mut Connection {
        &mut self.connection
    }

    pub fn read_file_metadata(&mut self, name: &str, vid: Vid) -> Result<FileMetadata> {
        assert!(name.is_ascii());
        assert!(name.len() > 0);

        let mut packet = self.connection.begin_extended_sized_packet(PacketId::GetFileMetadataByName, 26);
        packet.write_u8(vid.id())?;
        packet.write_padded_str(name, 24)?;

        let response = packet.send()?;
        let payload = response.get_data();
        let vid = Vid::try_from(payload[0])?;
        let size = u32::from_le_bytes(payload[1..5].try_into().unwrap());
        let addr = u32::from_le_bytes(payload[5..9].try_into().unwrap());
        let crc = u32::from_le_bytes(payload[9..13].try_into().unwrap());
        let file_type = String::from_utf8_lossy(payload[13..17].try_into().unwrap()).to_string();
        let timestamp = DateTime::<Local>::from_local(NaiveDateTime::from_timestamp_millis((u32::from_le_bytes(payload[17..21].try_into().unwrap()) as i64) * 1000_i64 + EPOCH_MS_TO_JAN_1_2000).unwrap(), FixedOffset::west_opt(0).unwrap());
        let name = u32::from_le_bytes(payload[21..45].try_into().unwrap()).to_string();
        Ok(FileMetadata { vid, size, addr, crc, file_type, timestamp, name })
    }

    pub fn upload_file(&mut self, target: TransferTarget, file_type: FileType, vid: Vid, file: &[u8], remote_name: &str, address: u32, crc: u32, overwrite: bool, timestamp: DateTime<Local>, linked_file: Option<(&str, Vid)>, action: UploadAction) -> Result<()> {
        let meta = self.initialize_file_transfer(TransferDirection::Upload, target, vid, overwrite, file.len() as u32, address, crc, 0b00_01_00, file_type, remote_name, timestamp)?;
        assert!(meta.file_size >= file.len() as u32);
        if let Some((name, vid)) = linked_file {
            self.link_file_transfer(name, vid)?;
        }
        let max_packet_size = meta.max_packet_size / 2;
        let max_packet_size = max_packet_size - (max_packet_size % 4); //4 byte alignment
        for i in (0..file.len()).step_by(max_packet_size as usize) {
            let end = file.len().min(i + max_packet_size as usize);
            self.write_file_transfer_part(&file[i..end], address + i as u32)?;
        }
        self.complete_file_transfer(action)?;
        Ok(())
    }

    pub fn link_file_transfer(&mut self, name: &str, vid: Vid) -> Result<PacketResponse> {
        let mut packet = self.connection.begin_extended_sized_packet(PacketId::SetFileTransferLink, 1);
        packet.write_u8(vid.id())?;
        packet.write_u8(0)?;
        packet.write_padded_str(name, 24)?;
        Ok(packet.send()?)
    }

    pub fn initialize_file_transfer(&mut self, direction: TransferDirection, target: TransferTarget, vid: Vid, overwrite: bool, length: u32, address: u32, crc: u32, version: u32, file_type: FileType, name: &str, timestamp: DateTime<Local>) -> Result<UploadMeta> {
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
        packet.write_u32(((&timestamp.timestamp_millis() - EPOCH_MS_TO_JAN_1_2000) as u32) / 1000)?;
        packet.write_u32(version)?;
        packet.write_padded_str(name, 24)?;
        let response = packet.send()?;
        let payload = response.get_data();
        Ok(UploadMeta { max_packet_size: u16::from_le_bytes(payload[0..2].try_into().unwrap()), file_size: u32::from_le_bytes(payload[2..6].try_into().unwrap()), crc: u32::from_le_bytes(payload[6..10].try_into().unwrap()) })
    }

    pub fn write_file_transfer_part(&mut self, slice: &[u8], address: u32) -> Result<()> {
        let mut packet = self.connection.begin_extended_sized_packet(PacketId::FileTransferWrite, (4 + slice.len() + 1) as u16);
        packet.write_u32(address)?;
        packet.write(slice)?;
        packet.write_u8(0)?;
        Ok(())
    }

    pub fn complete_file_transfer(&mut self, after: UploadAction) -> Result<()> {
        let mut packet = self.connection.begin_packet(PacketId::FileTransferWrite);
        packet.write_u8(after as u8)?;
        packet.send()?;
        Ok(())
    }

    pub fn get_system_version(&mut self) -> Result<SystemVersion> {
        let response = self.connection.begin_packet(PacketId::FileTransferWrite).send()?;
        let payload = response.get_data();

        Ok(SystemVersion {
            major: payload[0],
            minor: payload[1],
            patch: payload[2],
            a: payload[3],
            b: payload[4],
            product: Product::try_from(payload[5])?,
            flag: payload[6]
        })
    }
}

impl Connection {
    pub fn new(connection: Box<dyn SerialPort>) -> Self {
        Connection {
            raw: connection
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
        self.raw.flush()
    }
}
