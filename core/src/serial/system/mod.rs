pub mod packet;

use crate::error::Error;
use crate::serial::system::packet::{Packet, PacketResponse};
use packet::PacketId;
use serialport::SerialPort;
use std::fmt::{Display, Formatter};
use std::io;
use std::io::{Read, Write};
use std::ops::{Add, Sub};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use log::info;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

pub const EPOCH_MS_TO_JAN_1_2000: u64 = 946684800000;

type Result<T> = std::result::Result<T, Error>;

pub enum FileType {
    Bin,
    Ini,
}

impl FileType {
    fn get_name(&self) -> &'static str {
        match self {
            Self::Bin => "bin",
            Self::Ini => "ini",
        }
    }
}

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum CompetitionStatus {
    Disabled = 11,
    Autonomous = 10,
    OpControl = 8,
}

impl TryFrom<u8> for CompetitionStatus {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            11 => Ok(Self::Disabled),
            10 => Ok(Self::Autonomous),
            8 => Ok(Self::OpControl),
            _ => Err(Error::InvalidId(value)),
        }
    }
}

impl TryFrom<&str> for FileType {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self> {
        match value.to_lowercase().as_str() {
            "bin" => Ok(Self::Bin),
            "ini" => Ok(Self::Ini),
            _ => Err(Error::InvalidName(value.to_string())),
        }
    }
}

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum Product {
    Brain,
    Controller { connected: bool },
}

impl Product {
    fn parse(id: u8, flag: u8) -> Result<Self> {
        match id {
            0x10 => Ok(Self::Brain),
            0x11 => Ok(Self::Controller {
                connected: flag & 0b10 == 0b10,
            }),
            id => Err(Error::InvalidId(id)),
        }
    }

    pub fn get_id(&self) -> u8 {
        match &self {
            Self::Brain => 0x10,
            Self::Controller { .. } => 0x11,
        }
    }

    pub fn get_name(&self) -> &'static str {
        match self {
            Self::Brain => "Brain",
            Self::Controller { connected: true } => "Controller (Connected)",
            Self::Controller { connected: false } => "Controller (Disconnected)",
        }
    }
}

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum UploadAction {
    Nothing = 0b0,
    Run = 0b01,
    RunScreen = 0b11,
}

impl UploadAction {
    pub fn get_id(&self) {
        *self as u8;
    }
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
            _ => Err(Error::InvalidName(value.to_string())),
        }
    }
}

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum TransferDirection {
    Upload = 1,
    Download = 2,
}

impl TransferDirection {
    fn get_id(&self) -> u8 {
        *self as u8
    }
}

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum TransferTarget {
    DDR = 0,
    Flash = 1,
    Screen = 2,
}

impl TransferTarget {
    fn get_id(&self) -> u8 {
        *self as u8
    }
}

#[repr(u16)]
#[derive(Copy, Clone)]
pub enum Vid {
    User = 1,
    System = 15,
    Rms = 16,
    Pros = 24,
    V5Cli = 27,
    Mw = 32,
    Custom(u8),
}

impl Vid {
    pub fn get_id(&self) -> u8 {
        match self {
            Self::User => 1,
            Self::System => 15,
            Self::Rms => 16,
            Self::Pros => 24,
            Self::V5Cli => 27,
            Self::Mw => 32,
            Self::Custom(c) => *c,
        }
    }
}

impl From<u8> for Vid {
    fn from(id: u8) -> Self {
        match id {
            1 => Self::User,
            15 => Self::System,
            16 => Self::Rms,
            24 => Self::Pros,
            27 => Self::V5Cli,
            32 => Self::Mw,
            i => Self::Custom(i),
        }
    }
}

impl Display for Vid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({})",
            match self {
                Self::User => "user",
                Self::System => "system",
                Self::Rms => "rms",
                Self::Pros => "pros",
                Self::V5Cli => "v5_cli",
                Self::Mw => "mw",
                Self::Custom(_) => "custom",
            },
            self.get_id()
        )
    }
}

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum KernelVariable {
    TeamNumber = 7,
    RobotName = 16,
}

impl KernelVariable {
    pub fn get_max_len(&self) -> u8 {
        match self {
            Self::TeamNumber => 7,
            Self::RobotName => 16,
        }
    }
}

impl KernelVariable {
    fn get_id(&self) -> u8 {
        *self as u8
    }
}

impl TryFrom<u8> for KernelVariable {
    type Error = Error;

    fn try_from(id: u8) -> Result<Self> {
        match id {
            7 => Ok(Self::TeamNumber),
            16 => Ok(Self::RobotName),
            i => Err(Error::InvalidId(i)),
        }
    }
}

impl TryFrom<&str> for KernelVariable {
    type Error = Error;

    fn try_from(id: &str) -> Result<Self> {
        match id.to_lowercase().as_str() {
            "team_number" => Ok(Self::TeamNumber),
            "robot_name" => Ok(Self::RobotName),
            s => Err(Error::InvalidName(s.to_string())),
        }
    }
}
impl TryFrom<String> for KernelVariable {
    type Error = Error;

    fn try_from(id: String) -> Result<Self> {
        match id.to_lowercase().as_str() {
            "team_number" => Ok(Self::TeamNumber),
            "robot_name" => Ok(Self::RobotName),
            _ => Err(Error::InvalidName(id)),
        }
    }
}

#[repr(u8)]
pub enum Channel {
    Pit = 0,
    Download = 1,
}

impl TryFrom<u8> for Channel {
    type Error = Error;

    fn try_from(id: u8) -> Result<Self> {
        match id {
            0 => Ok(Self::Pit),
            1 => Ok(Self::Download),
            i => Err(Error::InvalidId(i)),
        }
    }
}

pub struct Connection {
    raw: Box<dyn SerialPort>,
}

pub struct Brain {
    connection: Connection,
}

pub struct FileMetadata {
    pub vid: Vid,
    pub size: u32,
    pub addr: u32,
    pub crc: u32,
    pub file_type: String,
    pub timestamp: SystemTime,
    pub name: String,
}

pub struct FileMetadata2 {
    pub index: u8,
    pub size: u32,
    pub addr: u32,
    pub crc: u32,
    pub file_type: String,
    pub timestamp: SystemTime,
    pub version: String,
    pub name: String,
}

impl Display for FileMetadata2 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Name: {}\nVersion: {}\nSize: {}\nAddress: {}\nCRC: {}\nFile Type: {}\nTimestamp: {}",
            self.name,
            self.version,
            self.size,
            self.addr,
            self.crc,
            self.file_type,
            OffsetDateTime::from(self.timestamp)
                .format(&Rfc3339)
                .unwrap()
        )
    }
}

pub struct UploadMeta {
    max_packet_size: u16,
    file_size: u32,
    crc: u32,
}

pub struct SystemVersion {
    major: u8,
    minor: u8,
    patch: u8,
    a: u8,
    b: u8,
    product: Product,
}

impl SystemVersion {
    pub fn get_version(&self) -> String {
        format!(
            "{}.{}.{}-{}.{}",
            self.major, self.minor, self.patch, self.a, self.b
        )
    }

    pub fn get_product(&self) -> &Product {
        &self.product
    }

    pub fn is_brain_available(&self) -> bool {
        match self.product {
            Product::Brain => true,
            Product::Controller { connected } => connected,
        }
    }
}

pub struct Version {
    major: u8,
    minor: u8,
    patch: u8,
    extra: u8,
}

impl Version {
    fn new(major: u8, minor: u8, patch: u8, extra: u8) -> Self {
        Version {
            major,
            minor,
            patch,
            extra,
        }
    }

    fn new_from_array(value: [u8; 4]) -> Self {
        Self::new(value[0], value[1], value[2], value[3])
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}.{}.{}-{}",
            self.major, self.minor, self.patch, self.extra
        )
    }
}

pub struct SystemStatus {
    system: Version,
    cpu0: Version,
    cpu1: Version,
    touch: u8,
    system_id: u8,
}

impl SystemStatus {
    pub fn new(system: Version, cpu0: Version, cpu1: Version, touch: u8, system_id: u8) -> Self {
        SystemStatus {
            system,
            cpu0,
            cpu1,
            touch,
            system_id,
        }
    }

    pub fn get_system_version(&self) -> &Version {
        &self.system
    }

    pub fn get_cpu0_version(&self) -> &Version {
        &self.cpu0
    }

    pub fn get_cpu1_version(&self) -> &Version {
        &self.cpu1
    }

    pub fn get_touch_version(&self) -> u8 {
        self.touch
    }

    pub fn get_system_id(&self) -> u8 {
        self.system_id
    }
}

impl Brain {
    pub fn new(connection: Box<dyn SerialPort>) -> Self {
        Brain {
            connection: Connection { raw: connection },
        }
    }

    pub fn get_raw_connection(&mut self) -> &mut Connection {
        &mut self.connection
    }

    pub fn read_file_metadata(&mut self, name: &str, vid: Vid) -> Result<FileMetadata> {
        assert!(name.is_ascii());
        assert!(name.len() > 0);

        let mut packet = self
            .connection
            .begin_packet(PacketId::GetFileMetadataByName, 2 + name.len() + 1);
        packet.write_u8(vid.get_id())?;
        packet.write_u8(0)?; // "option"
        packet.write_str(name, 24)?;

        let response = packet.send()?;
        let payload = response.get_data();
        let vid = Vid::from(payload[0]);
        let size = u32::from_le_bytes(payload[1..5].try_into().unwrap());
        let addr = u32::from_le_bytes(payload[5..9].try_into().unwrap());
        let crc = u32::from_le_bytes(payload[9..13].try_into().unwrap()); //fixme
        let file_type = std::str::from_utf8(&payload[15..19])?
            .trim_end_matches('\0')
            .to_string();
        let timestamp = UNIX_EPOCH
            .add(Duration::from_millis(EPOCH_MS_TO_JAN_1_2000))
            .add(Duration::from_millis(
                (u32::from_le_bytes(payload[19..23].try_into().unwrap()) as u64) * 1000_u64, //fixme
            ));
        let name = std::str::from_utf8(&payload[27..])?.trim_end_matches('\0').to_string();
        Ok(FileMetadata {
            vid,
            size,
            addr,
            crc,
            file_type,
            timestamp,
            name,
        })
    }

    pub fn upload_file(
        &mut self,
        target: TransferTarget,
        file_type: FileType,
        vid: Vid,
        file: &[u8],
        remote_name: &str,
        address: u32,
        crc: u32,
        overwrite: bool,
        timestamp: SystemTime,
        linked_file: Option<(&str, Vid)>,
        action: UploadAction,
    ) -> Result<()> {
        let meta = self.initialize_file_transfer(
            TransferDirection::Upload,
            target,
            vid,
            overwrite,
            file.len() as u32,
            address,
            crc,
            0b00_01_00,
            file_type,
            remote_name,
            timestamp,
        )?;
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
        let mut packet = self
            .connection
            .begin_packet(PacketId::SetFileTransferLink, 1);
        packet.write_u8(vid.get_id())?;
        packet.write_u8(0)?;
        packet.write_padded_str(name, 24)?;
        Ok(packet.send()?)
    }

    pub fn initialize_file_transfer(
        &mut self,
        direction: TransferDirection,
        target: TransferTarget,
        vid: Vid,
        overwrite: bool,
        length: u32,
        address: u32,
        crc: u32,
        version: u32,
        file_type: FileType,
        name: &str,
        timestamp: SystemTime,
    ) -> Result<UploadMeta> {
        assert!(name.len() <= 24);
        assert!(name.len() > 0);
        let mut packet = self
            .connection
            .begin_packet(PacketId::FileTransferInitialize, 52);
        packet.write_u8(direction.get_id())?;
        packet.write_u8(target.get_id())?;
        packet.write_u8(vid.get_id())?;
        packet.write_u8(if overwrite { 1 } else { 0 })?;
        packet.write_u32(length)?;
        packet.write_u32(address)?;
        packet.write_u32(crc)?;
        packet.write_str(&file_type.get_name(), 4)?;
        packet.write_u32(
            (&timestamp
                .duration_since(UNIX_EPOCH)?
                .sub(Duration::from_millis(EPOCH_MS_TO_JAN_1_2000))
                .as_millis()
                / 1000) as u32,
        )?;
        packet.write_u32(version)?;
        packet.write_padded_str(name, 24)?;
        let response = packet.send()?;
        let payload = response.get_data();
        Ok(UploadMeta {
            max_packet_size: u16::from_le_bytes(payload[0..2].try_into().unwrap()),
            file_size: u32::from_le_bytes(payload[2..6].try_into().unwrap()),
            crc: u32::from_le_bytes(payload[6..10].try_into().unwrap()),
        })
    }

    pub fn write_file_transfer_part(&mut self, slice: &[u8], address: u32) -> Result<()> {
        let mut packet = self
            .connection
            .begin_packet(PacketId::FileTransferWrite, 4 + slice.len() + 1);
        packet.write_u32(address)?;
        packet.write(slice)?;
        packet.write_u8(0)?;
        Ok(())
    }

    pub fn complete_file_transfer(&mut self, after: UploadAction) -> Result<()> {
        let mut packet = self.connection.begin_packet(PacketId::FileTransferComplete, 1);
        packet.write_u8(after as u8)?;
        packet.send()?;
        Ok(())
    }

    pub fn get_system_version(&mut self) -> Result<SystemVersion> {
        self.connection.raw.write(&[0xc9, 0x36, 0xb8, 0x47, PacketId::GetSystemVersion as u8])?;
        const OFFSET: usize = 4;

        let mut response = [0_u8; OFFSET + 8];
        self.connection.raw.read_exact(&mut response)?;

        info!("Extra sys version byte: {}", response[OFFSET + 7]);

        Ok(SystemVersion {
            major: response[OFFSET + 0],
            minor: response[OFFSET + 1],
            patch: response[OFFSET + 2],
            a: response[OFFSET + 3],
            b: response[OFFSET + 4],
            product: Product::parse(response[OFFSET + 5], response[OFFSET + 6])?,
        })
    }

    pub fn get_kernel_variable(&mut self, variable: KernelVariable) -> Result<String> {
        let mut packet = self
            .connection
            .begin_packet(PacketId::GetKernelVariable, 1);
        packet.write_u8(variable.get_id())?;
        Ok(std::str::from_utf8(packet.send()?.get_data())
            .unwrap()
            .trim_end_matches('\0')
            .to_string())
    }

    pub fn set_kernel_variable(&mut self, variable: KernelVariable, value: &str) -> Result<String> {
        assert!(value.is_ascii());
        assert!(value.len() < variable.get_max_len() as usize);
        let mut packet = self
            .connection
            .begin_packet(PacketId::SetKernelVariable, 1 + value.len() + 1);
        packet.write_u8(variable.get_id())?;
        packet.write_str(value, variable.get_max_len() as u16)?;
        Ok(std::str::from_utf8(packet.send()?.get_data())
            .unwrap()
            .trim_end_matches('\0')
            .to_string())
    }

    pub fn get_system_status(&mut self) -> Result<SystemStatus> {
        let response = self
            .connection
            .begin_packet(PacketId::GetSystemStatus, 0)
            .send()?;
        let data = response.get_data();
        Ok(SystemStatus::new(
            Version::new_from_array((&data[0..4]).try_into()?),
            Version::new_from_array((&data[4..8]).try_into()?),
            Version::new_from_array((&data[8..12]).try_into()?),
            data[12],
            data[13],
        ))
    }

    pub fn get_directory_count(&mut self, vid: Vid, option: u8) -> Result<u16> {
        let mut packet = self
            .connection
            .begin_packet(PacketId::GetDirectoryCount, 2);
        packet.write_u8(vid.get_id())?;
        packet.write_u8(option)?;
        let response = packet.send()?;
        let data = response.get_data();
        Ok(u16::from_le_bytes(data[..2].try_into()?))
    }

    pub fn get_file_metadata_by_index(&mut self, index: u8, option: u8) -> Result<FileMetadata2> {
        let mut packet = self
            .connection
            .begin_packet(PacketId::GetDirectoryCount, 2);
        packet.write_u8(index)?;
        packet.write_u8(option)?;
        let response = packet.send()?;
        let payload = response.get_data();
        let index = payload[0];
        let size = u32::from_le_bytes(payload[1..5].try_into().unwrap());
        let addr = u32::from_le_bytes(payload[5..9].try_into().unwrap());
        let crc = u32::from_le_bytes(payload[9..13].try_into().unwrap());
        let file_type = std::str::from_utf8(&payload[13..17])?
            .trim_end_matches('\0')
            .to_string();
        let timestamp = UNIX_EPOCH
            .add(Duration::from_millis(EPOCH_MS_TO_JAN_1_2000))
            .add(Duration::from_millis(
                (u32::from_le_bytes(payload[17..21].try_into().unwrap()) as u64) * 1000_u64,
            ));
        let version = u32::from_le_bytes(payload[21..25].try_into().unwrap()).to_string();
        let name = std::str::from_utf8(&payload[25..49])?
            .trim_end_matches('\0')
            .to_string();
        Ok(FileMetadata2 {
            index,
            size,
            addr,
            crc,
            file_type,
            timestamp,
            version,
            name,
        })
    }

    pub fn execute_program(&mut self, vid: Vid, options: u8, file: &str) -> Result<()> {
        let mut packet = self
            .connection
            .begin_packet(PacketId::ExecuteProgram, 26);
        packet.write_u8(vid.get_id())?;
        packet.write_u8(options)?;
        packet.write_padded_str(file, 24)?;
        packet.send()?;
        Ok(())
    }

    pub fn delete_file(&mut self, vid: Vid, options: u8, file: &str) -> Result<()> {
        let mut packet = self
            .connection
            .begin_packet(PacketId::DeleteFile, 26);
        packet.write_u8(vid.get_id())?;
        packet.write_u8(options)?;
        packet.write_padded_str(file, 24)?;
        packet.send()?;
        self.complete_file_transfer(UploadAction::Nothing)?;
        Ok(())
    }

    pub fn manage_competition(&mut self, mode: CompetitionStatus) -> Result<()> {
        let mut packet = self
            .connection
            .begin_packet(PacketId::ManageCompetition, 5);
        packet.write_u8(mode as u8)?;
        packet.pad(4)?; // todo: what are these bytes?
        packet.send()?;
        Ok(())
    }
}

impl Connection {
    pub fn new(connection: Box<dyn SerialPort>) -> Self {
        Connection { raw: connection }
    }

    pub fn begin_packet(&mut self, id: PacketId, size: usize) -> Packet {
        assert!(size < u16::MAX as usize);
        Packet::create(self, id, size as usize)
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.raw.flush()
    }
}
