use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use v5_core::clap::builder::NonEmptyStringValueParser;
use v5_core::clap::{value_parser, Arg, ArgMatches, Command};
use v5_core::error::Error;
use v5_core::log::error;
use v5_core::plugin::{Plugin, PORT};
use v5_core::serial::system::{Brain, KernelVariable, Vid};

type Result<T> = std::result::Result<T, Error>;

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

impl ManagePlugin {
    pub fn default() -> Self {
        ManagePlugin {}
    }
}

impl Plugin for ManagePlugin {
    fn get_name(&self) -> &'static str {
        MANAGE
    }

    fn create_commands(
        &self,
        command: Command,
        registry: &mut HashMap<
            &'static str,
            Box<fn(ArgMatches) -> Pin<Box<dyn Future<Output = ()>>>>,
        >,
    ) -> Command {
        registry.insert(MANAGE, Box::new(|f| Box::pin(manage(f))));
        command.subcommand(
            Command::new(MANAGE)
                .about("Manage the robot brain")
                .help_expected(true)
                .subcommand(Command::new(STATUS).about("Get the status of the robot brain"))
                .subcommand(
                    Command::new(METADATA)
                        .about("Reads file metadata")
                        .arg(
                            Arg::new(FILE_NAME)
                                .index(1)
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
                .subcommand(Command::new(LIST_FILES).about("Lists all files on the brain"))
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
                        .help_expected(true)
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

async fn manage(args: ArgMatches) {
    let mut brain =
        v5_core::serial::connect_to_brain(args.get_one(PORT).map(|f: &String| f.to_string()));
    if let Some((command, args)) = args.subcommand() {
        match command {
            STATUS => get_status(brain, args).await,
            METADATA => get_metadata(brain, args).await,
            LIST_FILES => list_files(brain, args).await,
            STOP => stop_execution(brain, args).await,
            RUN => execute_program(brain, args).await,
            REMOVE_ALL_PROGRAMS => remove_all_programs(brain, args).await,
            REMOVE_FILE => remove_file(brain, args).await,
            REMOVE_PROGRAM => remove_program(brain, args).await,
            KERNEL_VARIABLE => kernel_variable(brain, args).await,
            CAPTURE => capture_screen(brain, args).await,
            _ => Err(Error::Generic("Invalid subcommand! (see `--help`)")),
        }
        .unwrap()
    } else {
        error!("Missing subcommand (see `--help`)");
    }
}

async fn get_status(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    let status = brain.get_system_status()?;
    println!(
        "System Version: {}\nCPU 0: {}\nCPU 1: {}\nTouch: {}\nSystem ID: {}",
        status.get_system_version(),
        status.get_cpu0_version(),
        status.get_cpu1_version(),
        status.get_touch_version(),
        status.get_system_id()
    );
    Ok(())
}

async fn get_metadata(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    let metadata = brain.read_file_metadata(
        args.get_one::<String>(FILE_NAME)
            .expect("missing file name!")
            .as_str(),
        Vid::from(*args.get_one::<u8>(VID).expect("missing VID")),
    )?;
    println!(
        "Name: {}\nVid: {}\nSize: {}\nAddress: {}\n CRC: {}\nFile Type: {}\nTimestamp: {}",
        metadata.name,
        metadata.vid,
        metadata.size,
        metadata.addr,
        metadata.crc,
        metadata.file_type,
        metadata.timestamp.format("%+")
    );
    Ok(())
}

async fn list_files(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    let amount = brain.get_directory_count(
        Vid::from(*args.get_one::<u8>(VID).expect("missing VID")),
        *args.get_one::<u8>(OPTION).unwrap_or(&0),
    )?;
    for i in 0..amount {
        println!("{}\n--", brain.get_file_metadata_by_index(i as u8, 0)?);
    }
    Ok(())
}

async fn stop_execution(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    brain.execute_program(Vid::User, 0, "")?;
    Ok(())
}

async fn execute_program(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    let vid = Vid::from(*args.get_one::<u8>(VID).expect("missing VID"));
    let slot = *args.get_one::<u8>(SLOT).expect("no slot provided");
    brain.execute_program(vid, 0x80, format!("slot_{}.bin", slot).as_str())?;
    Ok(())
}

async fn remove_all_programs(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    let vid = Vid::from(*args.get_one::<u8>(VID).expect("missing VID"));
    let c = brain.get_directory_count(vid, 0)?;
    let mut vec = Vec::new();
    vec.reserve(c as usize);
    for i in 0..c {
        vec.push(brain.get_file_metadata_by_index(i as u8, 0)?.name);
    }
    for name in vec {
        brain.delete_file(vid, 0, &name)?;
    }
    Ok(())
}

async fn remove_file(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    let vid = Vid::from(*args.get_one::<u8>(VID).expect("missing VID"));
    let name = args
        .get_one::<String>(FILE_NAME)
        .expect("missing name")
        .clone();
    brain.delete_file(vid, 0, &name)?;
    Ok(())
}

async fn remove_program(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    let vid = Vid::from(*args.get_one::<u8>(VID).expect("missing VID"));
    let slot = *args.get_one::<u8>(SLOT).expect("missing slot");
    brain.delete_file(vid, 0, &format!("slot_{}.bin", slot))?;
    brain.delete_file(vid, 0, &format!("slot_{}.ini", slot))?;
    Ok(())
}

async fn kernel_variable(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    if let Some((command, args)) = args.subcommand() {
        match command {
            GET => get_kernel_variable(brain, args).await,
            SET => set_kernel_variable(brain, args).await,
            _ => Err(Error::Generic("Invalid subcommand! (see `--help`)")),
        }
    } else {
        Err(Error::Generic("Missing subcommand (see `--help`)"))
    }
}

async fn get_kernel_variable(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    let variable = KernelVariable::try_from(args.get_one::<String>(VARIABLE).unwrap().clone())?;
    let value = brain.get_kernel_variable(variable)?;
    println!("{}", value);
    Ok(())
}

async fn set_kernel_variable(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    let variable = KernelVariable::try_from(args.get_one::<String>(VARIABLE).unwrap().clone())?;
    let value = args.get_one::<String>(VALUE).unwrap();
    let actual_value = brain.set_kernel_variable(variable, value.as_str())?;

    println!("{}", actual_value);
    Ok(())
}

async fn capture_screen(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    Ok(())
}
