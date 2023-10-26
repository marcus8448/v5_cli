use crate::brain::Brain;
use crate::buffer::RawWrite;
use crate::error::ParseError;

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

impl Brain {
    pub async fn set_competition_state(&mut self, state: CompetitionState, unknown: u32) -> Result<(), std::io::Error> {
        let mut packet = self.packet(5, 0x2E);
        packet.write_u8(state.into());
        packet.write_u32(unknown);
        packet.send().await?;
        Ok(())
    }
}