mod plugin;
mod upload;
mod serial;

use std::collections::HashMap;
use std::io;
use std::io::{Read, Write};
use serialport::SerialPort;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

enum State {
    AwaitConnection,
    Connecting,
    Connected
}

// struct PacketRegistry<'a> {
//     index: u16,
//     id_to_name: HashMap<u16, &'static str>,
//     name_to_id: HashMap<&'static str, u16>,
//     id_to_plugin: HashMap<u16, &'a()>
// }
//
// impl<'a> PacketRegistry<'a> {
//     pub fn new() -> PacketRegistry<'a> {
//         PacketRegistry {
//             index: 0,
//             id_to_name: HashMap::new(),
//             name_to_id: HashMap::new(),
//             id_to_plugin: HashMap::new()
//         }
//     }
//
//     pub fn register(&mut self, name: &'static str, plugin: &'a()) {
//         self.id_to_name[&self.index] = name;
//         self.name_to_id[name] = self.index;
//         self.id_to_plugin[&self.index] = plugin;
//         (&mut self).index += 1;
//     }
//
//     pub fn get_name(&self, id: u16) -> &'static str{
//         self.id_to_name[&id]
//     }
//
//     pub fn get_plugin(&self, id: u16) -> &(){
//         self.id_to_plugin[&id]
//     }
//
//     pub fn get_id(&self, name: &'static str) -> u16 {
//         self.name_to_id[name]
//     }
// }
//
// struct RobotConnection<'a> {
//     connection: Box<dyn SerialPort>,
//     registry: PacketRegistry<'a>
// }
//
// impl<'a> RobotConnection<'a> {
//     pub fn new(connection: Box<dyn SerialPort>, registry: PacketRegistry<'a>) -> RobotConnection {
//         RobotConnection {
//             connection,
//             registry
//         }
//     }
//
//     pub fn send_id(&mut self, id: u16) {
//         self.connection.write(&id.to_le_bytes());
//     }
//
//     pub fn send_raw(&mut self, data: &[u8]) {
//         self.connection.write(data);
//     }
//
//     pub fn send_variable(&mut self, name: &'static str, data: &[u8]) {
//         self.connection.write(&self.registry.get_id(name).to_le_bytes());
//         self.connection.write(&data.len().to_le_bytes());
//         self.connection.write(data);
//     }
//
//     pub fn send_fixed(&mut self, name: &'static str, data: &[u8]) {
//         self.connection.write(&self.registry.get_id(name).to_le_bytes());
//         self.connection.write(data);
//     }
//
//     pub async fn read_exact(&mut self, buf: &mut [u8], len: u16) {
//         let len = len as usize;
//         if buf.len() > len {
//             panic!("buf > len");
//         }
//         self.connection.read_exact(&mut buf[0..len]).expect("Failed to read!");
//     }
//
//     pub async fn read_id(&mut self) -> u16 {
//         let mut buf = [0_u8; 2];
//         self.connection.read_exact(&mut buf);
//         return u16::from_le_bytes(buf);
//     }
//
//     pub async fn flush(&mut self) {
//         self.connection.flush();
//     }
// }