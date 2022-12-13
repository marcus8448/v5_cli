use serialport::SerialPort;

pub struct RobotConnection {
    connection: Box<dyn SerialPort>
}

impl RobotConnection {
    pub fn new(connection: Box<dyn SerialPort>) -> RobotConnection {
        RobotConnection {
            connection
        }
    }

    pub fn send_u8(&mut self, id: u8) -> std::io::Result<usize> {
        self.connection.write(&id.to_le_bytes())
    }
    pub fn send_u16(&mut self, id: u16) -> std::io::Result<usize> {
        self.connection.write(&id.to_le_bytes())
    }

    pub fn send_buf(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.connection.write(data)
    }

    pub fn read_exact(&mut self, buf: &mut [u8], len: u16) {
        let len = len as usize;
        if buf.len() > len {
            panic!("buf > len");
        }
        self.connection.read_exact(&mut buf[0..len]).expect("Failed to read!");
    }

    pub fn read_id(&mut self) -> std::io::Result<u16> {
        let mut buf = [0_u8; 2];
        self.connection.read_exact(&mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        self.connection.flush()
    }
}
