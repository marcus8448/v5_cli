use std::collections::HashMap;
use std::future::Future;
use std::ops::Index;
use std::pin::Pin;
use v5_core::clap::{Arg, ArgMatches, Command, value_parser};
use v5_core::clap::builder::{NonEmptyStringValueParser};
use v5_core::log::{error, info};
use v5_core::plugin::{Plugin, PORT};
use v5_core::serial::system::{Brain, KernelVariable};
use v5_core::error::Error;

type Result<T> = std::result::Result<T, Error>;

// export_plugin!(Box::new(UploadPlugin::default()));

const MANAGE: &str = "manage";

const STATUS: &str = "status";
const METADATA: &str = "metadata";
const LIST_FILES: &str = "ls_files";
const FILE_NAME: &str = "file_name";
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

pub struct ManagePlugin {
}

impl ManagePlugin {
    pub fn default() -> Self {
        ManagePlugin {}
    }
}

impl Plugin for ManagePlugin {
    fn get_name(&self) -> &'static str {
        MANAGE
    }

    fn create_commands(&self, command: Command, registry: &mut HashMap<&'static str, Box<fn(ArgMatches) -> Pin<Box<dyn Future<Output = ()>>>>>) -> Command {
        registry.insert(MANAGE, Box::new(|f| Box::pin(manage(f))));
        command.subcommand(Command::new(MANAGE)
            .about("Manage the robot brain")
            .subcommand(Command::new(STATUS))
            .subcommand(Command::new(METADATA)
                .arg(Arg::new(FILE_NAME)
                    .index(1)
                    .value_parser(NonEmptyStringValueParser::new())
                )
            )
            .subcommand(Command::new(LIST_FILES))
            .subcommand(Command::new(STOP))
            .subcommand(Command::new(RUN)
                .arg(Arg::new(SLOT)
                    .index(1)
                    .value_parser(value_parser!(u8).range(1..=8))
                )
            )
            .subcommand(Command::new(REMOVE_ALL_PROGRAMS))
            .subcommand(Command::new(REMOVE_FILE)
                .arg(Arg::new(FILE_NAME)
                    .index(1)
                    .value_parser(NonEmptyStringValueParser::new())
                )
            )
            .subcommand(Command::new(REMOVE_PROGRAM)
                .arg(Arg::new(SLOT)
                    .index(1)
                    .value_parser(value_parser!(u8).range(1..=8))
                )
            )
            .subcommand(Command::new(CAPTURE))
            .subcommand(Command::new(KERNEL_VARIABLE)
                .subcommand(Command::new(GET)
                    .arg(Arg::new(VARIABLE)
                        .index(1)
                        .value_parser(["team_number", "robot_name"])
                    )
                )
                .subcommand(Command::new(SET)
                    .arg(Arg::new(VARIABLE)
                        .index(1)
                        .value_parser(["team_number", "robot_name"])
                    )
                    .arg(Arg::new(VALUE)
                        .index(2)
                    )
                )
            )
        )
    }
}

async fn manage(args: ArgMatches) {
    let mut brain = v5_core::serial::connect_to_brain(args.get_one(PORT).map(|f: &String| f.to_string()));
    if let Some((command, args)) = args.subcommand() {
        match command {
            STATUS => get_status(brain, args),
            METADATA => get_metadata(brain, args),
            LIST_FILES => list_files(brain, args),
            STOP => stop_execution(brain, args),
            RUN => execute_program(brain, args),
            REMOVE_ALL_PROGRAMS => remove_all_programs(brain, args),
            REMOVE_FILE => remove_file(brain, args),
            REMOVE_PROGRAM => remove_program(brain, args),
            KERNEL_VARIABLE => kernel_variable(brain, args),
            CAPTURE => capture_screen(brain, args),
            _ => Err(Error::Generic("Invalid subcommand! (see `--help`)"))
        }.unwrap()
    } else {
        error!("Missing subcommand (see `--help`)");
    }
}

fn get_status(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    Ok(())
}

fn get_metadata(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    Ok(())
}

fn list_files(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    Ok(())
}

fn stop_execution(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    Ok(())
}

fn execute_program(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    Ok(())
}

fn remove_all_programs(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    Ok(())
}

fn remove_file(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    Ok(())
}

fn remove_program(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    Ok(())
}

fn kernel_variable(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    if let Some((command, args)) = args.subcommand() {
        match command {
            GET => get_kernel_variable(brain, args),
            SET => set_kernel_variable(brain, args),
            _ => {
                Err(Error::Generic("Invalid subcommand! (see `--help`)"))
            }
        }
    } else {
        Err(Error::Generic("Missing subcommand (see `--help`)"))
    }
}

fn get_kernel_variable(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    let variable = KernelVariable::try_from(args.get_one::<String>(VARIABLE).unwrap().clone())?;
    let value = brain.get_kernel_variable(variable)?;
    info!("{}", value);
    Ok(())
}

fn set_kernel_variable(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    let variable = KernelVariable::try_from(args.get_one::<String>(VARIABLE).unwrap().clone())?;
    let value = args.get_one::<String>(VALUE).unwrap();
    let actual_value = brain.set_kernel_variable(variable, value.as_str())?;

    info!("{}", actual_value);
    Ok(())
}

fn capture_screen(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    Ok(())
}
