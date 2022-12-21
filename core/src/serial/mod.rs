pub mod system;
pub mod program;

use crc::{Crc, CRC_16_IBM_3740, CRC_32_BZIP2};
use log::info;
use serialport::{DataBits, Parity, SerialPort, SerialPortType};

pub const CRC16: Crc::<u16> = Crc::<u16>::new(&CRC_16_IBM_3740);
pub const CRC32: Crc::<u32> = Crc::<u32>::new(&CRC_32_BZIP2);

pub enum PortType {
    User,
    System,
    Controller
}

impl PortType {
    pub fn match_name(&self, name: &str) -> bool {
        match self {
            PortType::User => name.contains("User"),
            PortType::System => name.contains("System") || name.contains("Communications"),
            PortType::Controller => name.contains("Controller"),
        }
    }
}

pub fn find_port(port_type: PortType) -> Option<String> {
    for p in serialport::available_ports().expect("Failed to obtain list of ports!") {
        if let SerialPortType::UsbPort(info) = p.port_type {
            if info.pid == 0x0501 && info.vid == 0x2888 && port_type.match_name(&p.port_name) {
                return Some(p.port_name);
            }
        }
    }
    return None;
}

pub fn print_out_ports(port_type: Option<PortType>) {
    for p in serialport::available_ports().expect("Failed to obtain list of ports!") {
        if let SerialPortType::UsbPort(info) = p.port_type {
            if info.pid == 0x0501 && info.vid == 0x2888 && (port_type.is_none() || port_type.as_ref().unwrap().match_name(&p.port_name)) {
                info!("{}: {} {} ({} by {})", p.port_name, info.pid, info.vid, info.product.unwrap_or_default(), info.manufacturer.unwrap_or_default());
            }
        }
    }
}

pub fn open_serial_port(port: Option<String>, port_type: PortType) -> Box<dyn SerialPort> {
    return serialport::new(port.or(find_port(port_type)).expect("Unable to find V5 port!"), 115200).parity(Parity::None).data_bits(DataBits::Eight).open().expect("Failed to connect to robot!");
}

pub fn open_robot_connection(port: Option<String>) -> program::Connection {
    program::Connection::new(open_serial_port(port, PortType::User))
}

pub fn connect_to_brain(port: Option<String>) -> system::Brain {
    system::Brain::new(open_serial_port(port, PortType::System))
}
