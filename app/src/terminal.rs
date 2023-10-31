use clap::{ArgMatches, Command};

use v5_core::connection::RobotConnectionOptions;
use v5_core::error::CommandError;

pub(crate) const COMMAND: &str = "terminal";

pub(crate) fn command() -> Command {
    Command::new(COMMAND).about("Open serial connection to the robot")
}

pub(crate) async fn terminal(
    _cmd: &mut Command,
    _args: ArgMatches,
    _options: RobotConnectionOptions,
) -> Result<(), CommandError> {
    Ok(())
}
