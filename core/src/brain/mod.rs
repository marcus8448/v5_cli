use std::ops::{Deref, DerefMut};

use crate::connection::{Packet, RobotConnection};

pub mod competition;
pub mod filesystem;
pub mod system;

pub struct Brain {
    pub connection: Box<dyn RobotConnection + Send>,
}

impl Deref for Brain {
    type Target = Box<dyn RobotConnection + Send>;

    fn deref(&self) -> &Self::Target {
        &self.connection
    }
}

impl DerefMut for Brain {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.connection
    }
}

impl Brain {
    pub fn new(connection: Box<dyn RobotConnection + Send>) -> Self {
        Self { connection }
    }

    fn packet(&mut self, content_len: usize, packet_id: u8) -> Packet {
        Packet::new(packet_id, content_len, self)
    }
}
