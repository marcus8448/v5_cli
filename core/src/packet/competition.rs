use crate::buffer::{RawRead, RawWrite};
use crate::connection::brain::Brain;
use crate::error::ParseError;
use crate::packet::Packet;

impl Brain {
    
}
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum CompetitionState {
    Disabled = 11,
    Autonomous = 10,
    OpControl = 8,
}

impl TryFrom<u8> for CompetitionState {
    type Error = ParseError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            11 => Ok(Self::Disabled),
            10 => Ok(Self::Autonomous),
            8 => Ok(Self::OpControl),
            _ => Err(ParseError::InvalidId(value as u32)),
        }
    }
}

impl From<CompetitionState> for u8 {
    fn from(val: CompetitionState) -> Self {
        val as u8
    }
}

#[derive(Debug)]
pub struct ManageCompetition {
    state: CompetitionState,
}

impl ManageCompetition {
    pub fn new(state: CompetitionState) -> Self {
        Self { state }
    }
}

impl Packet<0x2E> for ManageCompetition {
    type Response = ();

    fn send_len(&self) -> usize {
        5
    }

    fn write_buffer(&self, buffer: &mut dyn RawWrite) -> std::io::Result<()> {
        buffer.write_u8(self.state.into());
        buffer.write_u8(0);
        buffer.write_u8(0);
        buffer.write_u8(0);
        buffer.write_u8(0);
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
