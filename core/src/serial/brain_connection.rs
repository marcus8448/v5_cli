use std::io::{Read, Write};
use std::time::Duration;
use serialport::SerialPort;

const RESPONSE_HEADER: [u8; 2] = [0xAA, 0x55];
const PACKET_HEADER: [u8; 4] = [0xc9, 0x36, 0xb8, 0x47];
const TEN_MILLIS: Duration = Duration::from_millis(10);

#[repr(u8)]
pub enum PacketId {
    A = 1,
    GetProduct = 0x21,
    B = 12
}

pub struct PacketResponse {
    command: u8,
    payload: Vec<u8>
}

impl PacketId {
    fn id(self) -> u8 {
        return self as u8;
    }
}

#[repr(u8)]
pub enum TransferDirection {
    Upload = 1,
    Download = 2
}

#[repr(u8)]
pub enum TransferTarget {
    DDR = 0,
    Flash = 1,
    Screen = 2
}

#[repr(u8)]
pub enum Vid {
    User = 1,
    System = 15
}

#[repr(u8)]
pub enum Channel {
    Pit = 0,
    Download = 1
}

pub struct BrainConnection {
    connection: Box<dyn SerialPort>
}

impl BrainConnection {
    pub fn new(connection: Box<dyn SerialPort>) -> Self {
        BrainConnection {
            connection
        }
    }

    pub fn send_packet(&mut self, id: PacketId) {
        let mut packet: [u8; 5] = [0, 0, 0, 0, id.id()];
        packet[..4].copy_from_slice(&PACKET_HEADER);
        self.connection.write(&packet).unwrap();
    }

    pub fn send_receive_packet(&mut self, id: PacketId, timeout_millis: u128) -> PacketResponse {
        let mut packet: [u8; 5] = [0, 0, 0, 0, id.id()];
        packet[..4].copy_from_slice(&PACKET_HEADER);
        self.connection.write(&packet).unwrap();
        self.receive_packet(timeout_millis)
    }

    pub fn send_data_packet(&mut self, id: PacketId, data: Box<[u8]>) {
        self.send_packet(id);
        let mut vector = Vec::new();
        vector.reserve(/*5 + */data.len());
        // vector.as_mut_slice()[..4].copy_from_slice(&PACKET_HEADER);
        // vector.push(id.id());
        vector.as_mut_slice()/*[5..]*/.copy_from_slice(&data);
    }

    pub fn send_receive_data_packet(&mut self, id: PacketId, data: Box<[u8]>, timeout_millis: u128) -> PacketResponse {
        self.send_packet(id);
        let mut vector = Vec::new();
        vector.reserve(data.len());
        vector.as_mut_slice().copy_from_slice(&data);
        return self.receive_packet(timeout_millis);
    }

    pub fn send_large_packet(&mut self, id: PacketId) {
        let mut vector = Vec::new();
        vector.reserve(5);
        vector.splice(0..0, PACKET_HEADER);
        vector.push(id.id());
    }

    pub fn receive_packet(&mut self, timeout_millis: u128) -> PacketResponse {
        let mut buf: [u8; 1] = [0];
        let start = std::time::Instant::now();
        self.connection.set_timeout(Duration::from_millis(timeout_millis as u64)).unwrap();
        let mut success = false;
        while start.elapsed().as_millis() < timeout_millis {
            self.connection.read_exact(&mut buf).unwrap();
            if buf[0] != RESPONSE_HEADER[0] {
                continue
            }
            self.connection.read_exact(&mut buf).unwrap();
            if buf[0] != RESPONSE_HEADER[1] {
                continue
            }
            success = true;
            break
        }
        if !success {
            panic!("no header")
        }

        self.connection.read_exact(&mut buf).unwrap();
        let command = buf[0];
        self.connection.read_exact(&mut buf).unwrap();
        let mut len: u16 = buf[0] as u16;
        let mut payload = Vec::new();
        if command == 0x56 && len & 0x80 == 0x80 {
            self.connection.read_exact(&mut buf).unwrap();
            len = ((len & 0x7f) << 8) + buf[0] as u16;
        }
        payload.reserve(len as usize);
        self.connection.read_exact(&mut payload).unwrap();
        PacketResponse {
            command,
            payload
        }
    }

    pub fn receive_packet_raw(&mut self, timeout_millis: u128) -> PacketResponse {
        let mut payload = Vec::new();
        payload.reserve(4);
        let start = std::time::Instant::now();
        self.connection.set_timeout(Duration::from_millis(timeout_millis as u64)).unwrap();
        let mut success = false;
        while start.elapsed().as_millis() < timeout_millis {
            self.connection.read_exact(&mut payload[..1]).unwrap();
            if payload[0] != RESPONSE_HEADER[0] {
                continue
            }
            self.connection.read_exact(&mut payload[..2]).unwrap();
            if payload[1] != RESPONSE_HEADER[1] {
                continue
            }
            success = true;
            break
        }
        if !success {
            panic!("no header")
        }

        self.connection.read_exact(&mut payload[2..4]).unwrap();
        let command = payload[2];
        let len = payload[3];
        payload.reserve(len as usize);
        self.connection.read_exact(&mut payload).unwrap();
        PacketResponse {
            command,
            payload
        }
    }

    pub fn initialize_file_transfer(direction: TransferDirection, target: TransferTarget, vid: Vid, overwrite: bool, length: u32, addr: u32, crc: u32, typ: String, name: String, timestamp: ) {

    }
}