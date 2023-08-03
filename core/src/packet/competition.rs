use std::io;
use crate::buffer::{ReadBuffer, WriteBuffer};
use crate::error::{Error, Result};
use crate::packet::Packet;

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum CompetitionState {
    Disabled = 11,
    Autonomous = 10,
    OpControl = 8,
}

impl TryFrom<u8> for CompetitionState {
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

impl Into<u8> for CompetitionState {
    fn into(self) -> u8 {
        self as u8
    }
}

pub struct ManageCompetition {
    state: CompetitionState
}

impl ManageCompetition {
    pub fn new(state: CompetitionState) -> Self {
        Self {
            state
        }
    }
}

impl Packet<0x2E> for ManageCompetition {
    type Response = ();

    fn get_size(&self) -> usize {
        5
    }

    fn write_buffer(&self, buffer: &mut dyn WriteBuffer) -> io::Result<()> {
        buffer.write_u8(self.state.into())?;
        buffer.write_u8(0)?;
        buffer.write_u8(0)?;
        buffer.write_u8(0)?;
        buffer.write_u8(0)?;
        Ok(())
    }

    fn read_response(&self, _: &mut dyn ReadBuffer) -> io::Result<Self::Response> {
        Ok(())
    }
}
