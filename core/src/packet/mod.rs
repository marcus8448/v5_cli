use std::fmt::Debug;

use crate::buffer::{RawRead, RawWrite};

pub mod competition;
pub mod filesystem;
pub mod system;

pub struct PacketBuf {

}

#[async_trait::async_trait]
pub trait Packet<const ID: u8>: Debug {
    type Response;

    fn send_len(&self) -> usize;

    fn is_simple(&self) -> bool {
        false
    }

    fn write_buffer(&self, buffer: &mut dyn RawWrite) -> std::io::Result<()>;

    fn read_response(
        &self,
        buffer: &mut dyn RawRead,
        len: usize,
    ) -> std::io::Result<Self::Response>;
}
