use clap::{Arg, ArgMatches, Command, value_parser};
use clap::builder::NonEmptyStringValueParser;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use v5_core::brain::filesystem::{DeleteFlags, FileFlags, Vid};
use v5_core::brain::system::{ExecutionFlags, KernelVariable};
use v5_core::connection::RobotConnectionOptions;
use v5_core::error::CommandError;

pub(crate) const COMMAND: &str = "manage";

const STATUS: &str = "status";
const METADATA: &str = "metadata";
const LIST_FILES: &str = "ls_files";
const FILE_NAME: &str = "file_name";
const VID: &str = "vid";
const OPTION: &str = "option";
const STOP: &str = "stop";
const RUN: &str = "run";
const SLOT: &str = "slot";
const REMOVE_ALL_PROGRAMS: &str = "rm_all";
const REMOVE_FILE: &str = "rm_file";
const REMOVE_PROGRAM: &str = "rm_program";
const KERNEL_VARIABLE: &str = "variable";
const SET: &str = "set";
const GET: &str = "get";
const VARIABLE: &str = "variable";
const VALUE: &str = "value";
const CAPTURE: &str = "capture";

pub(crate) fn command() -> Command {
    Command::new(COMMAND)
        .about("Manage the robot brain")
        .subcommand(Command::new(STATUS).about("Get the status of the robot brain"))
        .subcommand(
            Command::new(METADATA)
                .about("Reads file metadata")
                .arg(
                    Arg::new(FILE_NAME)
                        .index(1)
                        .required(true)
                        .value_parser(NonEmptyStringValueParser::new()),
                )
                .arg(
                    Arg::new(VID)
                        .short('v')
                        .default_value("1")
                        .value_parser(value_parser!(u8)),
                )
                .arg(
                    Arg::new(OPTION)
                        .short('o')
                        .default_value("0")
                        .value_parser(value_parser!(u8)),
                ),
        )
        .subcommand(
            Command::new(LIST_FILES)
                .about("Lists all files on the brain")
                .arg(
                    Arg::new(VID)
                        .short('v')
                        .default_value("1")
                        .value_parser(value_parser!(u8)),
                )
                .arg(
                    Arg::new(OPTION)
                        .short('o')
                        .default_value("0")
                        .value_parser(value_parser!(u8)),
                ),
        )
        .subcommand(Command::new(STOP).about("Terminates a running program"))
        .subcommand(
            Command::new(RUN)
                .about("Starts a program on the robot")
                .arg(
                    Arg::new(SLOT)
                        .index(1)
                        .required(true)
                        .value_parser(value_parser!(u8).range(1..=8)),
                )
                .arg(
                    Arg::new(VID)
                        .short('v')
                        .default_value("1")
                        .value_parser(value_parser!(u8)),
                ),
        )
        .subcommand(
            Command::new(REMOVE_ALL_PROGRAMS)
                .about("Deletes all programs from the robot")
                .arg(
                    Arg::new(VID)
                        .short('v')
                        .default_value("1")
                        .value_parser(value_parser!(u8)),
                ),
        )
        .subcommand(
            Command::new(REMOVE_FILE)
                .about("Removes a file from the robot (by name)")
                .arg(
                    Arg::new(FILE_NAME)
                        .index(1)
                        .required(true)
                        .value_parser(NonEmptyStringValueParser::new()),
                )
                .arg(
                    Arg::new(VID)
                        .short('v')
                        .default_value("1")
                        .value_parser(value_parser!(u8)),
                ),
        )
        .subcommand(
            Command::new(REMOVE_PROGRAM)
                .about("Removes a program from the robot (by slot)")
                .arg(
                    Arg::new(SLOT)
                        .index(1)
                        .required(true)
                        .value_parser(value_parser!(u8).range(1..=8)),
                )
                .arg(
                    Arg::new(VID)
                        .short('v')
                        .default_value("1")
                        .value_parser(value_parser!(u8)),
                ),
        )
        .subcommand(Command::new(CAPTURE).about("Captures a screenshot of the V5 brain's screen"))
        .subcommand(
            Command::new(KERNEL_VARIABLE)
                .about("Management of kernel variables")
                .subcommand(
                    Command::new(GET)
                        .about("Gets the value of a kernel variable")
                        .arg(
                            Arg::new(VARIABLE)
                                .index(1)
                                .required(true)
                                .value_parser(["team_number", "robot_name"]),
                        ),
                )
                .subcommand(
                    Command::new(SET)
                        .about("Sets the value of a kernel variable")
                        .arg(
                            Arg::new(VARIABLE)
                                .index(1)
                                .required(true)
                                .value_parser(["team_number", "robot_name"]),
                        )
                        .arg(Arg::new(VALUE).index(2).required(true)),
                ),
        )
}

pub(crate) async fn manage(
    cmd: &mut Command,
    args: ArgMatches,
    options: RobotConnectionOptions,
) -> Result<(), CommandError> {
    if let Some((command, args)) = args.subcommand() {
        match command {
            STATUS => get_status(options).await,
            METADATA => get_metadata(options, args).await,
            LIST_FILES => list_files(options, args).await,
            STOP => stop_execution(options).await,
            RUN => execute_program(options, args).await,
            REMOVE_ALL_PROGRAMS => remove_all_programs(options, args).await,
            REMOVE_FILE => remove_file(options, args).await,
            REMOVE_PROGRAM => remove_program(options, args).await,
            KERNEL_VARIABLE => {
                kernel_variable(
                    cmd.find_subcommand_mut(KERNEL_VARIABLE)
                        .expect("get subcommand"),
                    options,
                    args,
                )
                .await
            }
            CAPTURE => capture_screen(options, args).await,
            _ => {
                cmd.print_long_help().expect("print help");
                Err(CommandError::InvalidSubcommand)
            }
        }
    } else {
        cmd.print_long_help().expect("print help");
        Err(CommandError::InvalidSubcommand)
    }
}

async fn get_status(options: RobotConnectionOptions) -> Result<(), CommandError> {
    let mut brain = v5_core::connection::connect_to_brain(options).await?;

    let status = brain.get_system_status().await?;
    println!(
        "System Version: {}\nCPU 0: {}\nCPU 1: {}\nTouch: {}\nSystem ID: {}",
        status.system, status.cpu0, status.cpu1, status.touch, status.system_id
    );
    Ok(())
}

async fn get_metadata(
    options: RobotConnectionOptions,
    args: &ArgMatches,
) -> Result<(), CommandError> {
    let mut brain = v5_core::connection::connect_to_brain(options).await?;
    let metadata = brain
        .get_file_metadata_by_name(
            Vid::from(*args.get_one::<u8>(VID).expect("missing VID")),
            FileFlags::empty(),
            args.get_one::<String>(FILE_NAME)
                .expect("missing file name!")
                .as_str(),
        )
        .await?;

    println!(
        "Name: {}\nVid: {}\nSize: {}\nAddress: {}\n CRC: {}\nFile Type: {}\nTimestamp: {}",
        metadata.name,
        metadata.vid,
        metadata.size,
        metadata.addr,
        metadata.crc,
        metadata.file_type,
        OffsetDateTime::from(metadata.timestamp)
            .format(&Rfc3339)
            .expect("parse timestamp")
    );
    Ok(())
}

async fn list_files(
    options: RobotConnectionOptions,
    args: &ArgMatches,
) -> Result<(), CommandError> {
    let mut brain = v5_core::connection::connect_to_brain(options).await?;
    let amount = brain
        .get_directory_count(
            Vid::from(*args.get_one::<u8>(VID).expect("missing VID")),
            args.get_one::<u8>(OPTION)
                .map(|b| FileFlags::from_bits_retain(*b))
                .unwrap_or(FileFlags::empty()),
        )
        .await?;

    for i in 0_u8..amount as u8 {
        let meta = brain
            .get_file_metadata_by_index(i, FileFlags::empty())
            .await?;
        println!(
            "Name: {}\nVersion: {}\nSize: {}\nAddress: {}\nCRC: {}\nFile Type: {}\nTimestamp: {}",
            meta.name,
            meta.version,
            meta.size,
            meta.addr,
            meta.crc,
            meta.file_type,
            OffsetDateTime::from(meta.timestamp)
                .format(&Rfc3339)
                .expect("parse timestamp")
        );
    }
    Ok(())
}

async fn stop_execution(options: RobotConnectionOptions) -> Result<(), CommandError> {
    let mut brain = v5_core::connection::connect_to_brain(options).await?;
    brain
        .execute_program(Vid::User, ExecutionFlags::STOP, "")
        .await?;
    Ok(())
}

async fn execute_program(
    options: RobotConnectionOptions,
    args: &ArgMatches,
) -> Result<(), CommandError> {
    let mut brain = v5_core::connection::connect_to_brain(options).await?;
    let vid = Vid::from(*args.get_one::<u8>(VID).expect("missing VID"));
    let slot = *args.get_one::<u8>(SLOT).expect("no slot provided");
    brain
        .execute_program(vid, ExecutionFlags::empty(), &format!("slot_{}.bin", slot))
        .await?;
    Ok(())
}

async fn remove_all_programs(
    options: RobotConnectionOptions,
    args: &ArgMatches,
) -> Result<(), CommandError> {
    let mut brain = v5_core::connection::connect_to_brain(options).await?;
    let vid = Vid::from(*args.get_one::<u8>(VID).expect("missing VID"));
    let c = brain.get_directory_count(vid, FileFlags::empty()).await?;
    let mut vec = Vec::new();
    vec.reserve(c as usize);
    for i in 0_u8..c as u8 {
        vec.push(
            brain
                .get_file_metadata_by_index(i, FileFlags::empty())
                .await?,
        );
    }

    for meta in vec {
        brain
            .delete_file(vid, DeleteFlags::ERASE_ALL, &meta.name)
            .await?;
    }
    Ok(())
}

async fn remove_file(
    options: RobotConnectionOptions,
    args: &ArgMatches,
) -> Result<(), CommandError> {
    let mut brain = v5_core::connection::connect_to_brain(options).await?;
    let vid = Vid::from(*args.get_one::<u8>(VID).expect("missing VID"));
    let name = args
        .get_one::<String>(FILE_NAME)
        .expect("missing name")
        .clone();
    brain.delete_file(vid, DeleteFlags::empty(), &name).await?;
    Ok(())
}

async fn remove_program(
    options: RobotConnectionOptions,
    args: &ArgMatches,
) -> Result<(), CommandError> {
    let mut brain = v5_core::connection::connect_to_brain(options).await?;
    let vid = Vid::from(*args.get_one::<u8>(VID).expect("missing VID"));
    let slot = *args.get_one::<u8>(SLOT).expect("missing slot");
    brain
        .delete_file(vid, DeleteFlags::empty(), &format!("slot_{}.bin", slot))
        .await?;
    brain
        .delete_file(vid, DeleteFlags::empty(), &format!("slot_{}.ini", slot))
        .await?;
    Ok(())
}

async fn kernel_variable(
    cmd: &mut Command,
    options: RobotConnectionOptions,
    args: &ArgMatches,
) -> Result<(), CommandError> {
    if let Some((command, args)) = args.subcommand() {
        match command {
            GET => get_kernel_variable(options, args).await,
            SET => set_kernel_variable(options, args).await,
            _ => {
                cmd.print_long_help().expect("print help");
                Err(CommandError::InvalidSubcommand)
            }
        }
    } else {
        cmd.print_long_help().expect("print help");
        Err(CommandError::InvalidSubcommand)
    }
}

async fn get_kernel_variable(
    options: RobotConnectionOptions,
    args: &ArgMatches,
) -> Result<(), CommandError> {
    let mut brain = v5_core::connection::connect_to_brain(options).await?;
    let variable = KernelVariable::try_from(
        &*args
            .get_one::<String>(VARIABLE)
            .expect("variable name")
            .clone(),
    )?;
    let value = brain.get_kernel_variable(variable).await?;
    println!("{}", value);
    Ok(())
}

async fn set_kernel_variable(
    options: RobotConnectionOptions,
    args: &ArgMatches,
) -> Result<(), CommandError> {
    let mut brain = v5_core::connection::connect_to_brain(options).await?;
    let variable = KernelVariable::try_from(
        &*args
            .get_one::<String>(VARIABLE)
            .expect("variable name")
            .clone(),
    )?;
    let value = args.get_one::<String>(VALUE).expect("variable value");
    brain.set_kernel_variable(variable, value.as_str()).await?;
    Ok(())
}

async fn capture_screen(
    options: RobotConnectionOptions,
    _args: &ArgMatches,
) -> Result<(), CommandError> {
    let _brain = v5_core::connection::connect_to_brain(options).await?;
    Ok(())
}
