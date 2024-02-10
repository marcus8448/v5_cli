use std::io::ErrorKind::WouldBlock;
use std::time::{Duration, SystemTime};

use log::debug;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio_serial::{
    DataBits, FlowControl, Parity, SerialPort, SerialPortBuilderExt, SerialPortType, SerialStream,
};

use crate::buffer::ReceivingBuffer;
use crate::connection::{CRC16, Nack, RESPONSE_HEADER, RobotConnection};
use crate::error::{CommunicationError, ConnectionError};

pub struct SerialPortConnection {
    system_port: SerialStream,
    communications_port: Option<SerialStream>,
}

pub(crate) async fn find_packet_header<T: AsyncRead + AsyncReadExt + Unpin>(
    port: &mut T,
) -> Result<(), CommunicationError> {
    let mut value = 0;
    let mut i = 0;
    let start = SystemTime::now();
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

        match port.read_u8().await {
            Ok(v) => value = v,
            Err(err) if err.kind() == WouldBlock => {
                tokio::time::sleep(Duration::from_millis(2)).await;
                value = 0;
                if SystemTime::now()
                    .duration_since(start)
                    .unwrap_or(Duration::ZERO)
                    > Duration::from_millis(1000)
                {
                    return Err(CommunicationError::TimedOut);
                }
            }
            Err(err) => return Err(err.into()),
        }
    }
    debug!(
        "found header in {}ms",
        SystemTime::now().duration_since(start).unwrap().as_millis()
    );
    Ok(())
}

#[async_trait::async_trait]
impl RobotConnection for SerialPortConnection {
    fn get_max_packet_size(&self) -> u16 {
        0b0111_1111_1111_1111
    }

    async fn send_packet(&mut self, data: &[u8]) -> Result<ReceivingBuffer, CommunicationError> {
        self.system_port.write_all(&data).await?;

        find_packet_header(&mut self.system_port).await?;

        let mut payload = Vec::with_capacity(64);
        payload.extend_from_slice(&RESPONSE_HEADER);

        let command = self.system_port.read_u8().await?;
        payload.push(command);

        let mut len = self.system_port.read_u8().await? as u16;
        payload.push(len as u8);
        if len & 0b1000_0000 != 0 {
            let nxt = self.system_port.read_u8().await?;
            len = u16::from_le_bytes([len as u8 & 0b0111_1111, nxt]);
            payload.push(nxt);
        }

        let start = payload.len();
        payload.resize(start + len as usize, 255_u8);

        self.system_port.read_exact(&mut payload[start..]).await?;

        if let Ok(nack) = Nack::try_from(payload[start + 1]) {
            return Err(CommunicationError::NegativeAcknowledgement(nack));
        }

        assert_eq!(
            data[4], command,
            "response: {:?}, data: {:?}",
            payload, data
        );
        assert_eq!(CRC16.checksum(&payload), 0, "response: {:?}", payload);

        Ok(ReceivingBuffer::new(payload.into_boxed_slice(), start + 2))
    }

    async fn write_serial(&mut self, data: &[u8]) -> Result<usize, CommunicationError> {
        if let Some(port) = self.communications_port.as_mut() {
            Ok(port.write(data).await?)
        } else {
            todo!()
        }
    }

    async fn read_serial(&mut self, data: &mut [u8]) -> Result<usize, CommunicationError> {
        if let Some(port) = self.communications_port.as_mut() {
            Ok(port.read(data).await?)
        } else {
            todo!()
        }
    }

    async fn reset(&mut self) -> Result<(), CommunicationError> {
        todo!()
    }

    async fn shutdown(&mut self) -> Result<(), CommunicationError> {
        self.system_port.shutdown().await?;
        if let Some(stream) = self.communications_port.as_mut() {
            stream.shutdown().await?;
        }
        Ok(())
    }
}

pub(crate) fn find_ports(_port: Option<String>) -> Result<(String, String), ConnectionError> {
    let mut system = Vec::new();
    let mut user = Vec::new();
    let mut controller = Vec::new();

    let mut unknown = Vec::new();

    let ports = tokio_serial::available_ports();
    match ports {
        Ok(ports) => {
            for port in ports {
                if let SerialPortType::UsbPort(info) = &port.port_type {
                    if info.pid == 0x0501 && info.vid == 0x2888 {
                        if let Some(product) = &info.product {
                            let product = product.to_lowercase();
                            if product.contains("user") {
                                &mut user
                            } else if product.contains("system")
                                || product.contains("communications")
                            {
                                &mut system
                            } else if product.contains("controller") {
                                &mut controller
                            } else {
                                &mut unknown
                            }
                            .push(port.port_name.clone())
                        }
                    }
                }
            }

            if system.is_empty() || user.is_empty() {
                if unknown.len() >= 2 {
                    return Ok((unknown[0].clone(), unknown[1].clone()));
                }
                return Err(ConnectionError::DeviceNotFound);
            }

            Ok((system[0].clone(), user[0].clone()))
        }
        Err(err) => Err(ConnectionError::SerialPortError(err)),
    }
}

pub(crate) async fn open_connection(
    system: String,
    user: String,
) -> Result<SerialPortConnection, ConnectionError> {
    let system_port = tokio_serial::new(system, 115200)
        .parity(Parity::None)
        .data_bits(DataBits::Eight)
        .timeout(Duration::from_secs(5))
        .flow_control(FlowControl::None)
        .open_native_async()
        .expect("Failed to connect to robot!");

    let user_port = tokio_serial::new(user, 115200)
        .parity(Parity::None)
        .data_bits(DataBits::Eight)
        .timeout(Duration::from_secs(5))
        .flow_control(FlowControl::None)
        .open_native_async()
        .expect("Failed to connect to robot!");

    Ok(SerialPortConnection {
        system_port,
        communications_port: Some(user_port),
    })
}
