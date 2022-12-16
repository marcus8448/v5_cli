use std::io::{Read, Write};
use std::ops::Sub;
use std::time::{Duration};
use chrono::{NaiveDateTime, Timelike};
use serialport::SerialPort;

const RESPONSE_HEADER: [u8; 2] = [0xAA, 0x55];
const PACKET_HEADER: [u8; 4] = [0xc9, 0x36, 0xb8, 0x47];
const TEN_MILLIS: Duration = Duration::from_millis(10);
pub const EPOCH_TO_JAN_1_2000: u32 = 946684800;

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


pub enum FileType {
    Bin
}

impl FileType {
    pub fn get_name(&self) -> &'static str {
        match self {
            _Bin => "bin"
        }
    }
    pub fn from_name(name: &str) -> Self {
        match name {
            "bin" => Self::Bin,
            _ => todo!()
        }
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

impl Vid {
    pub fn from_id(id: u8) -> Self {
        match id {
            1 => Self::User,
            15 => Self::System,
            16 => Self::Rms,
            24 => Self::Pros,
            32 => Self::Mw,
            _ => todo!()
        }
    }
}

#[repr(u8)]
pub enum Channel {
    Pit = 0,
    Download = 1
}

pub struct Connection {
    connection: Box<dyn SerialPort>
}

pub struct Brain {
    connection: Connection
}

pub struct FileMetadata {
    vid: Vid,
    size: u32,
    addr: u32,
    crc: u32,
    file_type: String,
    timestamp: NaiveDateTime,
    name: String
}

pub struct UploadMeta {
    max_packet_size: u16,
    file_size: u32,
    crc: u32
}

impl Brain {
    pub fn new(connection: Box<dyn SerialPort>) -> Self {
        Brain {
            connection: Connection { connection }
        }
    }

    pub fn get_raw_connection(&mut self) -> &mut Connection {
        &mut self.connection
    }

    pub fn read_file_metadata(&mut self, name: &str, vid: Vid) -> FileMetadata {
        assert!(name.is_ascii());
        assert!(name.len() > 0);
        
        let mut data: [u8; 26] = [0, 0, ' ' as u8, ' ' as u8, ' ' as u8, ' ' as u8, ' ' as u8, ' ' as u8, ' ' as u8, ' ' as u8, ' ' as u8, ' ' as u8, ' ' as u8, ' ' as u8, ' ' as u8, ' ' as u8, ' ' as u8, ' ' as u8, ' ' as u8, ' ' as u8, ' ' as u8, ' ' as u8, ' ' as u8, ' ' as u8, ' ' as u8, ' ' as u8];
        data[0] = vid as u8;
        // data[1] = 0;
        data[2..].copy_from_slice(name.as_bytes());
        self.connection.send_extended_packet(PacketId::GetFileMetadataByName, &data);
        let response = self.connection.receive_packet(10000);
        let vid = Vid::from_id(response.payload[0]);
        let size = u32::from_le_bytes(data[1..5].try_into().unwrap());
        let addr = u32::from_le_bytes(data[5..9].try_into().unwrap());
        let crc = u32::from_le_bytes(data[9..13].try_into().unwrap());
        let file_type = String::from_utf8_lossy(data[13..17].try_into().unwrap()).to_string();
        let timestamp = chrono::NaiveDateTime::from_timestamp_millis(u32::from_le_bytes(data[17..21].try_into().unwrap()) as i64 * 1000).unwrap() + EPOCH_TO_JAN_1_2000;
        let name = u32::from_le_bytes(data[21..45].try_into().unwrap()).to_string();
        FileMetadata { vid, size, addr, crc, file_type, timestamp, name }
    }

    pub fn upload_file(&mut self, file: &[u8], remote_name: &str, display_name: &str, address: u32, crc: u32, overwrite: bool, timestamp: chrono::NaiveDateTime) {
        let meta = self.initialize_file_transfer(TransferDirection::Upload, TransferTarget::Flash, Vid::System, overwrite, file.len() as u32, address, crc, 0b00_01_00, FileType::Bin, display_name, timestamp);
        assert!(meta.file_size >= file.len() as u32);
        let max_packet_size = meta.max_packet_size / 2;
        let max_packet_size = max_packet_size - (max_packet_size % 4); //padding
        for i in (0..file.len()).step_by(max_packet_size as usize) {
            let end = file.len().min(i + max_packet_size as usize);
            self.write_file_transfer_part(&file[i..end], address + i as u32);
        }

    }

    pub fn write_file_transfer_part(&mut self, slice: &[u8], address: u32) {
        let x = [0_u8; slice.len() + 1];
        x.copy_from_slice(address.to_le_bytes());
        self.connection.send_extended_packet(PacketId::FileTransferWrite, )
        // logger(__name__).debug('Sending ext 0x13 command')
        // if isinstance(payload, str):
        //     payload = payload.encode(encoding='ascii')
        // if len(payload) % 4 != 0:
        //     padded_payload = bytes([*payload, *([0] * (4 - (len(payload) % 4)))])
        // else:
        //     padded_payload = payload
        // tx_fmt = "<I{}s".format(len(padded_payload))
        // tx_payload = struct.pack(tx_fmt, addr, padded_payload)
        // ret = self._txrx_ext_packet(0x13, tx_payload, 0)
        // logger(__name__).debug('Completed ext 0x13 command')

    }

    pub fn initialize_file_transfer(&mut self, direction: TransferDirection, target: TransferTarget, vid: Vid, overwrite: bool, length: u32, address: u32, crc: u32, version: u32, file_type: FileType, name: &str, timestamp: chrono::NaiveDateTime) -> UploadMeta {
        let mut data = [0_u8; 52];
        data[0] = direction as u8;
        data[1] = target as u8;
        data[2] = vid as u8;
        data[3] = overwrite as u8;
        data[4..8].copy_from_slice(&length.to_le_bytes());
        data[8..12].copy_from_slice(&address.to_le_bytes());
        data[12..16].copy_from_slice(&crc.to_le_bytes());
        data[16..20].copy_from_slice(&file_type.get_name().as_bytes());
        data[20..24].copy_from_slice(&((&timestamp.timestamp() - EPOCH_TO_JAN_1_2000 as i64) as u32).to_le_bytes());
        data[24..28].copy_from_slice(&version.to_le_bytes());
        data[28..52].copy_from_slice(name.as_bytes());
        self.connection.send_extended_packet(PacketId::FileTransferInitialize, &data);
        let response = self.connection.receive_packet(5000).payload;
        UploadMeta { max_packet_size: u16::from_le_bytes(response[0..2].try_into().unwrap()), file_size: u32::from_le_bytes(response[2..6].try_into().unwrap()), crc: u32::from_le_bytes(response[6..10].try_into().unwrap()) } 
    }
}

impl Connection {
    pub fn new(connection: Box<dyn SerialPort>) -> Self {
        Connection {
            connection
        }
    }

    pub fn send_packet(&mut self, id: PacketId) {
        let mut packet: [u8; 5] = [0, 0, 0, 0, id.id()];
        packet[..4].copy_from_slice(&PACKET_HEADER);
        self.connection.write(&packet).unwrap();
    }

    pub fn send_data_packet(&mut self, id: PacketId, data: &[u8]) {
        self.send_packet(id);
        let mut vector = Vec::new();
        vector.reserve(/*5 + */data.len());
        // vector[..4].copy_from_slice(&PACKET_HEADER);
        // vector.push(id.id());
        vector/*[5..]*/.copy_from_slice(&data);
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
}