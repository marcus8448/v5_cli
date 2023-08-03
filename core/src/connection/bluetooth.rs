use btleplug::api::{Central, CentralEvent, Characteristic, Manager, Peripheral, ScanFilter, WriteType};
use std::time::Duration;
use std::convert::TryInto;
use btleplug::platform::PeripheralId;
use futures::StreamExt;
use uuid::Uuid;
use crate::error::{Error, Result};

const V5_ROBOT_SERVICE: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb13d5);

const V5_CHARACTERISTIC_1: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb1306);
const V5_CHARACTERISTIC_2: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb1316);
const V5_CHARACTERISTIC_3: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb13e5);

pub async fn connect_to_robot() -> Result<()> {
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
                if let Some(peripheral) = find_vex_device(adapter, &id).await {
                    device = Some(peripheral);
                }
            }
            CentralEvent::DeviceUpdated(id) => {
                if let Some(peripheral) = find_vex_device(adapter, &id).await {
                    device = Some(peripheral);
                }
            }
            _ => {}
        }
        if device.is_some() {
            break
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

    peripheral.write(&vex_char_3, &[0xFF, 0xFF, 0xFF, 0xFF], WriteType::WithResponse);
    let pin = [1, 2, 3, 4];
    peripheral.write(&vex_char_3, &pin, WriteType::WithResponse);

    while peripheral.read(&vex_char_3).await? != pin {
        std::thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}

async fn find_vex_device(adapter: &btleplug::platform::Adapter, id: &PeripheralId) -> Option<btleplug::platform::Peripheral> {
    if let Ok(peripheral) = adapter.peripheral(id).await {
        println!("ID: {}", peripheral.id());
        println!("MAC: {}", peripheral.address());

        if peripheral.address().to_string().starts_with("54:6C:0E") {
            if let Ok(Some(properties)) = peripheral.properties().await {
                if properties.services.contains(&V5_ROBOT_SERVICE) {
                    return Some(peripheral);
                }
            }
        }
    }
    None
}
