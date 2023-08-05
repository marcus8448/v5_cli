use std::io;
use std::io::Read;
use std::path::Path;
use std::thread::JoinHandle;
use std::time::SystemTime;

use base64::Engine;
use ini::Ini;
use libdeflater::{CompressionLvl, Compressor};

use v5_core::clap::{Arg, ArgAction, ArgMatches, Command, value_parser, ValueHint};
use v5_core::clap::builder::NonEmptyStringValueParser;
use v5_core::connection::{RobotConnection, SerialConnection};
use v5_core::crc::{Algorithm, Crc};
use v5_core::log::info;
use v5_core::packet::filesystem::{
    FileTransferComplete, FileTransferInitialize, FileTransferWrite, FileType,
    GetFileMetadataByName, SetFileTransferLink, TransferDirection, TransferTarget, UploadAction,
    Vid,
};
use v5_core::packet::Packet;
use v5_core::plugin::{CommandRegistry, Plugin};
use v5_core::time::format_description::well_known::Rfc3339;
use v5_core::time::OffsetDateTime;

// export_plugin!(Box::new(UploadPlugin::default()));
pub const CRC32: Crc<u32> = Crc::<u32>::new(&Algorithm {
    width: 32,
    poly: 0x04C11DB7,
    init: 0,
    refin: false,
    refout: false,
    xorout: 0,
    check: 0x89A1897F,
    residue: 0,
});

const UPLOAD: &str = "upload";
const COLD_PACKAGE: &str = "cold";
const HOT_PACKAGE: &str = "hot";
const COLD_ADDRESS: &str = "cold-address";
const HOT_ADDRESS: &str = "hot-address";
const NAME: &str = "name";
const DESCRIPTION: &str = "description";
const INDEX: &str = "index";
const ACTION: &str = "action";

pub struct UploadPlugin {}

impl Plugin for UploadPlugin {
    fn get_name(&self) -> &'static str {
        UPLOAD
    }

    fn create_commands(&self, command: Command, registry: &mut CommandRegistry) -> Command {
        registry.insert(UPLOAD, Box::new(upload_program));
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
                        .short('t')
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
                        .short('d')
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

fn upload_program(args: ArgMatches, robot: RobotConnection) {
    let mut brain = robot.system_connection;
    let program_name = args.get_one::<String>(NAME).unwrap();
    let description = args.get_one::<String>(DESCRIPTION).unwrap();
    let cold_package_path = args.get_one::<String>(COLD_PACKAGE).unwrap().clone();
    let hot_package_path = args.get_one::<String>(HOT_PACKAGE).unwrap().clone();
    let cold_address = u32::from_str_radix(
        args.get_one::<String>(COLD_ADDRESS)
            .unwrap()
            .replace("0x", "")
            .as_str(),
        16,
    )
    .unwrap();
    let hot_address = u32::from_str_radix(
        args.get_one::<String>(HOT_ADDRESS)
            .unwrap()
            .replace("0x", "")
            .as_str(),
        16,
    )
    .unwrap();
    let action = args.get_one::<String>(ACTION).unwrap();
    let overwrite = true;
    let index = *args.get_one::<u8>(INDEX).unwrap() - 1;
    let timestamp = SystemTime::now();
    let file_name = format!("slot_{}.bin", index);
    let file_ini = format!("slot_{}.ini", index);
    let action = UploadAction::try_from(action.as_str()).unwrap();

    let cold_handle: JoinHandle<Result<Vec<u8>, io::Error>> =
        std::thread::spawn(move || load_compressed(cold_package_path)); //probably overkill
    let hot_handle: JoinHandle<Result<Vec<u8>, io::Error>> =
        std::thread::spawn(move || load_compressed(hot_package_path));

    let ini = generate_program_ini(
        "0.1.0",
        "PROS",
        program_name,
        "0.1.0",
        index,
        "USER902x.bmp",
        description,
        timestamp,
    );
    println!("{}", String::from_utf8(ini.clone()).unwrap());

    let cold_package = cold_handle.join().unwrap().unwrap();
    let cold_hash = base64::engine::general_purpose::STANDARD
        .encode(extendhash::md5::compute_hash(cold_package.as_slice()));
    let cold_len = cold_package.len();
    let crc = CRC32.checksum(&cold_package);
    let cold_package_name = &cold_hash[..22];

    let mut skip_cold = false;

    let available_package =
        GetFileMetadataByName::new(Vid::Pros, 0, cold_package_name).send(&mut brain);

    if let Ok(package) = &available_package {
        if package.size == cold_len as u32 && package.crc == crc {
            skip_cold = true;
        }
    } else {
        // todo handle/check nack
    }

    if !skip_cold {
        info!("Invalid cold package! Re-uploading...");
        upload_file(
            &mut brain,
            TransferTarget::Flash,
            FileType::Bin,
            Vid::Pros,
            &cold_package,
            cold_package_name,
            cold_address,
            crc,
            overwrite,
            timestamp,
            None,
            UploadAction::Nothing,
        )
        .unwrap();
    }

    let hot_package = hot_handle.join().unwrap().unwrap();
    let crc = CRC32.checksum(&hot_package);
    upload_file(
        &mut brain,
        TransferTarget::Flash,
        FileType::Bin,
        Vid::User,
        &hot_package,
        &file_name,
        hot_address,
        crc,
        overwrite,
        timestamp,
        Some((cold_package_name, Vid::Pros)),
        UploadAction::Nothing,
    )
    .unwrap();

    let conf = ini;
    let crc = CRC32.checksum(&conf);
    upload_file(
        &mut brain,
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
        action,
    )
    .unwrap();
}

fn load_compressed<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, io::Error> {
    let mut file = std::fs::File::open(path)?;
    let len = usize::try_from(file.metadata().unwrap().len()).expect("file too large");
    let mut compressor = Compressor::new(CompressionLvl::best());
    let max_len = compressor.gzip_compress_bound(len);
    let mut compressed_data = Vec::with_capacity(max_len);
    compressed_data.resize(max_len, 0);
    let mut input = Vec::with_capacity(len);
    file.read_to_end(&mut input).expect("failed to read input");
    let size = compressor
        .gzip_compress(&input, &mut compressed_data)
        .unwrap();
    compressed_data.resize(size, 0);
    Ok(compressed_data)
}

fn upload_file(
    brain: &mut Box<dyn SerialConnection>,
    target: TransferTarget,
    file_type: FileType,
    vid: Vid,
    file: &[u8],
    remote_name: &str,
    address: u32,
    crc: u32,
    overwrite: bool,
    timestamp: SystemTime,
    linked_file: Option<(&str, Vid)>,
    action: UploadAction,
) -> v5_core::error::Result<()> {
    let meta = FileTransferInitialize::new(
        TransferDirection::Upload,
        target,
        vid,
        overwrite,
        file.len() as u32,
        address,
        crc,
        0b00_01_00,
        file_type,
        remote_name,
        timestamp,
    )
    .send(brain)?;
    assert!(meta.file_size >= file.len() as u32);
    if let Some((name, vid)) = linked_file {
        SetFileTransferLink::new(name, vid).send(brain)?;
    }
    let max_packet_size = meta.max_packet_size / 2;
    let max_packet_size = max_packet_size - (max_packet_size % 4); //4 byte alignment
    for i in (0..file.len()).step_by(max_packet_size as usize) {
        let end = file.len().min(i + max_packet_size as usize);
        FileTransferWrite::new(&file[i..end], address + i as u32).send(brain)?;
    }
    FileTransferComplete::new(action).send(brain)?;
    Ok(())
}

fn generate_program_ini(
    project_version: &str,
    ide: &str,
    name: &str,
    _program_version: &str,
    slot: u8,
    icon: &str,
    description: &str,
    timestamp: SystemTime,
) -> Vec<u8> {
    let mut ini = Ini::new();
    ini.with_section(Some("project"))
        .set("version", project_version)
        .set("ide", ide);
    ini.with_section(Some("program"))
        .set("version", "16777216")
        .set("name", name)
        .set("slot", slot.to_string())
        .set("icon", icon)
        .set("description", description)
        .set(
            "date",
            OffsetDateTime::from(timestamp)
                .format(&Rfc3339)
                .unwrap()
                .trim_end_matches('Z'),
        );
    let mut conf = Vec::<u8>::new();
    conf.reserve(128);
    ini.write_to(&mut conf).unwrap();
    conf.shrink_to_fit();
    conf
}
