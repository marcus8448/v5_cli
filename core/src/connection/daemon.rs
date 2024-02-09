use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::buffer::ReceivingBuffer;
use crate::connection::RobotConnection;
use crate::error::{CommunicationError, ConnectionError};

#[repr(u8)]
pub enum DaemonCommand {
    SendSystem = 0,
    SendUser = 1,
    ClaimExclusive = 2,
    UnclaimExclusive = 3,
    Reset = 4,
}

impl From<DaemonCommand> for u8 {
    fn from(command: DaemonCommand) -> u8 {
        command as u8
    }
}

impl TryFrom<u8> for DaemonCommand {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(DaemonCommand::SendSystem),
            1 => Ok(DaemonCommand::SendUser),
            2 => Ok(DaemonCommand::ClaimExclusive),
            3 => Ok(DaemonCommand::UnclaimExclusive),
            4 => Ok(DaemonCommand::Reset),
            _ => Err(()),
        }
    }
}

pub struct SharedConnection {
    stream: TcpStream,
    max_packet_size: u16,
}

#[async_trait::async_trait]
impl RobotConnection for SharedConnection {
    fn get_max_packet_size(&self) -> u16 {
        self.max_packet_size
    }

    async fn claim_exclusive(&mut self) -> Result<(), CommunicationError> {
        self.stream
            .write_u8(DaemonCommand::ClaimExclusive.into())
            .await?;
        Ok(())
    }

    async fn unclaim_exclusive(&mut self) -> Result<(), CommunicationError> {
        self.stream
            .write_u8(DaemonCommand::UnclaimExclusive.into())
            .await?;
        Ok(())
    }

    async fn send_packet(&mut self, data: &[u8]) -> Result<ReceivingBuffer, CommunicationError> {
        self.stream
            .write_u8(DaemonCommand::SendSystem.into())
            .await?;
        self.stream.write_u16(data.len() as u16).await?;
        self.stream.write_all(data).await?;
        let len = self.stream.read_u16().await?;
        let mut vec1 = vec![0_u8; len as usize];
        vec1.resize(len as usize, 0_u8);
        self.stream.read_exact(&mut vec1).await?;
        return Ok(ReceivingBuffer::new(vec1.into_boxed_slice(), 4 + 2));
    }

    async fn write_serial(&mut self, data: &[u8]) -> Result<usize, CommunicationError> {
        self.stream.write_u8(DaemonCommand::SendUser.into()).await?;
        self.stream.write_u16(data.len() as u16).await?;
        self.stream.write_all(data).await?;
        Ok(data.len())
    }

    async fn read_serial(&mut self, data: &mut [u8]) -> Result<usize, CommunicationError> {
        Ok(self.stream.read(data).await?)
    }

    async fn reset(&mut self) -> Result<(), CommunicationError> {
        self.stream.write_u8(DaemonCommand::Reset.into()).await?;
        Ok(())
    }
}

pub(crate) async fn open_connection(port: u16) -> Result<SharedConnection, ConnectionError> {
    let mut stream =
        TcpStream::connect(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)).await?;
    let max_packet_size = stream.read_u16().await?;

    Ok(SharedConnection {
        stream,
        max_packet_size,
    })
}
