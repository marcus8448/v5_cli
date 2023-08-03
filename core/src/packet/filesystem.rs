use std::io;
use std::time::SystemTime;
use std::mem::size_of;
use crate::buffer::{ReadBuffer, WriteBuffer};
use crate::error::{Error, Result};
use crate::packet::system::{Channel, convert_from_vex_timestamp, convert_to_vex_timestamp};
use crate::packet::Packet;

pub struct UploadParameters {
    max_packet_size: u16,
    file_size: u32,
    crc: u32,
}

pub struct FileMetadata {
    pub vid: Vid,
    pub size: u32,
    pub addr: u32,
    pub crc: u32,
    pub file_type: String,
    pub timestamp: SystemTime,
    pub version: u32,
    pub name: String,
}

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum UploadAction {
    Nothing = 0b0,
    Run = 0b01,
    RunScreen = 0b11,
}

impl Into<u8> for UploadAction {
    fn into(self) -> u8 {
        self as u8
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

impl Into<u8> for TransferDirection {
    fn into(self) -> u8 {
        self as u8
    }
}

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum TransferTarget {
    DDR = 0,
    Flash = 1,
    Screen = 2,
}

impl Into<u8> for TransferTarget {
    fn into(self) -> u8 {
        self as u8
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

impl Into<u8> for Vid {
    fn into(self) -> u8 {
        match self {
            Self::User => 1,
            Self::System => 15,
            Self::Rms => 16,
            Self::Pros => 24,
            Self::V5Cli => 27,
            Self::Mw => 32,
            Self::Custom(c) => c,
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

pub struct FileTransferChannel {
    channel: Channel
}

impl FileTransferChannel {
    pub fn new(channel: Channel) -> Self {
        FileTransferChannel {
            channel
        }
    }
}

impl Packet<0x10> for FileTransferChannel {
    type Response = ();

    fn get_size(&self) -> usize {
        2
    }

    fn write_buffer(&self, buffer: &mut dyn WriteBuffer) -> io::Result<()> {
        buffer.write_u8(1)?;
        buffer.write_u8(self.channel.into())
    }

    fn read_response(&self, _: &mut dyn ReadBuffer) -> io::Result<Self::Response> {
        Ok(())
    }
}

struct FileTransferInitialize<'a> {
    direction: TransferDirection,
    target: TransferTarget,
    vid: Vid,
    overwrite: bool,
    length: u32,
    address: u32,
    crc: u32,
    version: u32,
    file_type: FileType,
    name: &'a str,
    timestamp: SystemTime,
}

impl<'a> Packet<0x11> for FileTransferInitialize<'a> {
    type Response = UploadParameters;

    fn get_size(&self) -> usize {
        1 + 1 + 24
    }

    fn write_buffer(&self, buffer: &mut dyn WriteBuffer) -> io::Result<()> {
        buffer.write_u8(self.direction.into())?;
        buffer.write_u8(self.target.into())?;
        buffer.write_u8(self.vid.into())?;
        buffer.write_u8(self.overwrite as u8)?;
        buffer.write_u32(self.length)?;
        buffer.write_u32(self.address)?;
        buffer.write_u32(self.crc)?;
        buffer.write_str(&self.file_type.get_name(), 4)?;
        buffer.write_u32(convert_to_vex_timestamp(self.timestamp))?;
        buffer.write_u32(self.version)?;
        buffer.write_str(self.name, 24)?;
        Ok(())
    }

    fn read_response(&self, buffer: &mut dyn ReadBuffer) -> io::Result<Self::Response> {
        Ok(UploadParameters {
            max_packet_size: buffer.read_u16()?,
            file_size: buffer.read_u32()?,
            crc: buffer.read_u32()?,
        })
    }
}

pub struct FileTransferComplete {
    upload_action: UploadAction
}

impl FileTransferComplete {
    pub fn new(upload_action: UploadAction) -> Self {
        FileTransferComplete {
            upload_action
        }
    }
}

impl Packet<0x12> for FileTransferComplete {
    type Response = ();

    fn get_size(&self) -> usize {
        1
    }

    fn write_buffer(&self, buffer: &mut dyn WriteBuffer) -> io::Result<()> {
        buffer.write_u8(self.upload_action.into())
    }

    fn read_response(&self, _: &mut dyn ReadBuffer) -> io::Result<Self::Response> {
        Ok(())
    }
}

pub struct FileTransferWrite<'a> {
    slice: &'a[u8],
    address: u32
}

impl<'a> FileTransferWrite<'a> {
    pub fn new(slice: &[u8], address: u32) -> Self {
        FileTransferWrite {
            slice,
            address
        }
    }
}

impl<'a> Packet<0x13> for FileTransferWrite<'a> {
    type Response = ();

    fn get_size(&self) -> usize {
        1
    }

    fn write_buffer(&self, buffer: &mut dyn WriteBuffer) -> io::Result<()> {
        buffer.write_u32(self.address)?;
        buffer.write(self.slice)?;
        buffer.write_u8(0)?;
        Ok(())
    }

    fn read_response(&self, _: &mut dyn ReadBuffer) -> io::Result<Self::Response> {
        Ok(())
    }
}

pub struct FileTransferRead {
    len: u16,
    address: u32
}

impl FileTransferRead {
    pub fn new(len: u16, address: u32) -> Self {
        FileTransferRead {
            len,
            address
        }
    }
}

impl Packet<0x14> for FileTransferRead {
    type Response = Box<[u8]>;

    fn get_size(&self) -> usize {
        size_of::<u32>() + size_of::<u16>()
    }

    fn write_buffer(&self, buffer: &mut dyn WriteBuffer) -> io::Result<()> {
        buffer.write_u32(self.address)?;
        buffer.write_u16(self.len)?;
        Ok(())
    }

    fn read_response(&self, buffer: &mut dyn ReadBuffer) -> io::Result<Self::Response> {
        let vec = vec!(0_u8; self.len as usize);
        buffer.read(&mut vec[..])?;
        Ok(vec.into_boxed_slice())
    }
}

pub struct SetFileTransferLink<'a> {
    name: &'a str,
    vid: Vid
}

impl<'a> SetFileTransferLink<'a> {
    pub fn new(name: &str, vid: Vid) -> Self {
        SetFileTransferLink {
            name,
            vid
        }
    }
}

impl<'a> Packet<0x15> for SetFileTransferLink<'a> {
    type Response = ();

    fn get_size(&self) -> usize {
        size_of::<u8>() + 24
    }

    fn write_buffer(&self, buffer: &mut dyn WriteBuffer) -> io::Result<()> {
        buffer.write_u8(self.vid.into())?;
        buffer.write_u8(0)?;
        buffer.write_str(self.name, 24)?;
        Ok(())
    }

    fn read_response(&self, _: &mut dyn ReadBuffer) -> io::Result<Self::Response> {
        Ok(())
    }
}

pub struct GetDirectoryCount {
    vid: Vid,
    option: u8
}

impl GetDirectoryCount {
    pub fn new(vid: Vid, option: u8) -> Self {
        Self { vid, option }
    }
}

impl Packet<0x16> for GetDirectoryCount {
    type Response = u16;

    fn get_size(&self) -> usize {
        size_of::<u8>() + size_of::<u8>()
    }

    fn write_buffer(&self, buffer: &mut dyn WriteBuffer) -> io::Result<()> {
        buffer.write_u8(self.vid.into())?;
        buffer.write_u8(self.option)?;
        Ok(())
    }

    fn read_response(&self, buffer: &mut dyn ReadBuffer) -> io::Result<Self::Response> {
        buffer.read_u16()
    }
}

pub struct GetFileMetadataByIndex {
    index: u8,
    option: u8
}

impl GetFileMetadataByIndex {
    pub fn new(index: u8, option: u8) -> Self {
        Self { index, option }
    }
}

impl Packet<0x17> for GetFileMetadataByIndex {
    type Response = FileMetadata;

    fn get_size(&self) -> usize {
        size_of::<u8>() + size_of::<u8>()
    }

    fn write_buffer(&self, buffer: &mut dyn WriteBuffer) -> io::Result<()> {
        buffer.write_u8(self.index)?;
        buffer.write_u8(self.option)?;
        Ok(())
    }

    fn read_response(&self, buffer: &mut dyn ReadBuffer) -> io::Result<Self::Response> {
        Ok(FileMetadata {
            vid: Vid::from(buffer.read_u8()?),
            size: buffer.read_u32()?,
            addr: buffer.read_u32()?,
            crc: buffer.read_u32()?,
            file_type: buffer.read_str(4)?,
            timestamp: convert_from_vex_timestamp(buffer.read_u32()?),
            version: buffer.read_u32()?,
            name: buffer.read_str(24)?
        })
    }
}

struct GetFileMetadataByName<'a> {
    vid: Vid,
    option: u8,
    file_name: &'a str
}

impl<'a> GetFileMetadataByName<'a> {
    fn new(vid: Vid, option: u8, filename: &str) -> Self {
        GetFileMetadataByName {
            vid,
            option,
            file_name: filename
        }
    }
}

impl<'a> Packet<0x19> for GetFileMetadataByName<'a> {
    type Response = FileMetadata;

    fn get_size(&self) -> usize {
        1 + 1 + 24
    }

    fn write_buffer(&self, buffer: &mut dyn WriteBuffer) -> io::Result<()> {
        buffer.write_u8(self.vid.into())?;
        buffer.write_u8(self.option)?;
        buffer.write_str(self.file_name, 24)?;
        Ok(())
    }

    fn read_response(&self, buffer: &mut dyn ReadBuffer) -> io::Result<Self::Response> {
        Ok(FileMetadata {
            vid: Vid::from(buffer.read_u8()?),
            size: buffer.read_u32()?,
            addr: buffer.read_u32()?,
            crc: buffer.read_u32()?,
            file_type: buffer.read_str(4)?,
            timestamp: convert_from_vex_timestamp(buffer.read_u32()?),
            version: buffer.read_u32()?,
            name: buffer.read_str(24)?
        })
    }
}

pub struct SetProgramFileMetadata<'a> {
    vid: Vid,
    options: u8,
    address: u32,
    file_type: &'a str,
    timestamp: u32,
    version: u32,
    filename: &'a str
}

impl<'a> SetProgramFileMetadata<'a> {
    pub fn new(vid: Vid, options: u8, address: u32, file_type: &'a str, timestamp: u32, version: u32, filename: &'a str) -> Self {
        Self { vid, options, address, file_type, timestamp, version, filename }
    }
}

impl<'a> Packet<0x1A> for SetProgramFileMetadata<'a> {
    type Response = ();

    fn get_size(&self) -> usize {
        0
    }

    fn write_buffer(&self, buffer: &mut dyn WriteBuffer) -> io::Result<()> {
        buffer.write_u8(self.vid.into())?;
        buffer.write_u8(self.options)?;
        buffer.write_u32(self.address)?;
        buffer.write_str(self.file_type, 4)?;
        buffer.write_u32(self.timestamp)?;
        buffer.write_u32(self.version)?;
        buffer.write_str(self.filename, 24)?;

        Ok(())
    }

    fn read_response(&self, _: &mut dyn ReadBuffer) -> io::Result<Self::Response> {
        Ok(())
    }
}

// send FT complete
pub struct DeleteFile<'a> {
    vid: Vid,
    erase_all: bool,
    file_name: &'a str
}

impl<'a> DeleteFile<'a> {
    pub fn new(vid: Vid, erase_all: bool, file_name: &'a str) -> Self {
        Self { vid, erase_all, file_name }
    }
}

impl<'a> Packet<0x1A> for DeleteFile<'a> {
    type Response = ();

    fn get_size(&self) -> usize {
        0
    }

    fn write_buffer(&self, buffer: &mut dyn WriteBuffer) -> io::Result<()> {
        buffer.write_u8(self.vid.into())?;
        buffer.write_u8(if self.erase_all { 0x80 } else { 0 })?;
        buffer.write_str(self.file_name, 24)?;

        Ok(())
    }

    fn read_response(&self, _: &mut dyn ReadBuffer) -> io::Result<Self::Response> {
        Ok(())
    }
}

pub struct GetProgramFileSlot<'a> {
    vid: Vid,
    options: u8,
    file_name: &'a str
}

impl<'a> GetProgramFileSlot<'a> {
    pub fn new(vid: Vid, options: u8, file_name: &'a str) -> Self {
        Self { vid, options, file_name }
    }
}

impl<'a> Packet<0x1C> for GetProgramFileSlot<'a> {
    type Response = u8;

    fn get_size(&self) -> usize {
        0
    }

    fn write_buffer(&self, buffer: &mut dyn WriteBuffer) -> io::Result<()> {
        buffer.write_u8(self.vid.into())?;
        buffer.write_u8(self.options)?;
        buffer.write_str(self.file_name, 24)?;

        Ok(())
    }

    fn read_response(&self, buffer: &mut dyn ReadBuffer) -> io::Result<Self::Response> {
        buffer.read_u8()
    }
}
