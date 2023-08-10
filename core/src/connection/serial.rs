use std::io::{Read, Write};
use std::time::Duration;

use serialport::{DataBits, FlowControl, Parity, SerialPort, SerialPortType};

use crate::connection::SerialConnection;
use crate::error::ConnectionError;

pub struct SerialPortConnection {
    serial_port: Box<dyn SerialPort>,
}

#[async_trait::async_trait]
impl SerialConnection for SerialPortConnection { //fixme async
    async fn write(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.serial_port.write_all(buf)
    }

    async fn flush(&mut self) -> std::io::Result<()> {
        self.serial_port.flush()
    }

    async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        self.serial_port.read_exact(buf)
    }
}

pub(crate) fn find_ports(_port: Option<String>) -> Result<(String, String), ConnectionError> {
    let mut system = Vec::new();
    let mut user = Vec::new();
    let mut controller = Vec::new();

    let mut unknown = Vec::new();

    for port in serialport::available_ports().expect("Failed to obtain list of ports!") {
        if let SerialPortType::UsbPort(info) = &port.port_type {
            if info.pid == 0x0501 && info.vid == 0x2888 {
                if let Some(product) = &info.product {
                    let product = product.to_lowercase();
                    if product.contains("user") {
                        &mut user
                    } else if product.contains("system") || product.contains("communications") {
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

pub fn print_out_ports() {
    for p in serialport::available_ports().expect("Failed to obtain list of ports!") {
        if let SerialPortType::UsbPort(info) = p.port_type {
            if info.pid == 0x0501 && info.vid == 0x2888 {
                println!(
                    "{}: {} {} ({} by {})",
                    p.port_name,
                    info.pid,
                    info.vid,
                    info.product.unwrap_or_default(),
                    info.manufacturer.unwrap_or_default()
                );
            }
        } else {
            println!("{}: {:?}", p.port_name, p.port_type);
        }
    }
}

pub(crate) fn open_connection(port: String) -> Result<SerialPortConnection, ConnectionError> {
    let mut user = serialport::new(port, 115200)
        .parity(Parity::None)
        .data_bits(DataBits::Eight)
        .timeout(Duration::from_secs(5))
        .flow_control(FlowControl::None)
        .open()
        .expect("Failed to connect to robot!");

    user.write_data_terminal_ready(true).unwrap();
    return Ok(SerialPortConnection { serial_port: user })
}
