use std::fmt::{Debug, Display, Formatter};
use std::mem::size_of;
use std::time::SystemTime;

use bitflags::bitflags;

use crate::brain::Brain;
use crate::brain::system::Channel;
use crate::buffer::ReceivingBuffer;
use crate::error::{CommunicationError, ParseError};

pub struct UploadParameters {
    pub max_packet_size: u16,
    pub file_size: u32,
    pub crc: u32,
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
#[derive(Copy, Clone, Debug)]
pub enum UploadAction {
    Nothing = 0b0,
    Run = 0b01,
    RunScreen = 0b11,
}

impl From<UploadAction> for u8 {
    fn from(val: UploadAction) -> Self {
        val as u8
    }
}

impl Default for UploadAction {
    fn default() -> Self {
        Self::Nothing
    }
}

impl TryFrom<&str> for UploadAction {
    type Error = ParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "nothing" => Ok(Self::Nothing),
            "run" => Ok(Self::Run),
            "screen" => Ok(Self::RunScreen),
            _ => Err(ParseError::InvalidName(value.to_string())),
        }
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum TransferDirection {
    Upload = 1,
    Download = 2,
}

impl From<TransferDirection> for u8 {
    fn from(val: TransferDirection) -> Self {
        val as u8
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum TransferTarget {
    DDR = 0,
    Flash = 1,
    Screen = 2,
}

impl From<TransferTarget> for u8 {
    fn from(val: TransferTarget) -> Self {
        val as u8
    }
}

#[repr(u16)]
#[derive(Copy, Clone, Debug)]
pub enum Vid {
    User = 1,
    System = 15,
    Rms = 16,
    Pros = 24,
    Mw = 32,
    Custom(u8),
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
                Self::Mw => "mw",
                Self::Custom(_) => "custom",
            },
            u8::from(*self)
        )
    }
}

impl From<Vid> for u8 {
    fn from(value: Vid) -> Self {
        match value {
            Vid::User => 1,
            Vid::System => 15,
            Vid::Rms => 16,
            Vid::Pros => 24,
            Vid::Mw => 32,
            Vid::Custom(c) => c,
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
            32 => Self::Mw,
            i => Self::Custom(i),
        }
    }
}

#[derive(Debug)]
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
    type Error = ParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "bin" => Ok(Self::Bin),
            "ini" => Ok(Self::Ini),
            _ => Err(ParseError::InvalidName(value.to_string())),
        }
    }
}

pub struct FileTransfer<'a> {
    brain: &'a mut Brain,
    pub parameters: UploadParameters,
}

bitflags! {
    pub struct FileFlags: u8 {
        const _ = !0_u8;
    }
}

bitflags! {
    pub struct DeleteFlags: u8 {
        const ERASE_ALL = 0b1000_0000;
        const _ = !0_u8;
    }
}

impl Brain {
    pub async fn get_directory_count(
        &mut self,
        vid: Vid,
        option: FileFlags,
    ) -> Result<u16, CommunicationError> {
        let mut packet = self.packet(size_of::<u8>() + size_of::<u8>(), 0x16);

        packet.write_u8(vid.into());
        packet.write_u8(option.bits());

        let mut response = packet.send().await?;
        Ok(response.read_u16())
    }

    pub async fn get_file_metadata_by_index(
        &mut self,
        index: u8,
        flags: FileFlags,
    ) -> Result<FileMetadata, CommunicationError> {
        let mut packet = self.packet(size_of::<u8>() + size_of::<u8>(), 0x17);

        packet.write_u8(index);
        packet.write_u8(flags.bits());

        Ok(parse_metadata(packet.send().await?))
    }

    pub async fn get_file_metadata_by_name(
        &mut self,
        vid: Vid,
        flags: FileFlags,
        filename: &str,
    ) -> Result<FileMetadata, CommunicationError> {
        let mut packet = self.packet(size_of::<u8>() + size_of::<u8>() + 24, 0x19);

        packet.write_u8(vid.into());
        packet.write_u8(flags.bits());
        packet.write_str(filename, 24);

        Ok(parse_metadata(packet.send().await?))
    }

    pub async fn set_file_metadata(
        &mut self,
        vid: Vid,
        filename: &str,
        flags: FileFlags,
        address: u32,
        file_type: &str,
        timestamp: u32,
        version: u32,
    ) -> Result<(), CommunicationError> {
        let mut packet = self.packet(
            size_of::<u8>()
                + size_of::<u8>()
                + size_of::<u32>()
                + 4
                + size_of::<u32>()
                + size_of::<u32>()
                + 24,
            0x1A,
        );

        packet.write_u8(vid.into());
        packet.write_u8(flags.bits());
        packet.write_u32(address);
        packet.write_str(file_type, 4);
        packet.write_u32(timestamp);
        packet.write_u32(version);
        packet.write_str(filename, 24);

        let _response = packet.send().await?;
        Ok(())
    }

    // send FT complete
    pub async fn delete_file(
        &mut self,
        vid: Vid,
        flags: DeleteFlags,
        filename: &str,
    ) -> Result<(), CommunicationError> {
        let mut packet = self.packet(size_of::<u8>() + size_of::<u8>() + 24, 0x1B);

        packet.write_u8(vid.into());
        packet.write_u8(flags.bits());
        packet.write_str(filename, 24);

        let _response = packet.send().await?;
        Ok(())
    }

    pub async fn get_program_file_slot(
        &mut self,
        vid: Vid,
        flags: FileFlags,
        filename: &str,
    ) -> Result<u8, CommunicationError> {
        let mut packet = self.packet(size_of::<u8>() + size_of::<u8>() + 24, 0x1C);

        packet.write_u8(vid.into());
        packet.write_u8(flags.bits());
        packet.write_str(filename, 24);

        let mut response = packet.send().await?;
        Ok(response.read_u8())
    }

    pub async fn file_transfer_initialize<'a>(
        &'a mut self,
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
    ) -> Result<FileTransfer<'a>, CommunicationError> {
        let mut packet = self.packet(
            size_of::<u8>() * 4 + size_of::<u32>() * 3 + 4 + size_of::<u32>() * 2 + 24,
            0x11,
        );

        packet.write_u8(direction.into());
        packet.write_u8(target.into());
        packet.write_u8(vid.into());
        packet.write_u8(overwrite as u8);
        packet.write_u32(length);
        packet.write_u32(address);
        packet.write_u32(crc);
        packet.write_str(file_type.get_name(), 4);
        packet.write_u32(crate::brain::system::convert_to_vex_timestamp(timestamp));
        packet.write_u32(version);
        packet.write_str(name, 24);

        let mut response: ReceivingBuffer = packet.send().await?;
        Ok(FileTransfer {
            brain: self,
            parameters: UploadParameters {
                max_packet_size: response.read_u16(),
                file_size: response.read_u32(),
                crc: response.read_u32(),
            },
        })
    }
}

impl<'a> FileTransfer<'a> {
    pub async fn set_channel(&mut self, channel: Channel) -> Result<(), CommunicationError> {
        let mut packet = self.brain.packet(5, 0x10);
        packet.write_u8(1);
        packet.write_u8(channel.into());
        let _response = packet.send().await?;
        Ok(())
    }

    pub async fn set_link(&mut self, name: &str, vid: Vid) -> Result<(), CommunicationError> {
        let mut packet = self.brain.packet(1 + 1 + 24, 0x15);

        packet.write_u8(vid.into());
        packet.write_u8(0);
        packet.write_str(name, 24);

        let _response = packet.send().await?;
        Ok(())
    }

    pub async fn write(&mut self, slice: &[u8], address: u32) -> Result<(), CommunicationError> {
        let mut packet = self.brain.packet(
            size_of::<u32>()
                + slice.len()
                + if slice.len() % 4 != 0 {
                    4 - (slice.len() % 4)
                } else {
                    0
                },
            0x13,
        );

        packet.write_u32(address);
        packet.write_raw(slice);
        if slice.len() % 4 != 0 {
            packet.pad(4 - slice.len() % 4);
        }
        let _response = packet.send().await?;
        Ok(())
    }

    pub async fn read(&mut self, len: u16, address: u32) -> Result<Box<[u8]>, CommunicationError> {
        let mut packet = self.brain.packet(size_of::<u32>() + size_of::<u16>(), 0x14);

        packet.write_u32(address);
        packet.write_u16(len);

        let mut response = packet.send().await?;

        let mut val = vec![0_u8; len as usize].into_boxed_slice();
        response.read_raw(&mut val[..]);
        Ok(val)
    }

    pub async fn complete(self, upload_action: UploadAction) -> Result<(), CommunicationError> {
        let mut packet = self.brain.packet(1, 0x12);
        packet.write_u8(upload_action.into());
        let _response = packet.send().await?;
        Ok(())
    }
}

fn parse_metadata(mut response: ReceivingBuffer) -> FileMetadata {
    FileMetadata {
        vid: Vid::from(response.read_u8()),
        size: response.read_u32(),
        addr: response.read_u32(),
        crc: response.read_u32(),
        file_type: response.read_str(4),
        timestamp: crate::brain::system::convert_from_vex_timestamp(response.read_u32()),
        version: response.read_u32(),
        name: response.read_str(24),
    }
}
