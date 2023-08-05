use std::{io, thread};
use std::convert::TryInto;
use std::io::{ErrorKind, Read, stdin, Write};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use btleplug::api::{
    BDAddr, Central, CentralEvent, Characteristic, Manager, Peripheral, ScanFilter, WriteType,
};
use btleplug::platform::PeripheralId;
use futures::StreamExt;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::connection::{RobotConnection, SerialConnection};
use crate::error::{Error, Result};

const V5_ROBOT_SERVICE: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb13d5);

const CHARACTERISTIC_TX_DATA: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb1306); // WRITE_WITHOUT_RESPONSE | NOTIFY | INDICATE
const CHARACTERISTIC_RX_DATA: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb13f5); // WRITE_WITHOUT_RESPONSE | WRITE | NOTIFY

const CHARACTERISTIC_TX_USER: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb1316); // WRITE_WITHOUT_RESPONSE | NOTIFY | INDICATE
const CHARACTERISTIC_RX_USER: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb1326); // WRITE_WITHOUT_RESPONSE | WRITE | NOTIFY

const CHARACTERISTIC_CODE: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb13e5); // READ | WRITE_WITHOUT_RESPONSE | WRITE

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
        fresh = true;
        peripheral.connect().await?;
    } else {
        println!("Device already connected?");
    }

    peripheral.discover_services().await?;

    let mut tx_data: Option<Characteristic> = None;
    let mut rx_data: Option<Characteristic> = None;
    let mut tx_user: Option<Characteristic> = None;
    let mut rx_user: Option<Characteristic> = None;
    let mut code: Option<Characteristic> = None;
    for characteristic in peripheral.characteristics() {
        println!("{}", &characteristic.uuid);
        println!("PROP {:?}", &characteristic.properties);
        for desc in &characteristic.descriptors {
            println!(
                "DESC: {:?}",
                String::from_utf8(peripheral.read_descriptor(desc).await?)
            );
        }

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

    if fresh {
        let vec = peripheral.read(&code).await?;
        if u32::from_be_bytes(vec[0..4].try_into().unwrap()) != 0xdeadface {
            println!("RESLP: {:?}", &vec);
            return Err(Error::Generic("Invalid device response"));
        }
    }

    peripheral
        .write(&code, &[0xFF, 0xFF, 0xFF, 0xFF], WriteType::WithoutResponse)
        .await?;

    while pin.is_none() {
        println!("Please enter the PIN shown on the V5 brain");
        let mut str = String::new();
        stdin().read_line(&mut str).expect("Failed to read stdin");
        if str.len() == 4 && u16::from_str(&str).is_ok() {
            pin = Some(parse_pin(&str));
        }
    }

    let pin = pin.unwrap();
    println!("PIN: {:?}", pin);
    peripheral
        .write(&code, &pin, WriteType::WithoutResponse)
        .await?;

    let read = peripheral.read(&code).await?;
    if read != pin {
        println!("{:?}", read);
        return Err(Error::Generic("Invalid PIN?"));
    }

    peripheral.discover_services().await?;

    println!("C1: {:?}", &tx_data.properties);
    println!("C2: {:?}", &tx_user.properties);
    println!("C3: {:?}", &code.properties);

    Ok(RobotConnection {
        user_connection: Box::new(
            DualSubscribedBluetoothConnection::new(tx_user, rx_user, peripheral.clone()).await,
        ),
        system_connection: Box::new(
            DualSubscribedBluetoothConnection::new(tx_data, rx_data, peripheral).await,
        ),
    })
}

struct DualSubscribedBluetoothConnection {
    tx_characteristic: Characteristic,
    rx_characteristic: Characteristic,
    read_buf: Arc<Mutex<Vec<u8>>>,
    peripheral: btleplug::platform::Peripheral,
}

impl DualSubscribedBluetoothConnection {
    async fn new(
        tx_characteristic: Characteristic,
        rx_characteristic: Characteristic,
        peripheral: btleplug::platform::Peripheral,
    ) -> DualSubscribedBluetoothConnection {
        let arc = Arc::new(Mutex::new(Vec::new()));

        let arc1 = arc.clone();
        let peripheral1 = peripheral.clone();
        let characteristic1 = tx_characteristic.clone();
        peripheral1
            .subscribe(&tx_characteristic)
            .await
            .expect("Sub");
        tokio::spawn(async move {
            loop {
                let mut pin = peripheral1
                    .notifications()
                    .await
                    .expect("Failed to listen to notifications");
                loop {
                    if let Some(val) = pin.next().await {
                        if val.uuid == characteristic1.uuid {
                            println!("SUB: {:?}", String::from_utf8_lossy(&val.value));
                            arc1.lock().await.extend_from_slice(&val.value[..]);
                        }
                    }
                }
            }
        });

        DualSubscribedBluetoothConnection {
            tx_characteristic,
            rx_characteristic,
            read_buf: arc,
            peripheral,
        }
    }
}

impl Read for DualSubscribedBluetoothConnection {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        thread::scope(move |s| {
            s.spawn(move || {
                crate::TOKIO_RUNTIME.get().unwrap().block_on(async {
                    let mut guard = self.read_buf.lock().await;
                    let len = buf.len().min(guard.len());
                    buf[..len].copy_from_slice(&guard[..len]);
                    guard.copy_within(len.., 0);
                    let i = guard.len();
                    guard.truncate(i - len);
                    Ok(len)
                })
            }).join()
        }).unwrap()
    }

    fn read_exact(&mut self, mut buf: &mut [u8]) -> io::Result<()> {
        thread::scope(move |s| {
            s.spawn(move || {
                crate::TOKIO_RUNTIME.get().unwrap().block_on(async {
                    let mut fail = 0;
                    while !buf.is_empty() {
                        match {
                            let mut guard = self.read_buf.lock().await;
                            let len = buf.len().min(guard.len());
                            buf[..len].copy_from_slice(&guard[..len]);
                            guard.copy_within(len.., 0);
                            let i = guard.len();
                            guard.truncate(i - len);
                            len
                        } {
                            0 => {
                                fail += 1;
                                if fail >= 5000 / 50 {
                                    break;
                                }
                                tokio::time::sleep(Duration::from_millis(50)).await;
                            }
                            n => {
                                let tmp = buf;
                                buf = &mut tmp[n..];
                            }
                        }
                    }
                    if !buf.is_empty() {
                        Err(std::io::Error::new(
                            ErrorKind::UnexpectedEof,
                            "failed to fill whole buffer",
                        ))
                    } else {
                        Ok(())
                    }
                })
            }).join()
        }).unwrap()
    }
}

impl Write for DualSubscribedBluetoothConnection {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let result: std::result::Result<(), btleplug::Error> = thread::scope(|s| {
            s.spawn(|| {
                crate::TOKIO_RUNTIME.get().unwrap().block_on(async {
                    let mut chunks = buf.chunks_exact(244);
                    for chunk in chunks.by_ref() {
                        println!("Write chunk: {:?}", chunk);
                        self.peripheral
                            .write(&self.rx_characteristic, chunk, WriteType::WithoutResponse)
                            .await?;
                        tokio::time::sleep(Duration::from_millis(40)).await;
                    }

                    let remainder = chunks.remainder();

                    if !remainder.is_empty() {
                        println!("write remainder {:?}", remainder);
                        self.peripheral
                            .write(
                                &self.rx_characteristic,
                                &remainder,
                                WriteType::WithoutResponse,
                            )
                            .await?;
                        tokio::time::sleep(Duration::from_millis(40)).await;
                    }
                    Ok(())
                })
            })
            .join()
        })
        .unwrap();
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

impl SerialConnection for DualSubscribedBluetoothConnection {}

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
