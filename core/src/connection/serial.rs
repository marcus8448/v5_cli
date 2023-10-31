use std::io::{Read, Write};
use std::io::ErrorKind::WouldBlock;
use std::time::Duration;

use tokio_serial::{
    ClearBuffer, DataBits, FlowControl, Parity, SerialPort, SerialPortBuilderExt, SerialPortType,
    SerialStream,
};

use crate::connection::SerialConnection;
use crate::error::ConnectionError;

pub struct SerialPortConnection {
    serial_port: SerialStream,
}

#[async_trait::async_trait]
impl SerialConnection for SerialPortConnection {
    async fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        loop {
            #[cfg(not(windows))]
            self.serial_port.writable().await?;
            match self.serial_port.write_all(buf) {
                Ok(_) => return Ok(()),
                Err(err) if err.kind() == WouldBlock => {
                    #[cfg(windows)]
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
                Err(err) => return Err(err),
            }
        }
    }

    async fn flush(&mut self) -> std::io::Result<()> {
        self.serial_port.flush()
    }

    async fn clear(&mut self) -> std::io::Result<()> {
        self.serial_port.clear(ClearBuffer::All)?;
        Ok(())
    }

    async fn try_read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.serial_port.try_read(buf)
    }

    async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        loop {
            #[cfg(not(windows))]
            self.serial_port.readable().await?;
            match self.serial_port.read_exact(buf) {
                Ok(_) => return Ok(()),
                Err(err) if err.kind() == WouldBlock => {
                    #[cfg(windows)]
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
                Err(err) => return Err(err),
            };
        }
    }

    async fn try_read_one(&mut self) -> std::io::Result<u8> {
        let mut buf = [0_u8; 1];
        loop {
            #[cfg(not(windows))]
            self.serial_port.readable().await?;

            match self.serial_port.try_read(&mut buf) {
                Ok(1) => return Ok(buf[0]),
                Ok(_) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "eof",
                    ))
                }
                Err(err) if err.kind() == WouldBlock => {
                    #[cfg(windows)]
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
                Err(err) => return Err(err),
            };
        }
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

pub(crate) async fn open_connection(port: String) -> Result<SerialPortConnection, ConnectionError> {
    let mut serial_port = tokio_serial::new(port, 115200)
        .parity(Parity::None)
        .data_bits(DataBits::Eight)
        .timeout(Duration::from_secs(5))
        .flow_control(FlowControl::None)
        .open_native_async()
        .expect("Failed to connect to robot!");

    serial_port.write_data_terminal_ready(true).expect("dtr");
    #[cfg(unix)]
    serial_port.set_exclusive(false).expect("set not exclusive");
    Ok(SerialPortConnection { serial_port })
}
