use std::io::{Read, Write};
use std::time::Duration;
use serialport::SerialPort;

const RESPONSE_HEADER: [u8; 2] = [0xAA, 0x55];
const PACKET_HEADER: [u8; 4] = [0xc9, 0x36, 0xb8, 0x47];
const TEN_MILLIS: Duration = Duration::from_millis(10);

#[repr(u8)]
pub enum PacketId {
    GetSystemVersion = 0xA4,
    FileTransferChannel = 0x10,
    FileTransferInitialize = 0x11,
    FileTransferComplete = 0x12,
    FileTransferWrite = 0x13,
    FileTransferRead = 0x14,
    SetFileTransferLink = 0x15,
    GetDirectoryCount = 0x16,
    GetFileMetadataByIndex = 0x17,
    ExecuteProgram = 0x18,
    GetFileMetadataByName = 0x19,
    GetProduct = 0x21,
    GetSystemStatus = 0x22,
    SetProgramFileMetadata = 0x1A,
    DeleteFile = 0x1B,
    GetFileSlot = 0x1C
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
    System = 15,
    Rms = 16,
    Pros = 24,
    Mw = 32
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

    pub fn send_data_packet(&mut self, id: PacketId, data: &[u8]) {
        self.send_packet(id);
        let mut vector = Vec::new();
        vector.reserve(/*5 + */data.len());
        // vector[..4].copy_from_slice(&PACKET_HEADER);
        // vector.push(id.id());
        vector/*[5..]*/.copy_from_slice(&data);
    }

    pub fn send_receive_data_packet(&mut self, id: PacketId, data: &[u8], timeout_millis: u128) -> PacketResponse {
        self.send_packet(id);
        let mut vector = Vec::new();
        vector.reserve(data.len());
        vector.copy_from_slice(&data);
        return self.receive_packet(timeout_millis);
    }

    pub fn send_extended_packet(&mut self, id: PacketId, data: &[u8]) {
        let mut vector = Vec::new();
        vector.reserve(5 + data.len());
        vector[..4].copy_from_slice(&PACKET_HEADER);
        vector.push(id.id());
        vector[5..].copy_from_slice(&data);
    }

    pub fn send_receive_extended_packet(&mut self, id: PacketId, data: &[u8], timeout_millis: u128) -> PacketResponse {
        self.send_extended_packet(id, data);
        self.receive_packet(timeout_millis)
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

    pub fn read_file_metadata(&mut self, name: &str, vid: Vid) {
        assert!(name.is_ascii());
        assert!(name.len() > 0);
        
        let mut data: [u8; 26] = [0, 0, ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '];
        data[0] = vid.into();
        data[1] = 0;
        data[2..].copy_from_slice(name.as_bytes());
        let response = self.send_receive_extended_packet(PacketId::GetFileMetadataByName, &data, 5000);
        let vid = response.payload[0];
        let size = response.payload[1];
        let addr = response.payload[2];
        let crc: u32 = u32::from_le_bytes(addr[3..7]);
        let type_: u32 = u32::from_le_bytes(addr[7..11]);
        let timestamp: u32 = u32::from_le_bytes(addr[11..15]);
        assert!(response.payload == 1);
    }

    pub fn initialize_file_transfer(direction: TransferDirection, target: TransferTarget, vid: Vid, overwrite: bool, length: u32, addr: u32, crc: u32, typ: String, name: String, timestamp: ) {

    }
}