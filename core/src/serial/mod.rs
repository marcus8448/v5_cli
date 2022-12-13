mod brain_connection;
mod robot_connection;

use crc::{Crc, CRC_16_IBM_3740, CRC_32_BZIP2};
use serialport::{DataBits, Parity, SerialPort, SerialPortType};
use crate::serial::brain_connection::BrainConnection;
use crate::serial::robot_connection::RobotConnection;

pub const CRC16: Crc::<u16> = Crc::<u16>::new(&CRC_16_IBM_3740);
pub const CRC32: Crc::<u32> = Crc::<u32>::new(&CRC_32_BZIP2);

pub fn open_serial_connection(port: Option<&String>) -> Box<dyn SerialPort> {
    let mut serial_port = None;
    if let Some(port) = port {
        serial_port = Some(serialport::new(port, 115200).parity(Parity::None).data_bits(DataBits::Eight));
    } else {
        for p in serialport::available_ports().expect("Failed to obtain list of ports!") {
            if let SerialPortType::UsbPort(info) = p.port_type {
                println!("{}: {} {} ({} by {})", p.port_name, info.pid, info.vid, info.product.unwrap_or_default(), info.manufacturer.unwrap_or_default());
                if info.pid == 0x0501 && info.vid == 0x2888 {
                    serial_port = Some(serialport::new(p.port_name, 115200).parity(Parity::None).data_bits(DataBits::Eight));
                    break;
                }
            }
        }
    }

    serial_port.expect("Failed to find robot!").open().expect("Failed to connect to robot!")
}

pub fn open_brain_connection(port: Option<&String>) -> BrainConnection {
    let mut serial_port = None;
    if let Some(port) = port {
        serial_port = Some(serialport::new(port, 115200).parity(Parity::None).data_bits(DataBits::Eight));
    } else {
        for p in serialport::available_ports().expect("Failed to obtain list of ports!") {
            if let SerialPortType::UsbPort(info) = p.port_type {
                println!("{}: {} {} ({} by {})", p.port_name, info.pid, info.vid, info.product.unwrap_or_default(), info.manufacturer.unwrap_or_default());
                if info.pid == 0x0501 && info.vid == 0x2888 {
                    serial_port = Some(serialport::new(p.port_name, 115200).parity(Parity::None).data_bits(DataBits::Eight));
                    break;
                }
            }
        }
    }
    BrainConnection::new(serial_port.expect("Failed to find robot!").open().expect("Failed to connect to robot!"))
}