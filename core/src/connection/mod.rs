pub mod bluetooth;
pub mod serial;

pub struct RobotConnection {
    pub system_connection: Box<dyn SerialConnection>,
    pub user_connection: Box<dyn SerialConnection>,
}

pub trait SerialConnection: std::io::Read + std::io::Write {}
