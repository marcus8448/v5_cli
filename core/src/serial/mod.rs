pub mod brain_connection;
pub mod robot_connection;

use crc::{Crc, CRC_16_IBM_3740, CRC_32_BZIP2};
use serialport::{DataBits, Parity, SerialPort, SerialPortType};
use crate::serial::brain_connection::BrainConnection;
use crate::serial::robot_connection::RobotConnection;

pub const CRC16: Crc::<u16> = Crc::<u16>::new(&CRC_16_IBM_3740);
pub const CRC32: Crc::<u32> = Crc::<u32>::new(&CRC_32_BZIP2);

pub fn find_port() -> Option<String> {
    for p in serialport::available_ports().expect("Failed to obtain list of ports!") {
        if let SerialPortType::UsbPort(info) = p.port_type {
            println!("{}: {} {} ({} by {})", p.port_name, info.pid, info.vid, info.product.unwrap_or_default(), info.manufacturer.unwrap_or_default());
            if info.pid == 0x0501 && info.vid == 0x2888 {
                return Some(p.port_name);
            }
        }
    }
    return None;
}

pub fn open_serial_port(port: Option<String>) -> Box<dyn SerialPort> {
    return serialport::new(port.or(find_port()).unwrap(), 115200).parity(Parity::None).data_bits(DataBits::Eight).open().expect("Failed to connect to robot!");
}

pub fn open_robot_connection(port: Option<String>) -> RobotConnection {
    RobotConnection::new(open_serial_port(port))
}

pub fn open_brain_connection(port: Option<String>) -> BrainConnection {
    BrainConnection::new(open_serial_port(port))
}