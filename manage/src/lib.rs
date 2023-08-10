use v5_core::clap::{Arg, ArgMatches, Command, value_parser};
use v5_core::clap::builder::NonEmptyStringValueParser;
use v5_core::connection::{RobotConnectionOptions, RobotConnectionType};
use v5_core::error::CommandError;
use v5_core::packet::filesystem::{
    DeleteFile, GetDirectoryCount, GetFileMetadataByIndex, GetFileMetadataByName, Vid,
};
use v5_core::packet::Packet;
use v5_core::packet::system::{
    ExecuteProgram, GetKernelVariable, GetSystemStatus, KernelVariable, SetKernelVariable,
};
use v5_core::plugin::{CommandRegistry, Plugin};
use v5_core::time::format_description::well_known::Rfc3339;
use v5_core::time::OffsetDateTime;

type Result<T> = std::result::Result<T, CommandError>;

// export_plugin!(Box::new(UploadPlugin::default()));

const MANAGE: &str = "manage";

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
const KERNEL_VARIABLE: &str = "var";
const SET: &str = "set";
const GET: &str = "get";
const VARIABLE: &str = "variable";
const VALUE: &str = "value";
const CAPTURE: &str = "capture";

pub struct ManagePlugin {}

impl Plugin for ManagePlugin {
    fn get_name(&self) -> &'static str {
        MANAGE
    }

    fn create_commands(&self, command: Command, registry: &mut CommandRegistry) -> Command {
        registry.insert(
            MANAGE,
            Box::new(move |args, connection| Box::pin(manage(args, connection))),
        );
        command.subcommand(
            Command::new(MANAGE)
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
                .subcommand(
                    Command::new(CAPTURE).about("Captures a screenshot of the V5 brain's screen"),
                )
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
                ),
        )
    }
}

async fn manage(args: ArgMatches, options: RobotConnectionOptions) -> Result<()> {
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
            KERNEL_VARIABLE => kernel_variable(options, args).await,
            CAPTURE => capture_screen(options, args).await,
            _ => Err(CommandError::InvalidSubcommand),
        }
    } else {
        Err(CommandError::InvalidSubcommand)
    }
}

async fn get_status(options: RobotConnectionOptions) -> Result<()> {
    let mut brain = v5_core::connection::connect(RobotConnectionType::System, options).await?;
    let status = GetSystemStatus::new().send(&mut brain).await?;
    println!(
        "System Version: {}\nCPU 0: {}\nCPU 1: {}\nTouch: {}\nSystem ID: {}",
        status.system, status.cpu0, status.cpu1, status.touch, status.system_id
    );
    Ok(())
}

async fn get_metadata(options: RobotConnectionOptions, args: &ArgMatches) -> Result<()> {
    let mut brain = v5_core::connection::connect(RobotConnectionType::System, options).await?;
    let metadata = GetFileMetadataByName::new(
        Vid::from(*args.get_one::<u8>(VID).expect("missing VID")),
        0,
        args.get_one::<String>(FILE_NAME)
            .expect("missing file name!")
            .as_str(),
    )
    .send(&mut brain)
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
            .unwrap()
    );
    Ok(())
}

async fn list_files(options: RobotConnectionOptions, args: &ArgMatches) -> Result<()> {
    let mut brain = v5_core::connection::connect(RobotConnectionType::System, options).await?;
    let amount = GetDirectoryCount::new(
        Vid::from(*args.get_one::<u8>(VID).expect("missing VID")),
        *args.get_one::<u8>(OPTION).unwrap_or(&0),
    )
    .send(&mut brain)
    .await?;

    for i in 0..amount {
        println!(
            "{}\n--",
            GetFileMetadataByIndex::new(i as u8, 0)
                .send(&mut brain)
                .await?
        );
    }
    Ok(())
}

async fn stop_execution(options: RobotConnectionOptions) -> Result<()> {
    let mut brain = v5_core::connection::connect(RobotConnectionType::System, options).await?;
    ExecuteProgram::new(Vid::User, 0x80, "")
        .send(&mut brain)
        .await?;
    Ok(())
}

async fn execute_program(options: RobotConnectionOptions, args: &ArgMatches) -> Result<()> {
    let mut brain = v5_core::connection::connect(RobotConnectionType::System, options).await?;
    let vid = Vid::from(*args.get_one::<u8>(VID).expect("missing VID"));
    let slot = *args.get_one::<u8>(SLOT).expect("no slot provided");
    ExecuteProgram::new(vid, 0x00, format!("slot_{}.bin", slot).as_str())
        .send(&mut brain)
        .await?;
    Ok(())
}

async fn remove_all_programs(options: RobotConnectionOptions, args: &ArgMatches) -> Result<()> {
    let mut brain = v5_core::connection::connect(RobotConnectionType::System, options).await?;
    let vid = Vid::from(*args.get_one::<u8>(VID).expect("missing VID"));
    let c = GetDirectoryCount::new(vid, 0).send(&mut brain).await?;
    let mut vec = Vec::new();
    vec.reserve(c as usize);
    for i in 0..c {
        vec.push(
            GetFileMetadataByIndex::new(i as u8, 0)
                .send(&mut brain)
                .await?,
        );
    }

    for meta in vec {
        DeleteFile::new(vid, true, &meta.name)
            .send(&mut brain)
            .await?;
    }
    Ok(())
}

async fn remove_file(options: RobotConnectionOptions, args: &ArgMatches) -> Result<()> {
    let mut brain = v5_core::connection::connect(RobotConnectionType::System, options).await?;
    let vid = Vid::from(*args.get_one::<u8>(VID).expect("missing VID"));
    let name = args
        .get_one::<String>(FILE_NAME)
        .expect("missing name")
        .clone();
    DeleteFile::new(vid, false, &name).send(&mut brain).await?;
    Ok(())
}

async fn remove_program(options: RobotConnectionOptions, args: &ArgMatches) -> Result<()> {
    let mut brain = v5_core::connection::connect(RobotConnectionType::System, options).await?;
    let vid = Vid::from(*args.get_one::<u8>(VID).expect("missing VID"));
    let slot = *args.get_one::<u8>(SLOT).expect("missing slot");
    DeleteFile::new(vid, false, &format!("slot_{}.bin", slot))
        .send(&mut brain)
        .await?;
    DeleteFile::new(vid, false, &format!("slot_{}.ini", slot))
        .send(&mut brain)
        .await?;
    Ok(())
}

async fn kernel_variable(options: RobotConnectionOptions, args: &ArgMatches) -> Result<()> {
    if let Some((command, args)) = args.subcommand() {
        match command {
            GET => get_kernel_variable(options, args).await,
            SET => set_kernel_variable(options, args).await,
            _ => return Err(CommandError::InvalidSubcommand),
        }
    } else {
        return Err(CommandError::InvalidSubcommand);
    }
}

async fn get_kernel_variable(options: RobotConnectionOptions, args: &ArgMatches) -> Result<()> {
    let mut brain = v5_core::connection::connect(RobotConnectionType::System, options).await?;
    let variable = KernelVariable::try_from(&*args.get_one::<String>(VARIABLE).unwrap().clone())?;
    let value = GetKernelVariable::new(variable).send(&mut brain).await?;
    println!("{}", value);
    Ok(())
}

async fn set_kernel_variable(options: RobotConnectionOptions, args: &ArgMatches) -> Result<()> {
    let mut brain = v5_core::connection::connect(RobotConnectionType::System, options).await?;
    let variable = KernelVariable::try_from(&*args.get_one::<String>(VARIABLE).unwrap().clone())?;
    let value = args.get_one::<String>(VALUE).unwrap();
    SetKernelVariable::new(variable, value.as_str())
        .send(&mut brain)
        .await?;

    println!("set");
    Ok(())
}

async fn capture_screen(options: RobotConnectionOptions, _args: &ArgMatches) -> Result<()> {
    let _brain = v5_core::connection::connect(RobotConnectionType::System, options).await?;
    Ok(())
}
