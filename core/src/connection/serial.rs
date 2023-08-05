use std::io::{Read, Write};
use std::time::Duration;

use serialport::{DataBits, FlowControl, Parity, SerialPort, SerialPortType};

use crate::connection::{RobotConnection, SerialConnection};

struct SerialPortConnection {
    serial_port: Box<dyn SerialPort>,
}

impl Read for SerialPortConnection {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.serial_port.read(buf)
    }
}

impl Write for SerialPortConnection {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.serial_port.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.serial_port.flush()
    }
}

impl SerialConnection for SerialPortConnection {}

pub fn find_ports() -> Option<(String, String)> {
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
            return Some((unknown[0].clone(), unknown[1].clone()));
        }
        return None;
    }

    Some((system[0].clone(), user[0].clone()))
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

pub fn connect_to_robot(_port: Option<&String>) -> RobotConnection {
    let (system_port, user_port) = find_ports().expect("Unable to find v5 port!");
    let mut system = serialport::new(system_port, 115200)
        .parity(Parity::None)
        .data_bits(DataBits::Eight)
        .timeout(Duration::from_secs(5))
        .flow_control(FlowControl::None)
        .open()
        .expect("Failed to connect to robot!");

    let mut user = serialport::new(user_port, 115200)
        .parity(Parity::None)
        .data_bits(DataBits::Eight)
        .timeout(Duration::from_secs(5))
        .flow_control(FlowControl::None)
        .open()
        .expect("Failed to connect to robot!");
    system.write_data_terminal_ready(true).unwrap();
    user.write_data_terminal_ready(true).unwrap();

    RobotConnection {
        system_connection: Box::new(SerialPortConnection {
            serial_port: system,
        }),
        user_connection: Box::new(SerialPortConnection { serial_port: user }),
    }
}
