use std::collections::HashMap;
use std::ops::Sub;
use std::str::FromStr;
use std::time::SystemTime;
use flate2::{Compression, FlushCompress};
use ini::Ini;
use v5_core::clap::{Arg, ArgAction, ArgMatches, Command, value_parser, ValueHint};
use v5_core::crc::{Algorithm, CRC_16_IBM_3740, CRC_32_BZIP2, CRC_32_CKSUM};
use v5_core::log::info;
use v5_core::plugin::Plugin;
use v5_core::serial::system::EPOCH_TO_JAN_1_2000;
use v5_core::serial::{CRC16, CRC32};
use v5_core::serial::brain_connection::{PacketId, Vid};
use v5_core::serialport::SerialPort;
use crate::UploadFinalize::{Nothing, RunScreen};

// export_plugin!(Box::new(UploadPlugin::default()));

const UPLOAD: &str = "upload";
const COLD_PACKAGE: &str = "cold";
const HOT_PACKAGE: &str = "hot";
const COLD_ADDRESS: &str = "hot-address";
const HOT_ADDRESS: &str = "hot-address";
const NAME: &str = "name";
const DESCRIPTION: &str = "description";
const INDEX: &str = "index";

pub struct UploadPlugin {

}

impl UploadPlugin {
    pub fn default() -> Self {
        UploadPlugin {}
    }
}

enum UploadFinalize {
    Run,
    RunScreen,
    Nothing
}

impl Default for UploadFinalize {
    fn default() -> Self {
        RunScreen
    }
}

impl Plugin for UploadPlugin {
    fn get_name(&self) -> &'static str {
        UPLOAD
    }

    fn create_commands(&self, command: Command, registry: &mut HashMap<&'static str, fn(ArgMatches)>) -> Command {
        registry.insert(UPLOAD, upload_program);
        command.subcommand(Command::new(UPLOAD)
            .about("Uploads a program to the robot")
            .arg(Arg::new(COLD_PACKAGE)
                .short('c')
                .help("Location of the cold package binary")
                .default_value("bin/cold.package.bin")
                .value_hint(ValueHint::FilePath)
                .action(ArgAction::Set)
            )
            .arg(Arg::new(HOT_PACKAGE)
                .short('h')
                .help("Location of the hot package binary")
                .default_value("bin/hot.package.bin")
                .value_hint(ValueHint::FilePath)
                .action(ArgAction::Set)
            )
            .arg(Arg::new(COLD_ADDRESS)
                .help("Starting memory address of the cold package binary")
                .value_parser(value_parser!(u64))
                .default_value("0x03800000")
                .action(ArgAction::Set)
            )
            .arg(Arg::new(HOT_ADDRESS)
                .help("Starting memory address of the hot package binary")
                .value_parser(value_parser!(u64))
                .default_value("0x07800000")
                .action(ArgAction::Set)
            )
            .arg(Arg::new(NAME)
                .short('n')
                .help("Name of the program when uploading")
                .default_value("Program")
                .action(ArgAction::Set)
            )
            .arg(Arg::new(DESCRIPTION)
                .short('n')
                .help("Description of the program when uploading")
                .default_value("???")
                .action(ArgAction::Set)
            )
            .arg(Arg::new(INDEX)
                .short('i')
                .help("What slot to install the program into (1-7)")
                .value_parser(value_parser!(u8))
                .default_value("1")
                .action(ArgAction::Set)
            ))
    }

    fn register_serial_plugins(&self) {}
}

fn upload_program(args: ArgMatches) {
    let program_name = args.get_one::<String>(NAME).unwrap();
    let description = args.get_one::<String>(DESCRIPTION).unwrap();
    let mut connection = v5_core::serial::connect_to_brain(args.get_one("port"));
    let cold_package = std::fs::read(*args.get_one::<&String>(COLD_PACKAGE).unwrap()).unwrap();
    let hot_package = std::fs::read(*args.get_one::<&String>(HOT_PACKAGE).unwrap()).unwrap();
    let cold_address = *args.get_one::<u32>(COLD_ADDRESS).unwrap();
    let hot_address = *args.get_one::<u32>(HOT_ADDRESS).unwrap();
    let overwrite = true;
    let index = *args.get_one::<u32>(INDEX).unwrap();
    let cold_hash = base64::encode(md5::compute(&cold_package));
    let slot = format!("slot_{}.bin", index);
    let mut compressed_cold_package = Vec::new();
    compressed_cold_package.reserve(2097152); // 2 MiB
    flate2::Compress::new(Compression(9), true).compress(&cold_package, &mut compressed_cold_package, FlushCompress::None).unwrap();
    compressed_cold_package.shrink_to_fit();
    let cold_len = compressed_cold_package.len();
    // 4 bytes,  3 ints, 4lenChar 2 ints, 24char_s
    let mut ini = Ini::new();
    ini.with_section(Some("project"))
        .set("version", "0.1.0")
        .set("ide", "none");
    ini.with_section(Some("program"))
        .set("version", "0.1.0")
        .set("name", program_name)
        .set("slot", index)
        .set("icon", "USER902x.bmp")
        .set("description", description)
        .set("date", format!("{}", chrono::DateTime::from(SystemTime::now()).format("%+")));
    let mut conf = String::new();
    ini.write_to(&mut conf).unwrap();
    let crc = CRC32.checksum(&compressed_cold_package);
    let available_package = connection.read_file_metadata(&cold_hash, vid).unwrap();
    if (available_package.size != cold_len || available_package.crc != crc) {
        info!("Cold package differs! Re-uploading...");
        
    }
    connection.initialize_file_transfer();

    let crc1 = CRC16.digest().;

    let mut compressed = Vec::new();
    compressed.reserve(2097152);

}
