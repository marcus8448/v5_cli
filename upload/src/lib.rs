use std::collections::HashMap;
use flate2::{Compress, Compression, FlushCompress};
use ini::Ini;
use v5_core::clap::{Arg, ArgAction, ArgMatches, Command, value_parser, ValueHint};
use v5_core::clap::builder::Str;
use v5_core::log::info;
use v5_core::plugin::Plugin;
use v5_core::serial::system::{FileType, TransferTarget, UploadAction, Vid};
use v5_core::serial::{CRC16, CRC32};

// export_plugin!(Box::new(UploadPlugin::default()));

const UPLOAD: &str = "upload";
const COLD_PACKAGE: &str = "cold";
const HOT_PACKAGE: &str = "hot";
const COLD_ADDRESS: &str = "hot-address";
const HOT_ADDRESS: &str = "hot-address";
const NAME: &str = "name";
const DESCRIPTION: &str = "description";
const INDEX: &str = "index";
const ACTION: &str = "action";

pub struct UploadPlugin {
}

impl UploadPlugin {
    pub fn default() -> Self {
        UploadPlugin {}
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
                .default_value("0x03800000")
                .action(ArgAction::Set)
            )
            .arg(Arg::new(HOT_ADDRESS)
                .help("Starting memory address of the hot package binary")
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
            )
            .arg(Arg::new(ACTION)
                .short('a')
                .help("What to do after uploading the program")
                .value_parser(["nothing", "run", "screen"])
                .default_value("screen")
                .action(ArgAction::Set)
            ))
    }
}

fn upload_program(args: ArgMatches) {
    let program_name = args.get_one::<String>(NAME).unwrap();
    let description = args.get_one::<String>(DESCRIPTION).unwrap();
    let mut brain = v5_core::serial::connect_to_brain(args.get_one("port").map(|f: &String| f.clone()));
    let cold_package = std::fs::read(*args.get_one::<&String>(COLD_PACKAGE).unwrap()).unwrap();
    let mut hot_package = std::fs::read(*args.get_one::<&String>(HOT_PACKAGE).unwrap()).unwrap();
    let cold_address = u32::from_str_radix(*args.get_one::<&str>(COLD_ADDRESS).unwrap(), 16).unwrap();
    let hot_address = u32::from_str_radix(*args.get_one::<&str>(HOT_ADDRESS).unwrap(), 16).unwrap();
    let action = *args.get_one::<&str>(ACTION).unwrap();
    let overwrite = true;
    let index = *args.get_one::<u32>(INDEX).unwrap();
    let timestamp = chrono::Local::now();
    let cold_hash = base64::encode(extendhash::md5::compute_hash(cold_package.as_slice()));
    let file_name = format!("slot_{}.bin", index);
    let file_ini = format!("slot_{}.ini", index);

    let mut compress = Compress::new(Compression::new(9), true);

    let mut compressed_cold = Vec::new();
    compressed_cold.reserve(cold_package.len());
    compress.compress(&cold_package, &mut compressed_cold, FlushCompress::None).unwrap();
    compressed_cold.shrink_to_fit();
    let cold_package = compressed_cold;

    let mut compressed_hot = Vec::new();
    compressed_hot.reserve(hot_package.len());
    compress.compress(&hot_package, &mut compressed_hot, FlushCompress::None).unwrap();
    compressed_hot.shrink_to_fit();
    let hot_package = compressed_hot;

    let cold_len = cold_package.len();
    let action = UploadAction::try_from(action).unwrap();
    // 4 bytes,  3 ints, 4lenChar 2 ints, 24char_s
    let mut ini = Ini::new();
    ini.with_section(Some("project"))
        .set("version", "0.1.0")
        .set("ide", "none");
    ini.with_section(Some("program"))
        .set("version", "0.1.0")
        .set("name", program_name)
        .set("slot", index.to_string())
        .set("icon", "USER902x.bmp")
        .set("description", description)
        .set("date", format!("{}", timestamp.format("%+")));
    let mut conf = Vec::<u8>::new();
    ini.write_to(&mut conf).unwrap();

    let crc = CRC32.checksum(&cold_package);
    let available_package = brain.read_file_metadata(&cold_hash[..24], Vid::System).unwrap();
    if available_package.size != cold_len as u32 || available_package.crc != crc {
        info!("Cold package differs! Re-uploading...");
        brain.upload_file(TransferTarget::Flash, FileType::Bin, Vid::Pros, &cold_package, &cold_hash[..24], cold_address, crc, overwrite, timestamp, None, UploadAction::Nothing).unwrap();
    }

    let crc = CRC32.checksum(&hot_package);
    brain.upload_file(TransferTarget::Flash, FileType::Bin, Vid::User, &hot_package, &file_name, hot_address, crc, overwrite, timestamp, Some((&cold_hash[..24], Vid::Pros)), action).unwrap();
    let crc = CRC32.checksum(&conf);
    brain.upload_file(TransferTarget::Flash, FileType::Ini, Vid::User, &conf, &file_ini, 0, crc, overwrite, timestamp, None, UploadAction::Nothing).unwrap();

    let mut compressed = Vec::<u8>::new();
}
