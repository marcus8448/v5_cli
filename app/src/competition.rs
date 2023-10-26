use std::time::Duration;

use v5_core::clap::{Arg, ArgMatches, Command, value_parser};
use v5_core::connection::{RobotConnectionOptions};
use v5_core::error::CommandError;
use v5_core::packet::competition::{CompetitionState, ManageCompetition};

pub(crate) const COMMAND: &str = "competition";

const START: &str = "start";
const DISABLE: &str = "disable";
const AUTONOMOUS: &str = "autonomous";
const OPCONTROL: &str = "opcontrol";
const LENGTH: &str = "length";

pub(crate) fn command() -> Command {
    Command::new(COMMAND)
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
        .subcommand(Command::new(DISABLE).about("Disables the robot"))
}

pub(crate) async fn competition(
    args: ArgMatches,
    options: RobotConnectionOptions,
) -> Result<(), CommandError> {
    if let Some((command, args)) = args.subcommand() {
        match command {
            START => start(options, args).await,
            AUTONOMOUS => autonomous(options, args).await,
            OPCONTROL => opcontrol(options, args).await,
            DISABLE => disable(options, args).await,
            _ => Err(CommandError::InvalidSubcommand),
        }
    } else {
        Err(CommandError::InvalidSubcommand)
    }
}

async fn autonomous(
    options: RobotConnectionOptions,
    args: &ArgMatches,
) -> Result<(), CommandError> {
    let mut brain = v5_core::connection::connect_to_brain(options).await?;
    let time = Duration::from_millis(*args.get_one::<u64>(LENGTH).expect("length"));
    brain.send(&mut ManageCompetition::new(CompetitionState::Autonomous)).await?;
    tokio::time::sleep(time).await;
    Ok(())
}

async fn opcontrol(options: RobotConnectionOptions, args: &ArgMatches) -> Result<(), CommandError> {
    let mut brain = v5_core::connection::connect_to_brain(options).await?;
    let time = Duration::from_millis(*args.get_one::<u64>(LENGTH).expect("length"));
    brain.send(&mut ManageCompetition::new(CompetitionState::OpControl)).await?;
    tokio::time::sleep(time).await;
    Ok(())
}

async fn disable(options: RobotConnectionOptions, _args: &ArgMatches) -> Result<(), CommandError> {
    let mut brain = v5_core::connection::connect_to_brain(options).await?;
    brain.send(&mut ManageCompetition::new(CompetitionState::Disabled)).await?;
    Ok(())
}

async fn start(options: RobotConnectionOptions, _args: &ArgMatches) -> Result<(), CommandError> {
    let _brain = v5_core::connection::connect_to_brain(options).await?;
    //todo
    Ok(())
}
