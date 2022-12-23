use chrono::{DateTime, Local};
use flate2::{Compress, Compression, FlushCompress};
use ini::Ini;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use tokio::task;
use v5_core::clap::builder::NonEmptyStringValueParser;
use v5_core::clap::{value_parser, Arg, ArgAction, ArgMatches, Command, ValueHint};
use v5_core::log::info;
use v5_core::plugin::{Plugin, PORT};
use v5_core::serial::system::{FileType, TransferTarget, UploadAction, Vid};
use v5_core::serial::CRC32;
use v5_core::tokio;

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

pub struct UploadPlugin {}

impl UploadPlugin {
    pub fn default() -> Self {
        UploadPlugin {}
    }
}

impl Plugin for UploadPlugin {
    fn get_name(&self) -> &'static str {
        UPLOAD
    }

    fn create_commands(
        &self,
        command: Command,
        registry: &mut HashMap<
            &'static str,
            Box<fn(ArgMatches) -> Pin<Box<dyn Future<Output = ()>>>>,
        >,
    ) -> Command {
        registry.insert(UPLOAD, Box::new(|f| Box::pin(upload_program(f))));
        command.subcommand(
            Command::new(UPLOAD)
                .about("Uploads a program to the robot")
                .arg(
                    Arg::new(COLD_PACKAGE)
                        .short('c')
                        .help("Location of the cold package binary")
                        .default_value("bin/cold.package.bin")
                        .value_hint(ValueHint::FilePath)
                        .value_parser(NonEmptyStringValueParser::new())
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new(HOT_PACKAGE)
                        .short('h')
                        .help("Location of the hot package binary")
                        .default_value("bin/hot.package.bin")
                        .value_hint(ValueHint::FilePath)
                        .value_parser(NonEmptyStringValueParser::new())
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new(COLD_ADDRESS)
                        .help("Starting memory address of the cold package binary")
                        .default_value("0x03800000")
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new(HOT_ADDRESS)
                        .help("Starting memory address of the hot package binary")
                        .default_value("0x07800000")
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new(NAME)
                        .short('n')
                        .help("Name of the program when uploading")
                        .default_value("Program")
                        .value_parser(NonEmptyStringValueParser::new())
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new(DESCRIPTION)
                        .short('n')
                        .help("Description of the program when uploading")
                        .default_value("???")
                        .value_parser(NonEmptyStringValueParser::new())
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new(INDEX)
                        .short('i')
                        .help("What slot to install the program into (1-8)")
                        .value_parser(value_parser!(u8).range(1..=8))
                        .default_value("1")
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new(ACTION)
                        .short('a')
                        .help("What to do after uploading the program")
                        .value_parser(["nothing", "run", "screen"])
                        .default_value("screen")
                        .action(ArgAction::Set),
                ),
        )
    }
}

async fn upload_program(args: ArgMatches) {
    let mut brain =
        v5_core::serial::connect_to_brain(args.get_one(PORT).map(|f: &String| f.to_string()));
    let program_name = args.get_one::<String>(NAME).unwrap();
    let description = args.get_one::<String>(DESCRIPTION).unwrap();
    let cold_package = std::fs::read(*args.get_one::<&str>(COLD_PACKAGE).unwrap()).unwrap();
    let hot_package = std::fs::read(*args.get_one::<&str>(HOT_PACKAGE).unwrap()).unwrap();
    let cold_address =
        u32::from_str_radix(*args.get_one::<&str>(COLD_ADDRESS).unwrap(), 16).unwrap();
    let hot_address = u32::from_str_radix(*args.get_one::<&str>(HOT_ADDRESS).unwrap(), 16).unwrap();
    let action = *args.get_one::<&str>(ACTION).unwrap();
    let overwrite = true;
    let index = *args.get_one::<u8>(INDEX).unwrap();
    let timestamp = Local::now();
    let cold_hash = base64::encode(extendhash::md5::compute_hash(cold_package.as_slice()));
    let file_name = format!("slot_{}.bin", index);
    let file_ini = format!("slot_{}.ini", index);
    let action = UploadAction::try_from(action).unwrap();

    let compressed_cold = task::spawn(compress(cold_package));
    let compressed_hot = task::spawn(compress(hot_package));

    let ini = generate_ini(
        "0.1.0",
        "none",
        program_name,
        "0.1.0",
        index,
        "USER902x.bmp",
        description,
        timestamp,
    );

    let cold_package = compressed_cold.await.unwrap();
    let cold_len = cold_package.len();
    let crc = CRC32.checksum(&cold_package);
    let available_package = brain
        .read_file_metadata(&cold_hash[..24], Vid::System)
        .unwrap();
    if available_package.size != cold_len as u32 || available_package.crc != crc {
        info!("Cold package differs! Re-uploading...");
        brain
            .upload_file(
                TransferTarget::Flash,
                FileType::Bin,
                Vid::Pros,
                &cold_package,
                &cold_hash[..24],
                cold_address,
                crc,
                overwrite,
                timestamp,
                None,
                UploadAction::Nothing,
            )
            .unwrap();
    }

    let hot_package = compressed_hot.await.unwrap();
    let crc = CRC32.checksum(&hot_package);
    brain
        .upload_file(
            TransferTarget::Flash,
            FileType::Bin,
            Vid::User,
            &hot_package,
            &file_name,
            hot_address,
            crc,
            overwrite,
            timestamp,
            Some((&cold_hash[..24], Vid::V5Cli)),
            action,
        )
        .unwrap();

    let conf = ini;
    let crc = CRC32.checksum(&conf);
    brain
        .upload_file(
            TransferTarget::Flash,
            FileType::Ini,
            Vid::User,
            &conf,
            &file_ini,
            0,
            crc,
            overwrite,
            timestamp,
            None,
            UploadAction::Nothing,
        )
        .unwrap();
}

async fn compress(data: Vec<u8>) -> Vec<u8> {
    let mut compress = Compress::new(Compression::new(9), true);
    let mut out = Vec::new();
    let len = data.len();
    out.reserve_exact(data.len());
    loop {
        compress
            .compress_vec(&data, &mut out, FlushCompress::None)
            .unwrap();
        if compress.total_in() == len as u64 {
            break;
        } else {
            out.reserve_exact(len - compress.total_in() as usize);
        }
    }
    out.shrink_to_fit();
    out
}

fn generate_ini(
    project_version: &str,
    ide: &str,
    name: &str,
    program_version: &str,
    slot: u8,
    icon: &str,
    description: &str,
    timestamp: DateTime<Local>,
) -> Vec<u8> {
    let mut ini = Ini::new();
    ini.with_section(Some("project"))
        .set("version", project_version)
        .set("ide", ide);
    ini.with_section(Some("program"))
        .set("version", program_version)
        .set("name", name)
        .set("slot", slot.to_string())
        .set("icon", icon)
        .set("description", description)
        .set("date", format!("{}", timestamp.format("%+")));
    let mut conf = Vec::<u8>::new();
    conf.reserve(128);
    ini.write_to(&mut conf).unwrap();
    conf.shrink_to_fit();
    conf
}
