use std::time::Duration;

use clap::{Arg, ArgMatches, Command, value_parser};

use v5_core::brain::competition::CompetitionState;
use v5_core::connection::RobotConnectionOptions;
use v5_core::error::CommandError;

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
    cmd: &mut Command,
    args: ArgMatches,
    options: RobotConnectionOptions,
) -> Result<(), CommandError> {
    if let Some((command, args)) = args.subcommand() {
        match command {
            START => start(options, args).await,
            AUTONOMOUS => autonomous(options, args).await,
            OPCONTROL => opcontrol(options, args).await,
            DISABLE => disable(options, args).await,
            _ => {
                cmd.print_long_help().unwrap();
                Err(CommandError::InvalidSubcommand)
            },
        }
    } else {
        cmd.print_long_help().unwrap();
        Err(CommandError::InvalidSubcommand)
    }
}

async fn autonomous(
    options: RobotConnectionOptions,
    args: &ArgMatches,
) -> Result<(), CommandError> {
    let mut brain = v5_core::connection::connect_to_brain(options).await?;
    let time = Duration::from_millis(*args.get_one::<u64>(LENGTH).expect("length"));
    brain.set_competition_state(CompetitionState::Autonomous, 0).await?;
    tokio::time::sleep(time).await;
    Ok(())
}

async fn opcontrol(options: RobotConnectionOptions, args: &ArgMatches) -> Result<(), CommandError> {
    let mut brain = v5_core::connection::connect_to_brain(options).await?;
    let time = Duration::from_millis(*args.get_one::<u64>(LENGTH).expect("length"));
    brain.set_competition_state(CompetitionState::OpControl, 0).await?;
    tokio::time::sleep(time).await;
    Ok(())
}

async fn disable(options: RobotConnectionOptions, _args: &ArgMatches) -> Result<(), CommandError> {
    let mut brain = v5_core::connection::connect_to_brain(options).await?;
    brain.set_competition_state(CompetitionState::Disabled, 0).await?;
    Ok(())
}

async fn start(options: RobotConnectionOptions, _args: &ArgMatches) -> Result<(), CommandError> {
    let _brain = v5_core::connection::connect_to_brain(options).await?;
    //todo
    Ok(())
}
