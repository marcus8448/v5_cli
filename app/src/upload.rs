use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use base64::Engine;
use clap::{Arg, ArgAction, ArgMatches, Command, value_parser, ValueHint};
use clap::builder::NonEmptyStringValueParser;
use crc::{Algorithm, Crc};
use ini::Ini;
use libdeflater::{CompressionLvl, Compressor};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use v5_core::brain::Brain;
use v5_core::brain::filesystem::{FileFlags, FileType, TransferDirection, TransferTarget, UploadAction, Vid};
use v5_core::connection::RobotConnectionOptions;
use v5_core::error::CommandError;

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

pub(crate) const COMMAND: &str = "upload";
const COLD_PACKAGE: &str = "cold";
const HOT_PACKAGE: &str = "hot";
const COLD_ADDRESS: &str = "cold-address";
const HOT_ADDRESS: &str = "hot-address";
const NAME: &str = "name";
const DESCRIPTION: &str = "description";
const INDEX: &str = "index";
const ACTION: &str = "action";

pub(crate) fn command() -> Command {
    Command::new(COMMAND)
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
        )
}

pub(crate) async fn upload(
    _cmd: &mut Command,
    args: ArgMatches,
    options: RobotConnectionOptions,
) -> Result<(), CommandError> {
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

    let brain = tokio::task::spawn(v5_core::connection::connect_to_brain(options));
    let cold_handle = tokio::task::spawn(load_compressed(cold_package_path)); //probably overkill
    let hot_handle = tokio::task::spawn(load_compressed(hot_package_path));

    let ini = generate_program_ini(
        "0.1.0",
        "PROS",
        program_name,
        "0.1.0",
        index,
        "USER902x.bmp",
        description,
        timestamp,
    )
    .await;

    let cold_package = cold_handle.await.unwrap()?;
    let cold_hash = base64::engine::general_purpose::STANDARD
        .encode(extendhash::md5::compute_hash(cold_package.as_slice()));
    let cold_len = cold_package.len();
    let crc = CRC32.checksum(&cold_package);
    let cold_package_name = &cold_hash[..22];

    let mut skip_cold = false;

    let mut brain = brain.await.unwrap()?;
    let available_package = brain.get_file_metadata_by_name(Vid::Pros, FileFlags::empty(), cold_package_name).await;

    if let Ok(package) = &available_package {
        if package.size == cold_len as u32 && package.crc == crc {
            skip_cold = true;
        }
    } else {
        // todo handle/check nack
    }

    if !skip_cold {
        println!("Invalid cold package! Re-uploading...");
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
        .await?;
    }

    let hot_package = hot_handle.await.unwrap()?;
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
    .await?;

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
    .await?;
    Ok(())
}

async fn load_compressed<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, std::io::Error> {
    let input = std::fs::read(&path)?;
    let input_hash = extendhash::sha256::compute_hash(&input);
    let path = path.as_ref();
    let cache = adjacent_file(path, "cache");
    let gz_cache = adjacent_file(path, "gz");

    if let Ok(meta) = std::fs::metadata(&cache) {
        if meta.is_file() && meta.len() == 32 {
            if let Ok(gz_meta) = std::fs::metadata(&gz_cache) {
                if gz_meta.is_file() {
                    let mut cache = std::fs::File::open(&cache).unwrap();
                    let mut data = [0_u8; 32];
                    cache.read_exact(&mut data).unwrap();

                    if input_hash == data {
                        let gzipped = std::fs::read(&gz_cache).unwrap();
                        cache.read_exact(&mut data).unwrap();
                        if extendhash::sha256::compute_hash(&gzipped) == data {
                            return Ok(gzipped);
                        }
                    }
                }
            }
        }
    }

    let mut compressor = Compressor::new(CompressionLvl::best());
    let max_len = compressor.gzip_compress_bound(input.len());
    let mut compressed_data = vec![0; max_len];
    let size = compressor
        .gzip_compress(&input, &mut compressed_data)
        .unwrap();
    compressed_data.truncate(size);

    let comp2 = compressed_data.clone();
    tokio::task::spawn_blocking(move || {
        let mut cache = std::fs::OpenOptions::new().create(true).write(true).truncate(true).open(cache).unwrap();
        let mut gz_cache = std::fs::OpenOptions::new().create(true).write(true).truncate(true).open(gz_cache).unwrap();

        gz_cache.write_all(&comp2).unwrap();
        cache.write_all(&input_hash).unwrap();
        cache.write_all(&extendhash::sha256::compute_hash(&comp2)).unwrap();
    });

    Ok(compressed_data)
}

fn adjacent_file(path: &Path, extension: &'static str) -> PathBuf {
    if let Some(ext) = path.extension() {
        if !ext.is_empty() {
            if let Some(ext) = ext.to_str() {
                return path.with_extension(format!("{}.{}", ext, extension))
            }
        }
    }
    path.with_extension(extension)
}

async fn upload_file(
    brain: &mut Brain,
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
) -> Result<(), CommandError> {
    let mut transfer = brain.file_transfer_initialize(
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
    .await?;
    assert!(transfer.parameters.file_size >= file.len() as u32);
    if let Some((name, vid)) = linked_file {
        transfer.set_link(name, vid).await?;
    }
    let max_packet_size = (((transfer.parameters.max_packet_size / 2) / 244) * 244) - 14;
    let max_packet_size = max_packet_size - (max_packet_size % 4); //4 byte alignment
    for i in (0..file.len()).step_by(max_packet_size as usize) {
        let end = file.len().min(i + max_packet_size as usize);
        transfer.write(&file[i..end], address + i as u32).await?;
    }
    transfer.complete(action).await?;
    Ok(())
}

async fn generate_program_ini(
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
