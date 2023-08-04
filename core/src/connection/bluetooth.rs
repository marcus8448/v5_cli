use std::convert::TryInto;
use std::io::{ErrorKind, Read, stdin, Write};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use btleplug::api::{
    BDAddr, Central, CentralEvent, Characteristic, Manager, Peripheral, ScanFilter, WriteType,
};
use btleplug::platform::PeripheralId;
use futures::StreamExt;
use uuid::Uuid;

use crate::connection::{RobotConnection, SerialConnection};
use crate::error::{Error, Result};

const V5_ROBOT_SERVICE: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb13d5);

const V5_CHARACTERISTIC_1: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb1306);
const V5_CHARACTERISTIC_2: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb1316);
const V5_CHARACTERISTIC_3: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb13e5);

pub async fn connect_to_robot(
    mac_address: Option<&String>,
    pin: Option<&String>,
) -> Result<RobotConnection> {
    let mac_address =
        mac_address.map(|address| BDAddr::from_str(address).expect("Invalid MAC address"));
    let mut pin = pin.map(parse_pin);

    let manager = btleplug::platform::Manager::new().await?;
    let adapters = manager.adapters().await?;

    if adapters.is_empty() {
        return Err(Error::Generic("No bluetooth adapters available."));
    }
    let adapter = &adapters[0];

    let mut events = adapter.events().await?;
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

    if !device.is_some() {
        return Err(Error::Generic("Failed to find v5 robot"));
    }

    let peripheral = device.unwrap();
    peripheral.connect().await?;
    let mut vex_char_1: Option<Characteristic> = None;
    let mut vex_char_2: Option<Characteristic> = None;
    let mut vex_char_3: Option<Characteristic> = None;
    for characteristic in peripheral.characteristics() {
        if characteristic.uuid == V5_CHARACTERISTIC_1 {
            vex_char_1 = Some(characteristic);
        } else if characteristic.uuid == V5_CHARACTERISTIC_2 {
            vex_char_2 = Some(characteristic);
        } else if characteristic.uuid == V5_CHARACTERISTIC_3 {
            vex_char_3 = Some(characteristic);
        }
    }

    let vex_char_1 = vex_char_1.unwrap();
    let vex_char_2 = vex_char_2.unwrap();
    let vex_char_3 = vex_char_3.unwrap();

    let vec = peripheral.read(&vex_char_3).await?;
    if u32::from_be_bytes(vec[0..4].try_into().unwrap()) != 0xdeadface {
        return Err(Error::Generic("Invalid device response"));
    }

    peripheral.write(
        &vex_char_3,
        &[0xFF, 0xFF, 0xFF, 0xFF],
        WriteType::WithResponse,
    );

    while pin.is_none() {
        println!("Please enter the PIN shown on the V5 brain");
        let mut str = String::new();
        stdin().read_line(&mut str).expect("Failed to read stdin");
        if str.len() == 4 && u16::from_str(&str).is_ok() {
            pin = Some(parse_pin(&str))
        }
    }

    let pin = pin.unwrap();

    peripheral.write(&vex_char_3, &pin, WriteType::WithResponse);

    let mut i = 0;
    while peripheral.read(&vex_char_3).await? != pin {
        if i >= 50 {
            return Err(Error::Generic("Invalid PIN?"));
        }
        std::thread::sleep(Duration::from_millis(100));
        i += 1;
    }

    Ok(RobotConnection {
        user_connection: Box::new(
            SubscribedBluetoothConnection::new(vex_char_2, peripheral.clone()).await,
        ),
        system_connection: Box::new(
            SubscribedBluetoothConnection::new(vex_char_3, peripheral).await,
        ),
    })
}

struct SubscribedBluetoothConnection {
    characteristic: Characteristic,
    read_buf: Arc<Mutex<Vec<u8>>>,
    peripheral: btleplug::platform::Peripheral,
}

impl SubscribedBluetoothConnection {
    async fn new(
        characteristic: Characteristic,
        peripheral: btleplug::platform::Peripheral,
    ) -> SubscribedBluetoothConnection {
        let arc = Arc::new(Mutex::new(Vec::new()));

        let arc1 = arc.clone();
        let peripheral1 = peripheral.clone();
        let characteristic1 = characteristic.clone();
        peripheral1.subscribe(&characteristic);
        std::thread::spawn(move || loop {
            let mut pin = futures::executor::block_on(peripheral1.notifications())
                .expect("Failed to listen to notifications");
            loop {
                if let Some(val) = futures::executor::block_on(pin.next()) {
                    if val.uuid == characteristic1.uuid {
                        arc1.lock().unwrap().extend_from_slice(&val.value[..]);
                    }
                }
            }
        });

        SubscribedBluetoothConnection {
            characteristic,
            read_buf: arc,
            peripheral,
        }
    }
}

impl Drop for SubscribedBluetoothConnection {
    fn drop(&mut self) {
        let _ = futures::executor::block_on(self.peripheral.unsubscribe(&self.characteristic));
    }
}

impl Read for SubscribedBluetoothConnection {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if let Ok(mut guard) = self.read_buf.lock() {
            let len = buf.len().min(guard.len());
            buf.copy_from_slice(&guard[..len]);
            guard.copy_within(len.., 0);
            guard.truncate(len);
            return Ok(len);
        }
        Err(ErrorKind::Other.into())
    }
}

impl Write for SubscribedBluetoothConnection {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        return if futures::executor::block_on(self.peripheral.write(
            &self.characteristic,
            buf,
            WriteType::WithoutResponse,
        ))
        .is_err()
        {
            Err(ErrorKind::Other.into()) //technically breaking contract
        } else {
            Ok(buf.len())
        };
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl SerialConnection for SubscribedBluetoothConnection {}

fn parse_pin(str: &String) -> [u8; 4] {
    assert_eq!(str.len(), 4);
    let mut chars = str.chars();
    u16::from_str(str).expect("Invalid PIN!");

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
        if mac_address.is_some() {
            if mac_address.unwrap() == peripheral.address() {
                return Some(peripheral);
            }
        } else {
            println!("ID: {}", peripheral.id());
            println!("MAC: {}", peripheral.address());

            if mac_address.is_none() && peripheral.address().to_string().starts_with("54:6C:0E") {
                if let Ok(Some(properties)) = peripheral.properties().await {
                    if properties.services.contains(&V5_ROBOT_SERVICE) {
                        return Some(peripheral);
                    }
                }
            }
        }
    }
    None
}
