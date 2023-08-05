use std::{io, thread};
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

    if device.is_none() {
        return Err(Error::Generic("Failed to find v5 robot"));
    }

    let peripheral = device.unwrap();
    let mut fresh = false;
    if !peripheral.is_connected().await? {
        println!("Device already connected?");
        fresh = true;
        peripheral.connect().await?;
    }

    peripheral.discover_services().await?;

    let mut vex_char_1: Option<Characteristic> = None;
    let mut vex_char_2: Option<Characteristic> = None;
    let mut vex_char_3: Option<Characteristic> = None;
    for characteristic in peripheral.characteristics() {
        println!("{}", characteristic.uuid);
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

    dbg!(&vex_char_1);
    dbg!(&vex_char_2);
    dbg!(&vex_char_3);

    if fresh {
        let vec = peripheral.read(&vex_char_3).await?;
        if u32::from_be_bytes(vec[0..4].try_into().unwrap()) != 0xdeadface {
            println!("RESLP: {:?}", &vec);
            return Err(Error::Generic("Invalid device response"));
        }
    }

    peripheral.write(
        &vex_char_3,
        &[0xFF, 0xFF, 0xFF, 0xFF],
        WriteType::WithResponse,
    ).await?;

    std::thread::sleep(Duration::from_millis(500));

    while pin.is_none() {
        println!("Please enter the PIN shown on the V5 brain");
        let mut str = String::new();
        stdin().read_line(&mut str).expect("Failed to read stdin");
        if str.len() == 4 && u16::from_str(&str).is_ok() {
            pin = Some(parse_pin(&str));
        }
    }

    let pin = pin.unwrap();
    println!("{:?}", pin);
    peripheral.write(&vex_char_3, &pin, WriteType::WithResponse).await?;

    let read = peripheral.read(&vex_char_3).await?;
    if read != pin {
        println!("{:?}", read);
        return Err(Error::Generic("Invalid PIN?"));
    }

    println!("x: {:?}", [0xc9, 0x36, 0xb8, 0x47]);
    println!("CHANGE: {:?} v {:?}", read, 0xdeadface_u32.to_be_bytes());


    println!("C1");
    for desc in &vex_char_1.descriptors {
        println!("{:?}", String::from_utf8(peripheral.read_descriptor(desc).await?));
    }
    println!("{:?}", &vex_char_1.properties);

    println!("C2");
    for desc in &vex_char_2.descriptors {
        println!("{:?}", String::from_utf8(peripheral.read_descriptor(desc).await?));
    }
    println!("{:?}", &vex_char_2.properties);

    println!("C3");
    for desc in &vex_char_3.descriptors {
        println!("{:?}", String::from_utf8(peripheral.read_descriptor(desc).await?));
    }
    println!("{:?}", &vex_char_3.properties);

    // peripheral.write(&vex_char_3, &pin, WriteType::WithResponse).await?;
    // let mut connection = SubscribedBluetoothConnection::new(vex_char_1, peripheral.clone()).await;
    // let mut cjs = Vec::new();
    // connection.read_to_end(&mut cjs)?;
    // dbg!(cjs);

    Ok(RobotConnection {
        user_connection: Box::new(
            SubscribedBluetoothConnection::new(vex_char_2, peripheral.clone()).await,
        ),
        system_connection: Box::new(
            DirectBluetoothConnection::new(vex_char_3, peripheral),
        ),
    })
}

struct SubscribedBluetoothConnection {
    characteristic: Characteristic,
    read_buf: Arc<Mutex<Vec<u8>>>,
    peripheral: btleplug::platform::Peripheral,
}


struct DirectBluetoothConnection {
    characteristic: Characteristic,
    backup: Vec<u8>,
    peripheral: btleplug::platform::Peripheral,
}

impl DirectBluetoothConnection {
    pub fn new(characteristic: Characteristic, peripheral: btleplug::platform::Peripheral) -> Self {
        Self { characteristic, backup: Vec::with_capacity(128), peripheral }
    }
}

impl Read for DirectBluetoothConnection {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match thread::scope(|s| {
            s.spawn(|| {
                crate::TOKIO_RUNTIME.get().unwrap().block_on(self.peripheral.read(&self.characteristic))
            }).join()
        }).unwrap() {
            Ok(vec) => {
                println!("RE: {:?}", vec);
                self.backup.extend(vec);
                let len = buf.len().min(self.backup.len());
                buf[..len].copy_from_slice(&self.backup[..len]);
                self.backup.copy_within(len.., 0);
                self.backup.truncate(self.backup.len() - len);
                Ok(len)
            }
            Err(err) => {
                println!("{:?}", err);
                Err(io::Error::new(ErrorKind::InvalidData, err))
            }
        }
    }
}


impl Write for DirectBluetoothConnection {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let result: std::result::Result<(), btleplug::Error> = thread::scope(|s| {
            s.spawn(|| {
                crate::TOKIO_RUNTIME.get().unwrap().block_on(async {
                    let mut chunks = buf.chunks_exact(4);
                    for chunk in chunks.by_ref() {
                        println!("Write chunk: {:?}", chunk);
                        std::thread::sleep(Duration::from_millis(50));
                        self.peripheral.write(
                            &self.characteristic,
                            chunk,
                            WriteType::WithResponse,
                        ).await?;
                    }

                    let remainder = chunks.remainder();

                    if !remainder.is_empty() {
                        println!("write remainder {:?}", remainder);
                        let mut v = [0_u8; 4];
                        v[..remainder.len()].copy_from_slice(remainder);
                        std::thread::sleep(Duration::from_millis(50));
                        self.peripheral.write(
                            &self.characteristic,
                            &v,
                            WriteType::WithResponse,
                        ).await?;
                    }
                    Ok(())
                })
            }).join()
        }).unwrap();
        if let Err(err) = result {
            dbg!(&err);
            Err(io::Error::new(ErrorKind::Other, err.to_string())) //technically breaking contract
        } else {
            Ok(buf.len())
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl SerialConnection for DirectBluetoothConnection {

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
        peripheral1.subscribe(&characteristic).await.expect("Sub");
        tokio::spawn(async move {
            loop {
                let mut pin = peripheral1.notifications().await
                    .expect("Failed to listen to notifications");
                loop {
                    if let Some(val) = pin.next().await {
                        if val.uuid == characteristic1.uuid {
                            println!("SUB: {:?}", val.value);
                            arc1.lock().unwrap().extend_from_slice(&val.value[..]);
                        }
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
        // let _ = crate::TOKIO_RUNTIME.get().unwrap().block_on(self.peripheral.unsubscribe(&self.characteristic));
        // let _ = crate::TOKIO_RUNTIME.get().unwrap().block_on(self.peripheral.disconnect());
    }
}

impl Read for SubscribedBluetoothConnection {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.read_buf.lock() {
            Ok(mut guard) => {
                let len = buf.len().min(guard.len());
                buf[..len].copy_from_slice(&guard[..len]);
                guard.copy_within(len.., 0);
                let i = guard.len();
                guard.truncate(i - len);
                Ok(len)
            }
            Err(err) => Err(io::Error::new(ErrorKind::UnexpectedEof, err.to_string()))
        }
    }
}

impl Write for SubscribedBluetoothConnection {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let result: std::result::Result<(), btleplug::Error> = thread::scope(|s| {
            s.spawn(|| {
                crate::TOKIO_RUNTIME.get().unwrap().block_on(async {
                    let mut chunks = buf.chunks_exact(4);
                    for chunk in chunks.by_ref() {
                        println!("Write chunk: {:?}", chunk);
                        std::thread::sleep(Duration::from_millis(50));
                        self.peripheral.write(
                            &self.characteristic,
                            chunk,
                            WriteType::WithResponse,
                        ).await?;
                    }

                    let remainder = chunks.remainder();

                    if !remainder.is_empty() {
                        println!("write remainder {:?}", remainder);
                        let mut v = [0_u8; 4];
                        v[..remainder.len()].copy_from_slice(remainder);
                        std::thread::sleep(Duration::from_millis(50));
                        self.peripheral.write(
                            &self.characteristic,
                            &v,
                            WriteType::WithResponse,
                        ).await?;
                    }
                    Ok(())
                })
            }).join()
        }).unwrap();
        if let Err(err) = result {
            dbg!(&err);
            Err(io::Error::new(ErrorKind::Other, err.to_string())) //technically breaking contract
        } else {
            Ok(buf.len())
        }
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
        } else if let Ok(Some(properties)) = peripheral.properties().await {
            if properties.services.contains(&V5_ROBOT_SERVICE) {
                println!("FOUND MAC: {}", peripheral.address());
                return Some(peripheral);
            }
        }
    }
    None
}
