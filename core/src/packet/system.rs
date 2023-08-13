use core::time::Duration;
use std::fmt::{Display, Formatter};
use std::ops::Add;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::buffer::{RawRead, RawWrite};
use crate::error::ParseError;
use crate::packet::filesystem::Vid;
use crate::packet::Packet;

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

    fn try_from(id: &str) -> std::result::Result<Self, Self::Error> {
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

#[derive(Debug)]
pub struct GetSystemVersion {}

impl GetSystemVersion {
    pub fn new() -> Self {
        Self {}
    }
}

impl Packet<0xA4> for GetSystemVersion {
    type Response = SystemVersion;

    fn send_len(&self) -> usize {
        0
    }

    fn is_simple(&self) -> bool {
        true
    }

    fn write_buffer(&self, _: &mut dyn RawWrite) -> std::io::Result<()> {
        Ok(())
    }

    fn read_response(
        &self,
        buffer: &mut dyn RawRead,
        _len: usize,
    ) -> std::io::Result<Self::Response> {
        Ok(SystemVersion {
            major: buffer.read_u8(),
            minor: buffer.read_u8(),
            patch: buffer.read_u8(),
            a: buffer.read_u8(),
            b: buffer.read_u8(),
            product: Product::parse(buffer.read_u8(), buffer.read_u8())?,
        })
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

#[derive(Debug)]
pub struct ExecuteProgram<'a> {
    vid: Vid,
    options: u8, // 0x0 for start, 0x80 for stop?
    filename: &'a str,
}

impl<'a> ExecuteProgram<'a> {
    pub fn new(vid: Vid, options: u8, filename: &'a str) -> Self {
        ExecuteProgram {
            vid,
            options,
            filename,
        }
    }
}

impl<'a> Packet<0x18> for ExecuteProgram<'a> {
    type Response = ();

    fn send_len(&self) -> usize {
        1 + 1 + 24
    }

    fn write_buffer(&self, buffer: &mut dyn RawWrite) -> std::io::Result<()> {
        buffer.write_u8(self.vid.into());
        buffer.write_u8(self.options);
        buffer.write_str(self.filename, 24);
        Ok(())
    }

    fn read_response(
        &self,
        _buffer: &mut dyn RawRead,
        _len: usize,
    ) -> std::io::Result<Self::Response> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct GetProduct {}

impl GetProduct {
    pub fn new() -> Self {
        GetProduct {}
    }
}

impl Packet<0x21> for GetProduct {
    type Response = Box<[u8]>;

    fn send_len(&self) -> usize {
        0
    }

    fn is_simple(&self) -> bool {
        true
    }

    fn write_buffer(&self, _: &mut dyn RawWrite) -> std::io::Result<()> {
        Ok(())
    }

    fn read_response(
        &self,
        buffer: &mut dyn RawRead,
        _len: usize,
    ) -> std::io::Result<Self::Response> {
        Ok(Box::from(buffer.get_all()))
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

#[derive(Debug)]
pub struct GetSystemStatus {}

impl GetSystemStatus {
    pub fn new() -> Self {
        Self {}
    }
}

impl Packet<0x22> for GetSystemStatus {
    type Response = SystemStatus;

    fn send_len(&self) -> usize {
        0
    }

    fn write_buffer(&self, _: &mut dyn RawWrite) -> std::io::Result<()> {
        Ok(())
    }

    fn read_response(
        &self,
        buffer: &mut dyn RawRead,
        _len: usize,
    ) -> std::io::Result<Self::Response> {
        buffer.skip(1);
        let system = Version {
            major: buffer.read_u8(),
            minor: buffer.read_u8(),
            patch: buffer.read_u8(),
            extra: buffer.read_u8(),
        };
        let cpu0 = Version {
            major: buffer.read_u8(),
            minor: buffer.read_u8(),
            patch: buffer.read_u8(),
            extra: buffer.read_u8(),
        };
        let cpu1 = Version {
            major: buffer.read_u8(),
            minor: buffer.read_u8(),
            patch: buffer.read_u8(),
            extra: buffer.read_u8(),
        };
        buffer.skip(3);
        let touch = buffer.read_u8();
        let id = buffer.read_u32();
        Ok(SystemStatus::new(system, cpu0, cpu1, touch, id))
    }
}

#[derive(Debug)]
pub struct SendUserCommunications<'a> {
    channel: Channel,
    payload: &'a [u8],
}

impl<'a> SendUserCommunications<'a> {
    pub fn new(channel: Channel, payload: &'a [u8]) -> Self {
        assert!(payload.len() <= 224);

        Self { channel, payload }
    }
}

impl<'a> Packet<0x27> for SendUserCommunications<'a> {
    type Response = ();

    fn send_len(&self) -> usize {
        1 + 1 + self.payload.len()
    }

    fn write_buffer(&self, buffer: &mut dyn RawWrite) -> std::io::Result<()> {
        buffer.write_u8(self.channel.into());
        buffer.write_u8(0);
        buffer.write_raw(self.payload);

        Ok(())
    }

    fn read_response(
        &self,
        _buffer: &mut dyn RawRead,
        _len: usize,
    ) -> std::io::Result<Self::Response> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct ReadUserCommunications {
    channel: Channel,
    len: u8,
}

impl ReadUserCommunications {
    pub fn new(channel: Channel, len: u8) -> Self {
        Self { channel, len }
    }
}

impl Packet<0x27> for ReadUserCommunications {
    type Response = Box<[u8]>;

    fn send_len(&self) -> usize {
        2
    }

    fn write_buffer(&self, buffer: &mut dyn RawWrite) -> std::io::Result<()> {
        buffer.write_u8(self.channel.into());
        buffer.write_u8(self.len);

        Ok(())
    }

    fn read_response(
        &self,
        buffer: &mut dyn RawRead,
        _len: usize,
    ) -> std::io::Result<Self::Response> {
        Ok(Box::from(buffer.get_all()))
    }
}

// CopyScreenData = 0x28

#[derive(Debug)]
pub struct GetKernelVariable {
    variable: KernelVariable,
}

impl GetKernelVariable {
    pub fn new(variable: KernelVariable) -> Self {
        Self { variable }
    }
}

impl Packet<0x2E> for GetKernelVariable {
    type Response = String;

    fn send_len(&self) -> usize {
        let name: &str = KernelVariable::into(self.variable);
        name.len() + 1
    }

    fn write_buffer(&self, buffer: &mut dyn RawWrite) -> std::io::Result<()> {
        let name = self.variable.into();
        buffer.write_str(name, name.len() + 1);
        Ok(())
    }

    fn read_response(
        &self,
        buffer: &mut dyn RawRead,
        len: usize,
    ) -> std::io::Result<Self::Response> {
        Ok(buffer.read_str(len))
    }
}

#[derive(Debug)]
pub struct SetKernelVariable<'a> {
    variable: KernelVariable,
    payload: &'a str,
}

impl<'a> SetKernelVariable<'a> {
    pub fn new(variable: KernelVariable, payload: &'a str) -> Self {
        assert!(payload.len() < variable.get_max_len());

        Self { variable, payload }
    }
}

impl<'a> Packet<0x2F> for SetKernelVariable<'a> {
    type Response = ();

    fn send_len(&self) -> usize {
        let name: &str = KernelVariable::into(self.variable);
        name.len() + 1 + self.payload.len() + 1
    }

    fn write_buffer(&self, buffer: &mut dyn RawWrite) -> std::io::Result<()> {
        let name = self.variable.into();
        buffer.write_str(name, name.len() + 1);
        buffer.write_str(self.payload, self.payload.len() + 1);
        Ok(())
    }

    fn read_response(
        &self,
        _buffer: &mut dyn RawRead,
        _len: usize,
    ) -> std::io::Result<Self::Response> {
        Ok(())
    }
}
