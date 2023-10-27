use std::cell::Cell;
use std::convert::TryInto;
use std::ops::Sub;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use btleplug::api::{
    BDAddr, Central, CentralEvent, Characteristic, Manager, Peripheral, ScanFilter, WriteType,
};
use btleplug::platform::PeripheralId;
use futures::StreamExt;
use log::{debug, warn};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::connection::SerialConnection;
use crate::error::ConnectionError;

const V5_ROBOT_SERVICE: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb13d5);

const CHARACTERISTIC_TX_DATA: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb1306); // WRITE_WITHOUT_RESPONSE | NOTIFY | INDICATE
const CHARACTERISTIC_RX_DATA: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb13f5); // WRITE_WITHOUT_RESPONSE | WRITE | NOTIFY

const CHARACTERISTIC_TX_USER: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb1316); // WRITE_WITHOUT_RESPONSE | NOTIFY | INDICATE
const CHARACTERISTIC_RX_USER: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb1326); // WRITE_WITHOUT_RESPONSE | WRITE | NOTIFY

const CHARACTERISTIC_CODE: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb13e5); // READ | WRITE_WITHOUT_RESPONSE | WRITE

const WRITE_TIME: Duration = Duration::from_millis(25);

pub(crate) struct Characteristics {
    pub(crate) tx_data: Characteristic,
    pub(crate) rx_data: Characteristic,
    pub(crate) tx_user: Characteristic,
    pub(crate) rx_user: Characteristic,
}

pub(crate) async fn connect_to_robot(
    mac_address: Option<String>,
    pin: Option<String>,
) -> Result<(btleplug::platform::Peripheral, Characteristics), ConnectionError> {
    let mac_address =
        mac_address.map(|address| BDAddr::from_str(&address).expect("Invalid MAC address"));
    let mut pin = pin.as_deref().map(parse_pin);

    let manager = match btleplug::platform::Manager::new().await {
        Ok(man) => man,
        Err(_) => return Err(ConnectionError::NoBluetoothAdapters)
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

    if device.is_none() {
        return Err(ConnectionError::DeviceNotFound);
    }

    let peripheral = device.unwrap();
    if !peripheral.is_connected().await? {
        peripheral.connect().await?;
    } else {
        warn!("Bluetooth peripheral already connected");
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

    let tx_data = tx_data.unwrap();
    let rx_data = rx_data.unwrap();
    let tx_user = tx_user.unwrap();
    let rx_user = rx_user.unwrap();
    let code = code.unwrap();

    let vec = peripheral.read(&code).await?;
    if u32::from_be_bytes(vec[0..4].try_into().unwrap()) == 0xdeadface {
        peripheral
            .write(&code, &[0xFF, 0xFF, 0xFF, 0xFF], WriteType::WithoutResponse)
            .await?;
    }

    while pin.is_none() {
        println!("Please enter the PIN shown on the V5 brain");
        let mut str = String::new();
        std::io::stdin()
            .read_line(&mut str)
            .expect("Failed to read stdin");
        if str.len() >= 4 && u16::from_str(&str[..4]).is_ok() {
            pin = Some(parse_pin(&str[..4]));
        }
    }

    let pin = pin.unwrap();

    peripheral
        .write(&code, &pin, WriteType::WithoutResponse)
        .await?;

    let read = peripheral.read(&code).await?;
    if read != pin {
        return Err(ConnectionError::InvalidPIN);
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

pub(crate) struct DualSubscribedBluetoothConnection {
    send_timer: Mutex<Cell<SystemTime>>,
    rx_characteristic: Characteristic,
    read_buf: Arc<Mutex<Vec<u8>>>,
    peripheral: btleplug::platform::Peripheral,
}

impl DualSubscribedBluetoothConnection {
    pub(crate) async fn create(
        tx_characteristic: Characteristic,
        rx_characteristic: Characteristic,
        peripheral: btleplug::platform::Peripheral,
    ) -> DualSubscribedBluetoothConnection {
        let arc = Arc::new(Mutex::new(Vec::new()));

        let arc1 = arc.clone();
        let peripheral1 = peripheral.clone();
        let res = peripheral.subscribe(&tx_characteristic).await;

        if cfg!(not(windows)) {
            res.unwrap();
        }

        tokio::spawn(async move {
            let mut generator = peripheral1
                .notifications()
                .await
                .expect("Failed to listen to notifications");

            loop {
                if let Some(val) = generator.next().await {
                    if val.uuid == tx_characteristic.uuid {
                        arc1.lock().await.extend(val.value);
                    }
                }
            }
        });

        DualSubscribedBluetoothConnection {
            send_timer: Mutex::new(Cell::new(SystemTime::now().sub(Duration::from_secs(1)))),
            rx_characteristic,
            read_buf: arc,
            peripheral,
        }
    }
}

#[async_trait]
impl SerialConnection for DualSubscribedBluetoothConnection {
    async fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        let t = SystemTime::now();
        let guard = self.send_timer.lock().await;
        let mut chunks = buf.chunks_exact(244);
        for chunk in chunks.by_ref() {
            let time2 = guard.get();
            if let Some(duration) = WRITE_TIME.checked_sub(
                SystemTime::now()
                    .duration_since(time2)
                    .expect("time ran backwards"),
            ) {
                tokio::time::sleep(duration).await;
            }
            guard.set(SystemTime::now());
            if let Err(err) = self
                .peripheral
                .write(&self.rx_characteristic, chunk, WriteType::WithoutResponse)
                .await
            {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    err.to_string(),
                ));
            }
        }

        let remainder = chunks.remainder();

        if !remainder.is_empty() {
            let time1 = guard.get();
            if let Some(duration) = WRITE_TIME.checked_sub(
                SystemTime::now()
                    .duration_since(time1)
                    .expect("time ran backwards"),
            ) {
                tokio::time::sleep(duration).await;
            }
            guard.set(SystemTime::now());
            drop(guard);
            if let Err(err) = self
                .peripheral
                .write(&self.rx_characteristic, remainder, WriteType::WithResponse)
                .await
            {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    err.to_string(),
                ));
            }
        }

        debug!(
            "write took {}ms",
            SystemTime::now()
                .duration_since(t)
                .expect("time ran backwards")
                .as_millis()
        );

        Ok(())
    }

    async fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    async fn clear(&mut self) -> std::io::Result<()> {
        let mut guard = self.read_buf.lock().await;
        if !guard.is_empty() {
            guard.clear();
        }

        Ok(())
    }

    async fn try_read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut guard = self.read_buf.lock().await;
        let len = buf.len().min(guard.len());
        buf[..len].copy_from_slice(&guard[..len]);
        guard.copy_within(len.., 0);
        let i = guard.len();
        guard.truncate(i - len);
        Ok(len)
    }

    async fn read(&mut self, mut buf: &mut [u8]) -> std::io::Result<()> {
        let mut fail = 0;
        while !buf.is_empty() {
            match self.try_read(buf).await {
                Ok(0) => {
                    fail += 1;
                    if fail >= 1000 / 10 {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                Ok(n) => {
                    buf = &mut buf[n..];
                }
                Err(e) => return Err(e)
            }
        }
        if !buf.is_empty() {
            Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "failed to fill whole buffer",
            ))
        } else {
            Ok(())
        }
    }

    async fn try_read_one(&mut self) -> std::io::Result<u8> {
        let mut guard = self.read_buf.lock().await;
        return if !guard.is_empty() {
            Ok(guard.remove(0))
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "eof",
            ))
        };
    }
}

fn parse_pin(str: &str) -> [u8; 4] {
    assert_eq!(str.len(), 4);
    let mut chars = str.chars();
    u16::from_str(&str).expect("Invalid PIN!");

    [
        chars.next().unwrap().to_digit(10).unwrap() as u8,
        chars.next().unwrap().to_digit(10).unwrap() as u8,
        chars.next().unwrap().to_digit(10).unwrap() as u8,
        chars.next().unwrap().to_digit(10).unwrap() as u8,
    ]
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
