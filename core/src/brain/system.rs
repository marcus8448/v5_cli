use core::time::Duration;
use std::fmt::{Display, Formatter};
use std::mem::size_of;
use std::ops::Add;
use std::time::{SystemTime, UNIX_EPOCH};

use bitflags::bitflags;

use crate::brain::Brain;
use crate::brain::filesystem::Vid;
use crate::buffer::{RawRead, RawWrite};
use crate::error::ParseError;

const JAN_01_2000: Duration = Duration::from_secs(946684800);

pub fn convert_to_vex_timestamp(timestamp: SystemTime) -> u32 {
    u32::try_from((timestamp.duration_since(UNIX_EPOCH).unwrap() - JAN_01_2000).as_secs()).unwrap()
}

pub fn convert_from_vex_timestamp(timestamp: u32) -> SystemTime {
    UNIX_EPOCH
        .add(JAN_01_2000)
        .add(Duration::from_secs(timestamp as u64))
}

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum KernelVariable {
    TeamNumber,
    RobotName,
}

impl KernelVariable {
    pub fn get_max_len(&self) -> usize {
        match self {
            Self::TeamNumber => 7,
            Self::RobotName => 16,
        }
    }

    pub(crate) fn get_name(&self) -> &'static str {
        (*self).into()
    }
}

impl From<KernelVariable> for &'static str {
    fn from(val: KernelVariable) -> Self {
        match val {
            KernelVariable::TeamNumber => "teamnumber",
            KernelVariable::RobotName => "robotname",
        }
    }
}

impl TryFrom<&str> for KernelVariable {
    type Error = ParseError;

    fn try_from(id: &str) -> Result<Self, Self::Error> {
        match id.to_lowercase().as_str() {
            "team_number" => Ok(Self::TeamNumber),
            "robot_name" => Ok(Self::RobotName),
            _ => Err(ParseError::InvalidName(id.to_string())),
        }
    }
}

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum Product {
    Brain,
    Controller { has_robot: bool },
}

impl Display for Product {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Product::Brain => write!(f, "brain"),
            Product::Controller { has_robot: true } => write!(f, "controller (connected)"),
            Product::Controller { has_robot: false } => write!(f, "controller (disconnected)"),
        }
    }
}

impl Product {
    fn parse(id: u8, flag: u8) -> std::result::Result<Self, ParseError> {
        match id {
            0x10 => Ok(Self::Brain),
            0x11 => Ok(Self::Controller {
                has_robot: flag & 0b10 == 0b10,
            }),
            _ => Err(ParseError::InvalidId(id as u32)),
        }
    }
}

pub struct SystemStatus {
    pub system: Version,
    pub cpu0: Version,
    pub cpu1: Version,
    pub touch: u8,
    pub system_id: u32,
}

impl SystemStatus {
    pub fn new(system: Version, cpu0: Version, cpu1: Version, touch: u8, system_id: u32) -> Self {
        SystemStatus {
            system,
            cpu0,
            cpu1,
            touch,
            system_id,
        }
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum Channel {
    Pit = 0,
    Download = 1,
}

impl TryFrom<u8> for Channel {
    type Error = ParseError;

    fn try_from(id: u8) -> std::result::Result<Self, Self::Error> {
        match id {
            0 => Ok(Self::Pit),
            1 => Ok(Self::Download),
            id => Err(ParseError::InvalidId(id as u32)),
        }
    }
}

impl From<Channel> for u8 {
    fn from(val: Channel) -> Self {
        val as u8
    }
}

pub struct SystemVersion {
    major: u8,
    minor: u8,
    patch: u8,
    a: u8,
    b: u8,
    product: Product,
}

impl Display for SystemVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}.{}.{}-{}.{} {}",
            self.major, self.minor, self.patch, self.a, self.b, self.product
        )
    }
}

pub struct Version {
    major: u8,
    minor: u8,
    patch: u8,
    extra: u8,
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

bitflags! {
    pub struct ExecutionFlags: u8 {
        const STOP = 0b1000_0000;

        const _ = !0_u8;
    }
}

impl Brain {
    pub async fn get_system_version(&mut self) -> Result<SystemVersion, std::io::Error> {
        let mut response = self.send_simple(0xA4).await?;

        Ok(SystemVersion {
            major: response.read_u8(),
            minor: response.read_u8(),
            patch: response.read_u8(),
            a: response.read_u8(),
            b: response.read_u8(),
            product: Product::parse(response.read_u8(), response.read_u8())?,
        })
    }

    pub async fn get_product(&mut self) -> Result<String, std::io::Error> {
        let mut response = self.send_simple(0x21).await?;

        Ok(response.read_str(response.get_all().len()))
    }

    pub async fn execute_program(&mut self, vid: Vid, flags: ExecutionFlags, filename: &str) -> Result<(), std::io::Error> {
        let mut packet = self.packet(size_of::<u8>() + size_of::<u8>() + 24, 0x18);

        packet.write_u8(vid.into());
        packet.write_u8(flags.bits());
        packet.write_str(filename, 24);

        let _response = packet.send().await?;
        Ok(())
    }

    pub async fn get_system_status(&mut self) -> Result<SystemStatus, std::io::Error> {
        let mut response = self.packet(0, 0x22).send().await?;
        response.skip(1);
        let system = Version {
            major: response.read_u8(),
            minor: response.read_u8(),
            patch: response.read_u8(),
            extra: response.read_u8(),
        };
        let cpu0 = Version {
            major: response.read_u8(),
            minor: response.read_u8(),
            patch: response.read_u8(),
            extra: response.read_u8(),
        };
        let cpu1 = Version {
            major: response.read_u8(),
            minor: response.read_u8(),
            patch: response.read_u8(),
            extra: response.read_u8(),
        };
        response.skip(3);
        let touch = response.read_u8();
        let id = response.read_u32();
        Ok(SystemStatus::new(system, cpu0, cpu1, touch, id))
    }

    pub async fn send_user_communications(&mut self, channel: Channel, payload: &[u8]) -> Result<(), std::io::Error> {
        let mut packet = self.packet(size_of::<u8>() + size_of::<u8>() + payload.len(), 0x27);

        packet.write_u8(channel.into());
        packet.write_u8(0);
        packet.write_raw(payload);

        let _response = packet.send().await?;
        Ok(())
    }

    pub async fn read_user_communications(&mut self, channel: Channel, len: u8) -> Result<Box<[u8]>, std::io::Error> {
        assert!(len > 0);
        let mut packet = self.packet(size_of::<u8>() + size_of::<u8>(), 0x27);

        packet.write_u8(channel.into());
        packet.write_u8(len);

        Ok(packet.send().await?.get_data())
    }

    pub async fn get_kernel_variable(&mut self, variable: KernelVariable) -> Result<String, std::io::Error> {
        let mut packet = self.packet(variable.get_name().len() + 1, 0x2E);
        packet.write_str(variable.get_name(), variable.get_name().len() + 1);

        Ok(packet.send().await?.read_str(variable.get_max_len()))
    }

    pub async fn set_kernel_variable(&mut self, variable: KernelVariable, value: &str) -> Result<(), std::io::Error> {
        assert!(value.len() < variable.get_max_len());
        let mut packet = self.packet(variable.get_name().len() + 1 + value.len() + 1, 0x2F);
        packet.write_str(variable.get_name(), variable.get_name().len() + 1);
        packet.write_str(value, value.len() + 1);

        packet.send().await?;
        Ok(())
    }
}

// CopyScreenData = 0x28
