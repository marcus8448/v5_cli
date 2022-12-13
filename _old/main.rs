// mod lib;
//
// use std::alloc::System;
// use std::error::Error;
// use std::io::{Read, Write};
// use std::mem::size_of;
// use std::sync::atomic::{AtomicBool, Ordering};
// use std::time::Duration;
// use clap::{Arg, ArgAction, ArgMatches, Command};
// use serialport::{DataBits, Parity, SerialPortType};
//
// #[global_allocator]
// static ALLOCATOR: System = System;
//
// static EXIT: AtomicBool = AtomicBool::new(false);
// const CONTROLLER_SIZE: usize = 3 + (size_of::<f32>() * 4);
// const MOTOR_SIZE: usize = (size_of::<f64>() * 3) + (size_of::<u32>() * 1);
// const SIZE: usize = CONTROLLER_SIZE + (MOTOR_SIZE * 4);
//
// const STATUS: &'static str = "status";
// const SERIAL: &'static str = "serial";
// const UPLOAD: &'static str = "upload";
//
// struct BaseArgs {
//     port: String,
// }
//
// struct SerialArgs {
//     plugins: String,
// }
//
// struct UploadArgs {
//     plugins: String,
// }
//
// enum ParsedCommand {
//     Status(BaseArgs),
//     Serial(BaseArgs, SerialArgs),
//     Upload(BaseArgs, )
// }
//
// fn check_bit(num: u8, bit: u8) -> bool {
//     (num & bit) == bit
// }
//
// fn f32_from_slice(slice: &[u8]) -> f32 {
//     let mut buf: [u8; 4] = [0_u8; 4];
//     assert_eq!(slice.len(), 4);
//     buf.copy_from_slice(slice);
//     f32::from_le_bytes(buf)
// }
//
// fn main() {
//
//
//     let matches = command
//         .subcommand(Command::new(STATUS)
//             .about("Returns whether a V5 robot is connected")
//         )
//         .subcommand(Command::new(SERIAL)
//             .about("Opens a connection to a robot running a supported program")
//         ).get_matches();
//
//     match matches.subcommand() {
//         Some((command, matches)) => {
//
//         }
//         None => {
//             const DEFAULT_NAME: &'static str = "prog";
//             let exec = match std::env::current_exe() {
//                 Ok(x) => x.file_name().map(|f| f.to_str().unwrap_or(DEFAULT_NAME)).unwrap_or(DEFAULT_NAME),
//                 Err(_) => DEFAULT_NAME
//             };
//             println!("No command specified! See use `{} --help` for usage.", exec)
//         }
//     }
//
//     // tokio::runtime::Builder::new_current_thread()
//     //     .enable_all()
//     //     .build()
//     //     .unwrap()
//     //     .block_on(start(args));
// }
//
// fn register_default_plugins() {
//
// }
//
// // async fn start(args: ArgMatches) {
// //     args.su
// // }
// //
// // async fn run() -> Result<(), Box<dyn Error>> {
// //     let mut out = csv::Writer::from_path("output.csv")?;
// //     out.write_record(&["FR-Actual Velocity", "FR-Position", "FL-Actual Velocity", "FL-Position", "BR-Actual Velocity", "BR-Position", "BL-Actual Velocity", "BL-Position"])?;
// //     let mut serial_port = None;
// //     for p in serialport::available_ports().expect("Failed to obtain list of ports!") {
// //         if let SerialPortType::UsbPort(info) = p.port_type {
// //             println!("{}: {} {} ({})", p.port_name, info.pid, info.vid, info.manufacturer.unwrap_or(String::new("")));
// //             if info.pid == 0x0501 && info.vid == 0x2888 {
// //                 serial_port = Some(serialport::new(p.port_name, 115200).parity(Parity::None).data_bits(DataBits::Eight));
// //                 break;
// //             }
// //         }
// //     }
// //     let mut serial_port = serial_port.expect("Failed to find robot!").open().expect("Failed to connect to robot!");
// //
// //     std::thread::spawn(|| {
// //         let mut buffer = String::new();
// //         loop {
// //             std::io::stdin().read_line(&mut buffer).expect("Failed to read command line input!");
// //             if buffer.trim().eq_ignore_ascii_case("exit") {
// //                 EXIT.store(true, Ordering::Relaxed);
// //                 break;
// //             }
// //         }
// //     });
// //
// //     let mut connected = false;
// //     let mut buf = [0_u8; 4];
// //     let mut state_buf = [0_u8; SIZE];
// //
// //     loop {
// //         if !connected {
// //             serial_port.write_all(b"cnct")?;
// //             serial_port.flush()?;
// //             serial_port.read_exact(&mut buf)?;
// //             if buf != *b"recv" {
// //                 continue;
// //             }
// //             serial_port.write_all(b"okay")?;
// //             serial_port.flush()?;
// //             connected = true;
// //         } else {
// //             serial_port.write_all(b"R_ST")?;
// //             serial_port.read_exact(&mut state_buf)?;
// //             let a = check_bit(state_buf[0], 0b00000001);
// //             let b = check_bit(state_buf[0], 0b00000010);
// //             let x = check_bit(state_buf[0], 0b00000100);
// //             let y = check_bit(state_buf[0], 0b00001000);
// //             let up = check_bit(state_buf[0], 0b00010000);
// //             let down = check_bit(state_buf[0], 0b00100000);
// //             let left = check_bit(state_buf[0], 0b01000000);
// //             let right = check_bit(state_buf[0], 0b10000000);
// //             let l1 = check_bit(state_buf[1], 0b00000001);
// //             let l2 = check_bit(state_buf[1], 0b00000010);
// //             let r1 = check_bit(state_buf[1], 0b00000100);
// //             let r2 = check_bit(state_buf[1], 0b00001000);
// //             let digital_speed = state_buf[2];
// //             let left_stick_x = f32_from_slice(&state_buf[3..7]);
// //             let left_stick_y = f32_from_slice(&state_buf[(3 + size_of::<f32>())..(3 + size_of::<f32>() + 4)]);
// //             let right_stick_x = f32_from_slice(&state_buf[(3 + size_of::<f32>() * 2)..(3 + size_of::<f32>() * 2 + 4)]);
// //             let right_stick_y = f32_from_slice(&state_buf[(3 + size_of::<f32>() * 3)..(3 + size_of::<f32>() * 3 + 4)]);
// //
// //             serialize_motor(&buffer[CONTROLLER_SIZE + MOTOR_SIZE * 0]);
// //             serialize_motor(&buffer[CONTROLLER_SIZE + MOTOR_SIZE * 1]);
// //             serialize_motor(&buffer[CONTROLLER_SIZE + MOTOR_SIZE * 2]);
// //             serialize_motor(&buffer[CONTROLLER_SIZE + MOTOR_SIZE * 3]);
// //
// //             if EXIT.load(Ordering::Relaxed) {
// //                 serial_port.write_all(b"gbye")?;
// //                 serial_port.flush()?;
// //                 break;
// //             }
// //             serial_port.flush()?;
// //             std::thread::sleep(Duration::from_millis(200));
// //         }
// //     }
// //
// //     Ok(())
// // }
// //
// // async fn a() {
// //
// // }
