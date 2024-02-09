use std::convert::TryInto;
use std::io::Write;
use std::str::FromStr;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use btleplug::api::{
    BDAddr, Central, CentralEvent, Characteristic, Manager, Peripheral, ScanFilter, WriteType,
};
use btleplug::platform::PeripheralId;
use futures::{SinkExt, StreamExt};
use log::{debug, warn};
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc::Receiver;
use uuid::Uuid;

use crate::buffer::ReceivingBuffer;
use crate::connection::{CRC16, Nack, RESPONSE_HEADER, RobotConnection};
use crate::error::{CommunicationError, ConnectionError};

const V5_ROBOT_SERVICE: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb13d5);

const CHARACTERISTIC_TX_DATA: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb1306);
// WRITE_WITHOUT_RESPONSE | NOTIFY | INDICATE
const CHARACTERISTIC_RX_DATA: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb13f5); // WRITE_WITHOUT_RESPONSE | WRITE | NOTIFY

const CHARACTERISTIC_TX_USER: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb1316);
// WRITE_WITHOUT_RESPONSE | NOTIFY | INDICATE
const CHARACTERISTIC_RX_USER: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb1326); // WRITE_WITHOUT_RESPONSE | WRITE | NOTIFY

const CHARACTERISTIC_CODE: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb13e5);
// READ | WRITE_WITHOUT_RESPONSE | WRITE
const AUTH_REQUIRED: u32 = 0xdeadface;

pub(crate) struct Characteristics {
    pub(crate) tx_data: Characteristic,
    pub(crate) rx_data: Characteristic,
    pub(crate) tx_user: Characteristic,
    pub(crate) rx_user: Characteristic,
}

pub(crate) async fn connect_to_robot(
    mac_address: Option<String>,
    mut pin: Option<String>,
) -> Result<(btleplug::platform::Peripheral, Characteristics), ConnectionError> {
    let mac_address =
        mac_address.map(|address| BDAddr::from_str(&address).expect("Invalid MAC address"));

    let manager = match btleplug::platform::Manager::new().await {
        Ok(man) => man,
        Err(_) => return Err(ConnectionError::NoBluetoothAdapters),
    };
    let adapters = manager.adapters().await?;

    if adapters.is_empty() {
        return Err(ConnectionError::NoBluetoothAdapters);
    }

    let adapter = &adapters[0];

    let mut events = adapter.events().await?;
    let t = SystemTime::now();
    adapter.start_scan(ScanFilter::default()).await?;

    let mut device: Option<btleplug::platform::Peripheral> = None;

    while let Some(event) = events.next().await {
        match event {
            CentralEvent::DeviceDiscovered(id) => {
                if let Some(peripheral) = find_vex_device(mac_address, adapter, &id).await {
                    device = Some(peripheral);
                }
            }
            CentralEvent::DeviceUpdated(id) => {
                if let Some(peripheral) = find_vex_device(mac_address, adapter, &id).await {
                    device = Some(peripheral);
                }
            }
            _ => {}
        }
        if device.is_some() {
            break;
        }
    }
    debug!(
        "device search took {}ms",
        SystemTime::now()
            .duration_since(t)
            .expect("time ran backwards")
            .as_millis()
    );

    let peripheral = match device {
        None => return Err(ConnectionError::DeviceNotFound),
        Some(dev) => dev,
    };

    if !peripheral.is_connected().await? {
        peripheral.connect().await?;
    } else {
        warn!("bluetooth peripheral already connected?");
    }

    peripheral.discover_services().await?;

    let mut tx_data: Option<Characteristic> = None;
    let mut rx_data: Option<Characteristic> = None;
    let mut tx_user: Option<Characteristic> = None;
    let mut rx_user: Option<Characteristic> = None;
    let mut code: Option<Characteristic> = None;
    for characteristic in peripheral.characteristics() {
        if characteristic.uuid == CHARACTERISTIC_TX_DATA {
            tx_data = Some(characteristic);
        } else if characteristic.uuid == CHARACTERISTIC_RX_DATA {
            rx_data = Some(characteristic);
        } else if characteristic.uuid == CHARACTERISTIC_TX_USER {
            tx_user = Some(characteristic);
        } else if characteristic.uuid == CHARACTERISTIC_RX_USER {
            rx_user = Some(characteristic);
        } else if characteristic.uuid == CHARACTERISTIC_CODE {
            code = Some(characteristic);
        }
    }

    let code = code.expect("char: PIN");
    let tx_data = tx_data.expect("char: tx data");
    let rx_data = rx_data.expect("char: rx data");
    let tx_user = tx_user.expect("char: tx user");
    let rx_user = rx_user.expect("char: rx user");

    let vec = peripheral.read(&code).await?;
    if u32::from_be_bytes(vec[0..4].try_into().unwrap()) == AUTH_REQUIRED {
        if pin.is_none() {
            debug!("Sending PIN display request.");
            peripheral
                .write(&code, &[0xFF, 0xFF, 0xFF, 0xFF], WriteType::WithoutResponse)
                .await?;

            println!("Please enter the PIN shown on the V5 brain");
            let mut str = String::new();
            std::io::stdin()
                .read_line(&mut str)
                .expect("Failed to read stdin");
            pin = Some(str);
        }

        assert!(pin.is_some());
        let pin = pin.as_deref().map_or(Err(()), parse_pin);
        if pin.is_err() {
            return Err(ConnectionError::InvalidPIN);
        }
        let pin = pin.unwrap();

        peripheral
            .write(&code, &pin, WriteType::WithoutResponse)
            .await?;

        let read = peripheral.read(&code).await?;
        if read != pin {
            return Err(ConnectionError::InvalidPIN);
        }
    }

    Ok((
        peripheral,
        Characteristics {
            tx_data,
            rx_data,
            tx_user,
            rx_user,
        },
    ))
}

pub(crate) async fn find_packet_header(port: &mut Receiver<u8>) -> Result<(), CommunicationError> {
    let mut value = 0;
    let mut i = 0;
    let start = SystemTime::now();
    loop {
        if value == RESPONSE_HEADER[i] {
            i += 1;
            if i == RESPONSE_HEADER.len() {
                break;
            }
        } else if i > 0 {
            i = 0;
            continue;
        }

        match port.recv().await {
            Some(v) => value = v,
            None => {
                tokio::time::sleep(Duration::from_millis(2)).await;
                value = 0;
                if SystemTime::now()
                    .duration_since(start)
                    .unwrap_or(Duration::ZERO)
                    > Duration::from_millis(1000)
                {
                    return Err(CommunicationError::TimedOut);
                }
            }
        }
    }
    debug!(
        "found header in {}ms",
        SystemTime::now().duration_since(start).unwrap().as_millis()
    );
    Ok(())
}

pub(crate) struct BluetoothConnection {
    system_tx: Characteristic,
    system_rx: Receiver<u8>,
    user_tx: Characteristic,
    user_rx: Receiver<u8>,
    peripheral: btleplug::platform::Peripheral,
}

impl BluetoothConnection {
    pub(crate) async fn create(
        system_tx: Characteristic,
        system_rx: Characteristic,
        user_tx: Characteristic,
        user_rx: Characteristic,
        peripheral: btleplug::platform::Peripheral,
    ) -> BluetoothConnection {
        let (system_send, system_buf) = tokio::sync::mpsc::channel(1024);
        let (user_send, user_buf) = tokio::sync::mpsc::channel(1024);

        {
            let res = peripheral.subscribe(&system_rx).await;
            let res2 = peripheral.subscribe(&user_rx).await;

            let peripheral = peripheral.clone();

            if cfg!(not(windows)) {
                res.unwrap();
                res2.unwrap();
            }

            tokio::spawn(async move {
                let mut generator = peripheral
                    .notifications()
                    .await
                    .expect("Failed to listen to notifications");

                loop {
                    if let Some(val) = generator.next().await {
                        if val.uuid == system_rx.uuid {
                            system_send.reserve_many(val.value.len()).await.unwrap();
                            for x in val.value {
                                system_send.send(x).await.unwrap();
                            }
                        } else if val.uuid == user_rx.uuid {
                            user_send.reserve_many(val.value.len()).await.unwrap();
                            for x in val.value {
                                user_send.send(x).await.unwrap();
                            }
                        }
                    }
                }
            });
        }

        BluetoothConnection {
            system_tx,
            system_rx: system_buf,
            user_tx,
            user_rx: user_buf,
            peripheral,
        }
    }
}

#[async_trait]
impl RobotConnection for BluetoothConnection {
    fn get_max_packet_size(&self) -> u16 {
        244
    }

    async fn send_packet(&mut self, data: &[u8]) -> Result<ReceivingBuffer, CommunicationError> {
        self.peripheral
            .write(&self.system_tx, data, WriteType::WithoutResponse)
            .await?;

        find_packet_header(&mut self.system_rx).await?;

        let mut payload = Vec::with_capacity(64);
        payload.extend_from_slice(&RESPONSE_HEADER);

        let command = self.system_rx.recv().await.unwrap();
        payload.push(command);

        let mut len = self.system_rx.recv().await.unwrap() as u16;
        payload.push(len as u8);
        if len & 0b1000_0000 != 0 {
            let nxt = self.system_rx.recv().await.unwrap();
            len = u16::from_le_bytes([len as u8 & 0b0111_1111, nxt]);
            payload.push(nxt);
        }

        let start = payload.len();
        payload.resize(start + len as usize, 0_u8);

        for i in 0..len {
            payload[start + i as usize] = self.system_rx.recv().await.unwrap();
        }

        assert_eq!(data[2], command);

        if let Ok(nack) = Nack::try_from(payload[start + 1]) {
            return Err(CommunicationError::NegativeAcknowledgement(nack));
        }

        assert_eq!(CRC16.checksum(&payload), 0);

        Ok(ReceivingBuffer::new(payload.into_boxed_slice(), start + 2))
    }

    async fn write_serial(&mut self, data: &[u8]) -> Result<usize, CommunicationError> {
        self.peripheral
            .write(&self.user_tx, data, WriteType::WithoutResponse)
            .await?;
        Ok(data.len())
    }

    async fn read_serial(&mut self, data: &mut [u8]) -> Result<usize, CommunicationError> {
        for i in 0..data.len() {
            data[i] = self.user_rx.recv().await.unwrap();
        }
        Ok(data.len())
    }

    async fn reset(&mut self) -> Result<(), CommunicationError> {
        for x in self.peripheral.characteristics() {
            if x.uuid == CHARACTERISTIC_CODE {
                let pin = self.peripheral.read(&x).await?;
                self.peripheral
                    .write(&x, &[0xFF, 0xFF, 0xFF, 0xFF], WriteType::WithoutResponse)
                    .await?;
                self.peripheral
                    .write(&x, &pin, WriteType::WithoutResponse)
                    .await?;
                assert_eq!(self.peripheral.read(&x).await?, pin);
                return Ok(());
            }
        }
        Err(CommunicationError::Eof)
    }
}

fn parse_pin(str: &str) -> Result<[u8; 4], ()> {
    let mut chars = str.chars();
    let mut output = [0_u8; 4];
    for i in 0..4 {
        if let Some(next) = chars.next() {
            if next.is_digit(10) {
                output[i] = next.to_digit(10).unwrap() as u8;
                continue;
            }
        }
        return Err(());
    }
    Ok(output)
}

async fn find_vex_device(
    mac_address: Option<BDAddr>,
    adapter: &btleplug::platform::Adapter,
    id: &PeripheralId,
) -> Option<btleplug::platform::Peripheral> {
    if let Ok(peripheral) = adapter.peripheral(id).await {
        if let Some(mac_address) = mac_address {
            if mac_address == peripheral.address() {
                return Some(peripheral);
            }
        } else if let Ok(Some(properties)) = peripheral.properties().await {
            if properties.services.contains(&V5_ROBOT_SERVICE) {
                debug!("Found robot: {}", peripheral.address());
                return Some(peripheral);
            }
        }
    }
    None
}
