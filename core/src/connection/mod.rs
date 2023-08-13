use std::mem::size_of;
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::{Duration, SystemTime};

use async_recursion::async_recursion;
use crc::{Crc, CRC_16_XMODEM};

use crate::buffer::{FixedReadBuffer, RawWrite};
use crate::connection::bluetooth::DualSubscribedBluetoothConnection;
use crate::packet::Packet;

pub mod bluetooth;
pub mod serial;

pub const CRC16: Crc<u16> = Crc::<u16>::new(&CRC_16_XMODEM);

const PACKET_HEADER: &[u8; 4] = &[0xc9, 0x36, 0xb8, 0x47];
const RESPONSE_HEADER: [u8; 2] = [0xAA, 0x55];
static PACKETS_LOST: AtomicU16 = AtomicU16::new(0);

const EXT_PACKET_ID: u8 = 0x56;

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum Nack {
    General = 0xFF,
    InvalidCrc = 0xCE,
    PayloadTooSmall = 0xD0,
    TransferSizeTooLarge = 0xD1,
    CrcError = 0xD2,
    ProgramFileError = 0xD3,
    UninitializedTransfer = 0xD4,
    InvalidInitialization = 0xD5,
    NonPaddedData = 0xD6,
    UnexpectedPacketAddress = 0xD7,
    LengthMismatch = 0xD8,
    NonExistentDirectory = 0xD9,
    FileIndexFull = 0xDA,
    FileExists = 0xDB,
}

impl Nack {
    pub fn maybe_find(id: u8) -> Option<Self> {
        match id {
            0xFF => Some(Self::General),
            0xCE => Some(Self::InvalidCrc),
            0xD0 => Some(Self::PayloadTooSmall),
            0xD1 => Some(Self::TransferSizeTooLarge),
            0xD2 => Some(Self::CrcError),
            0xD3 => Some(Self::ProgramFileError),
            0xD4 => Some(Self::UninitializedTransfer),
            0xD5 => Some(Self::InvalidInitialization),
            0xD6 => Some(Self::NonPaddedData),
            0xD7 => Some(Self::UnexpectedPacketAddress),
            0xD8 => Some(Self::LengthMismatch),
            0xD9 => Some(Self::NonExistentDirectory),
            0xDA => Some(Self::FileIndexFull),
            0xDB => Some(Self::FileExists),
            _ => None,
        }
    }
}

pub enum RobotConnectionOptions {
    Serial {
        port: Option<String>,
    },

    Bluetooth {
        mac_address: Option<String>,
        pin: Option<String>,
    },
}

#[repr(u8)]
pub enum RobotConnectionType {
    User,
    System,
}
pub async fn connect(
    r#type: RobotConnectionType,
    options: RobotConnectionOptions,
) -> Result<Brain, crate::error::ConnectionError> {
    match options {
        RobotConnectionOptions::Serial { port } => {
            let (system, user) = serial::find_ports(port)?;
            Ok(Brain::new(Box::new(
                serial::open_connection(match r#type {
                    RobotConnectionType::User => user,
                    RobotConnectionType::System => system,
                })
                .await?,
            )))
        }
        RobotConnectionOptions::Bluetooth { mac_address, pin } => {
            match bluetooth::connect_to_robot(mac_address, pin).await {
                Ok((peripheral, characteristics)) => match r#type {
                    RobotConnectionType::User => Ok(Brain::new(Box::new(
                        DualSubscribedBluetoothConnection::create(
                            characteristics.tx_user,
                            characteristics.rx_user,
                            peripheral,
                        )
                        .await,
                    ))),
                    RobotConnectionType::System => Ok(Brain::new(Box::new(
                        DualSubscribedBluetoothConnection::create(
                            characteristics.tx_data,
                            characteristics.rx_data,
                            peripheral,
                        )
                        .await,
                    ))),
                },
                Err(err) => Err(err),
            }
        }
    }
}

#[async_trait::async_trait]
pub trait SerialConnection {
    // fn get_max_packet_size(&self) -> usize;

    async fn write(&mut self, buf: &[u8]) -> std::io::Result<()>;
    async fn flush(&mut self) -> std::io::Result<()>;

    async fn clear(&mut self) -> std::io::Result<()>;
    async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<()>;
    async fn try_read_one(&mut self) -> std::io::Result<u8>;
}

pub struct Brain {
    connection: Box<dyn SerialConnection + Send>
}

impl Brain {
    pub fn new(connection: Box<dyn SerialConnection + Send>) -> Self {
        Self { connection }
    }
    
    #[async_recursion(?Send)]
    pub async fn send<const ID: u8, T>(
        &mut self,
        packet: &mut dyn Packet<ID, Response=T>,
    ) -> std::result::Result<T, std::io::Error> {
        dbg!(&packet);
        let len = packet.send_len();
        let mut buffer =
            Vec::with_capacity(4 + 1 + 1 + if len < 0x80 { 1 } else { 2 } + len + size_of::<u16>());
        buffer.write_raw(PACKET_HEADER);

        if packet.is_simple() {
            buffer.write_u8(ID);
        } else {
            buffer.write_u8(EXT_PACKET_ID);
            buffer.write_u8(ID);

            if len < 0x80 {
                println!("normal size {}", len);
                buffer.write_u8(len as u8);
            } else {
                println!("pack size {}", len);
                buffer.write_u8((len >> 8 | 0x80) as u8);
                buffer.write_u8((len & 0xff) as u8);
            }

            let i = buffer.len();

            packet.write_buffer(&mut buffer)?;
            let j = buffer.len();
            println!("Act size: {}", j - i);

            buffer.write_raw(&CRC16.checksum(&buffer).to_be_bytes());
        }

        // println!("sending: {:02X?}", &buffer);
        self.connection.clear().await?;
        self.connection.write(&buffer).await?;
        self.connection.flush().await?;

        let mut value = 0;
        let mut i = 0;
        let time = SystemTime::now();
        loop {
            if value == RESPONSE_HEADER[i] {
                i += 1;
                if i == RESPONSE_HEADER.len() {
                    break;
                }
            } else if i > 0 {
                i = 0;
                continue;
            }

            match self.connection.try_read_one().await {
                Ok(v) => value = v,
                Err(_) => {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                    value = 0;
                    let mut dur = Duration::from_millis(300);
                    if ID == 0x12 {
                        dur = Duration::from_millis(2000);
                    }
                    if SystemTime::now()
                        .duration_since(time)
                        .expect("time ran backwards")
                        > dur
                    {
                        println!(
                            "resending ----------------------------------- {}",
                            PACKETS_LOST.fetch_add(1, Ordering::Relaxed) + 1
                        );
                        return self.send(packet).await;
                    }
                }
            }
        }
        println!(
            "response took {}ms",
            SystemTime::now().duration_since(time).unwrap().as_millis()
        );

        let mut payload = Vec::with_capacity(64);
        payload.extend_from_slice(&RESPONSE_HEADER);

        let mut metadata = [0_u8; 2];

        self.connection.read(&mut metadata).await?;
        let command = metadata[0];
        let mut len: usize = metadata[1] as usize;

        payload.extend_from_slice(&metadata);

        if !packet.is_simple() && len & 0x80 != 0 {
            let val = self.connection.try_read_one().await?;
            len = ((len & 0x7f) << 8) + val as usize;
            payload.push(val);
        }

        let start = payload.len();
        payload.reserve(len);
        payload.resize(start + len, 0_u8);

        self.connection.read(&mut payload[start..]).await?;

        // println!(
        //     "received data ({}): {:02X?}",
        //     len - if packet::is_simple() { 1 } else { 4 },
        //     &payload
        // );

        if packet.is_simple() {
            assert_eq!(command, ID);
            Ok(packet.read_response(&mut FixedReadBuffer::new(&payload[start + 1..]), len - 1)?)
        } else {
            assert_eq!(command, EXT_PACKET_ID);
            assert_eq!(ID, payload[start]);
            assert_eq!(CRC16.checksum(&payload), 0);

            if let Some(nack) = Nack::maybe_find(payload[start + 1]) {
                println!("NACK: {:?} ({})", &nack, payload[start + 1]);
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "NACK"));
            }
            Ok(packet.read_response(
                &mut FixedReadBuffer::new(&payload[start + 2..payload.len() - 2]),
                len - 4,
            )?)
        }
    }
}