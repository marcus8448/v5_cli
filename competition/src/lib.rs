use std::collections::HashMap;
use std::time::{Duration};
use v5_core::clap::{value_parser, Arg, ArgMatches, Command};
use v5_core::error::Error;
use v5_core::export_plugin;
use v5_core::log::error;
use v5_core::plugin::{Plugin, PORT};
use v5_core::serial::system::{Brain, CompetitionStatus};

type Result<T> = std::result::Result<T, Error>;

const COMPETITION: &str = "competition";

const START: &str = "status";
const DISABLE: &str = "metadata";
const AUTONOMOUS: &str = "ls_files";
const OPCONTROL: &str = "file_name";
const LENGTH: &str = "vid";

export_plugin!(Box::new(CompetitionPlugin::default()));

pub struct CompetitionPlugin {}

impl Default for CompetitionPlugin {
    fn default() -> Self {
        CompetitionPlugin {}
    }
}

impl Plugin for CompetitionPlugin {
    fn get_name(&self) -> &'static str {
        COMPETITION
    }

    fn create_commands(
        &self,
        command: Command,
        registry: &mut HashMap<
            &'static str,
            Box<fn(ArgMatches)>,
        >,
    ) -> Command {
        registry.insert(COMPETITION, Box::new(competition));
        command.subcommand(
            Command::new(COMPETITION)
                .about("Simulate a competition")
                .subcommand(Command::new(START).about("Starts an interactive competition manager"))
                .subcommand(
                    Command::new(AUTONOMOUS)
                        .about("Runs the autonomous period, then disables the robot")
                        .arg(
                            Arg::new(LENGTH)
                                .short('l')
                                .default_value("15000")
                                .value_parser(value_parser!(u64)),
                        ),
                )
                .subcommand(
                    Command::new(OPCONTROL)
                        .about("Runs the operator control period, then disables the robot")
                        .arg(
                            Arg::new(LENGTH)
                                .short('l')
                                .default_value("105000")
                                .value_parser(value_parser!(u64)),
                        ),
                )
                .subcommand(Command::new(DISABLE).about("Disables the robot")),
        )
    }
}

fn competition(args: ArgMatches) {
    let brain =
        v5_core::serial::connect_to_brain(args.get_one(PORT).map(|f: &String| f.to_string()));
    if let Some((command, args)) = args.subcommand() {
        match command {
            START => start(brain, args),
            AUTONOMOUS => autonomous(brain, args),
            OPCONTROL => opcontrol(brain, args),
            DISABLE => disable(brain, args),
            _ => Err(Error::Generic("Invalid subcommand! (see `--help`)")),
        }
        .unwrap()
    } else {
        error!("Missing subcommand (see `--help`)");
    }
}

fn autonomous(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    let time = Duration::from_millis(*args.get_one::<u64>(LENGTH).expect("length"));
    brain.manage_competition(CompetitionStatus::Autonomous)?;
    std::thread::sleep(time);
    Ok(())
}

fn opcontrol(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    let time = Duration::from_millis(*args.get_one::<u64>(LENGTH).expect("length"));
    brain.manage_competition(CompetitionStatus::OpControl)?;
    std::thread::sleep(time);
    Ok(())
}

fn disable(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    brain.manage_competition(CompetitionStatus::Disabled)?;
    Ok(())
}

fn start(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    //todo
    return Ok(())
}
