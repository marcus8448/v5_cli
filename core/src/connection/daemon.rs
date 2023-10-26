use std::io::ErrorKind::WouldBlock;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::connection::SerialConnection;
use crate::error::ConnectionError;

pub const HEADER: u32 = 0xc1a04d;

pub struct SharedConnection {
    stream: TcpStream,
}

#[async_trait::async_trait]
impl SerialConnection for SharedConnection {
    async fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        loop {
            self.stream.writable().await?;
            match self.stream.write_all(buf).await {
                Ok(_) => return Ok(()),
                Err(err) if err.kind() == WouldBlock => {
                    #[cfg(windows)]
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
                Err(err) => return Err(err)
            }
        }
    }

    async fn flush(&mut self) -> std::io::Result<()> {
        self.stream.flush().await
    }

    async fn clear(&mut self) -> std::io::Result<()> {
        self.stream.read_to_end(&mut Vec::<u8>::new()).await?;
        Ok(())
    }

    async fn try_read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.stream.readable().await?;

        return match self.stream.try_read(buf) {
            Ok(len) => Ok(len),
            Err(err) => Err(err)
        };
    }

    async fn read_to_end(&mut self, vec: &mut Vec<u8>) -> std::io::Result<usize> {
        self.stream.readable().await?;

        return match self.stream.read_to_end(vec).await {
            Ok(len) => Ok(len),
            Err(err) if err.kind() == WouldBlock => self.read_to_end(vec).await,
            Err(err) => Err(err)
        };
    }

    async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        loop {
            self.stream.readable().await?;

            match self.stream.read_exact(buf).await {
                Ok(_) => return Ok(()),
                Err(err) if err.kind() == WouldBlock => {
                    continue
                }
                Err(err) => {
                    return Err(err);
                }
            };
        }
    }

    async fn try_read_one(&mut self) -> std::io::Result<u8> {
        let mut buf = [0_u8; 1];
        loop {
            self.stream.readable().await?;

            return match self.stream.try_read(&mut buf) {
                Ok(1) => Ok(buf[0]),
                Ok(_) => Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "eof")),
                Err(err) => Err(err)
            };
        }
    }
}

pub(crate) async fn open_connection(port: u16) -> Result<SharedConnection, ConnectionError> {
    Ok(SharedConnection { stream: TcpStream::connect(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)).await.unwrap() })
}
