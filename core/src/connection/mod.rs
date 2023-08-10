use crate::connection::bluetooth::DualSubscribedBluetoothConnection;

pub mod bluetooth;
pub mod serial;

pub enum RobotConnectionOptions {
    Serial {
        port: Option<String>
    },

    Bluetooth {
        mac_address: Option<String>,
        pin: Option<String>
    }
}

#[repr(u8)]
pub enum RobotConnectionType {
    User,
    System
}
pub async fn connect(r#type: RobotConnectionType, options: RobotConnectionOptions) -> Result<Box<dyn SerialConnection + Send>, crate::error::ConnectionError>{
    match options {
        RobotConnectionOptions::Serial { port } => {
            let (system, user) = serial::find_ports(port)?;
            Ok(Box::new(serial::open_connection(
                match r#type {
                    RobotConnectionType::User => user,
                    RobotConnectionType::System => system
                }
            )?))
        }
        RobotConnectionOptions::Bluetooth { mac_address, pin } => {
            match bluetooth::connect_to_robot(mac_address, pin).await {
                Ok((peripheral, characteristics)) => {
                    match r#type {
                        RobotConnectionType::User => {
                            Ok(Box::new(DualSubscribedBluetoothConnection::create(characteristics.tx_user, characteristics.rx_user, peripheral).await))
                        }
                        RobotConnectionType::System => {
                            Ok(Box::new(DualSubscribedBluetoothConnection::create(characteristics.tx_data, characteristics.rx_data, peripheral).await))
                        }
                    }
                }
                Err(err) => Err(err)
            }
        }
    }
}

#[async_trait::async_trait]
pub trait SerialConnection {
    async fn write(&mut self, buf: &[u8]) -> std::io::Result<()>;
    async fn flush(&mut self) -> std::io::Result<()>;

    async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<()>;
}
