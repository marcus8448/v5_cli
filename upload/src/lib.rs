use std::collections::HashMap;
use flate2::{Compression, FlushCompress};
use v5_core::clap::{Arg, ArgAction, ArgMatches, Command, value_parser, ValueHint};
use v5_core::plugin::Plugin;
use v5_core::serial::{CRC16, CRC32};

// export_plugin!(Box::new(UploadPlugin::default()));

const UPLOAD: &str = "upload";
const COLD_PACKAGE: &str = "cold";
const HOT_PACKAGE: &str = "hot";
const COLD_ADDRESS: &str = "hot-address";
const HOT_ADDRESS: &str = "hot-address";
const NAME: &str = "name";
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
        UploadFinalize::RunScreen
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
    let mut connection = v5_core::serial::open_brain_connection(args.get_one("port").map(|f: &String| f.clone()));
    let cold_package = std::fs::read(*args.get_one::<&String>(COLD_PACKAGE).unwrap()).unwrap();
    let hot_package = std::fs::read(*args.get_one::<&String>(HOT_PACKAGE).unwrap()).unwrap();
    let cold_address = *args.get_one::<u32>(COLD_ADDRESS).unwrap();
    let hot_address = *args.get_one::<u32>(HOT_ADDRESS).unwrap();
    let overwrite = true;
    let index = *args.get_one::<u32>(INDEX).unwrap();
    let cold_hash = String::from_utf8_lossy(md5::compute(cold_package.as_slice()).as_slice());
    let slot = format!("slot_{}.bin", index);
    let mut compressed_cold_package = Vec::new();
    compressed_cold_package.reserve(2097152); // 2 MiB
    flate2::Compress::new(Compression::best(), true).compress(&cold_package, &mut compressed_cold_package, FlushCompress::None).unwrap();
    compressed_cold_package.shrink_to_fit();
    let cold_len = compressed_cold_package.len();
    // 4 bytes,  3 ints, 4 sohrts 2 ints, 24shorts

    let crc = CRC32.checksum(&compressed_cold_package);
    connection.send_large_packet()

    let crc1 = CRC16.digest().;

    let mut compressed = Vec::new();
    compressed.reserve(2097152);

}
