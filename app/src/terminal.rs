use clap::{ArgMatches, Command};

use v5_core::connection::RobotConnectionOptions;
use v5_core::error::CommandError;

type Result<T> = std::result::Result<T, CommandError>;

pub(crate) const COMMAND: &str = "terminal";

pub(crate) fn command() -> Command {
    Command::new(COMMAND)
        .about("Open serial connection to the robot")
}

pub(crate) async fn terminal(_cmd: &mut Command, _args: ArgMatches, _options: RobotConnectionOptions) -> Result<()> {
    Ok(())
}
