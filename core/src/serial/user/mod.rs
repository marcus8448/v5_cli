use crate::error::Result;
use serialport::SerialPort;
use std::io::Write;

pub struct UserProgram {
    connection: Box<dyn SerialPort>,
}

impl UserProgram {
    pub fn new(connection: Box<dyn SerialPort>) -> UserProgram {
        UserProgram { connection }
    }

    pub fn send_raw(&mut self, data: &[u8]) -> Result<usize> {
        self.connection.write(data).map_err(|f| f.into())
    }

    pub fn write_line(&mut self, data: &String) -> Result<usize> {
        self.connection.write(data.as_bytes())?;
        self.connection.write("\n".as_bytes()).map_err(|f| f.into())
    }

    pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        self.connection.read_exact(buf).map_err(|f| f.into())
    }

    pub fn flush(&mut self) -> Result<()> {
        self.connection.flush().map_err(|f| f.into())
    }
}
