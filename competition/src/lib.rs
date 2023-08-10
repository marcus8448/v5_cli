use std::time::Duration;

use v5_core::clap::{Arg, ArgMatches, Command, value_parser};
use v5_core::connection::{RobotConnectionOptions, RobotConnectionType};
use v5_core::error::CommandError;
use v5_core::export_plugin;
use v5_core::packet::competition::{CompetitionState, ManageCompetition};
use v5_core::packet::Packet;
use v5_core::plugin::{CommandRegistry, Plugin};

const COMPETITION: &str = "competition";

const START: &str = "start";
const DISABLE: &str = "disable";
const AUTONOMOUS: &str = "autonomous";
const OPCONTROL: &str = "opcontrol";
const LENGTH: &str = "length";

export_plugin!(Box::new(CompetitionPlugin {}));

pub struct CompetitionPlugin {}

impl Plugin for CompetitionPlugin {
    fn get_name(&self) -> &'static str {
        COMPETITION
    }

    fn create_commands(&self, command: Command, registry: &mut CommandRegistry) -> Command {
        registry.insert(
            COMPETITION,
            Box::new(move |args, connection| Box::pin(competition(args, connection))),
        );
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

async fn competition(
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
    let mut brain = v5_core::connection::connect(RobotConnectionType::System, options).await?;
    let time = Duration::from_millis(*args.get_one::<u64>(LENGTH).expect("length"));
    ManageCompetition::new(CompetitionState::Autonomous)
        .send(&mut brain)
        .await?;
    std::thread::sleep(time);
    Ok(())
}

async fn opcontrol(options: RobotConnectionOptions, args: &ArgMatches) -> Result<(), CommandError> {
    let mut brain = v5_core::connection::connect(RobotConnectionType::System, options).await?;
    let time = Duration::from_millis(*args.get_one::<u64>(LENGTH).expect("length"));
    ManageCompetition::new(CompetitionState::OpControl)
        .send(&mut brain)
        .await?;
    std::thread::sleep(time);
    Ok(())
}

async fn disable(options: RobotConnectionOptions, _args: &ArgMatches) -> Result<(), CommandError> {
    let mut brain = v5_core::connection::connect(RobotConnectionType::System, options).await?;
    ManageCompetition::new(CompetitionState::Disabled)
        .send(&mut brain)
        .await?;
    Ok(())
}

async fn start(options: RobotConnectionOptions, _args: &ArgMatches) -> Result<(), CommandError> {
    let _brain = v5_core::connection::connect(RobotConnectionType::System, options).await?;
    //todo
    Ok(())
}
