use clap::{Arg, ArgAction, ArgMatches, Command};
use corncobs::CobsError;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use v5_serial::connection::RobotConnectionOptions;
use v5_serial::error::{CommandError, CommunicationError};

pub(crate) const COMMAND: &str = "terminal";
const RAW_MODE: &str = "raw";

pub(crate) fn command() -> Command {
    Command::new(COMMAND)
        .about("Open serial connection to the robot")
        .arg(Arg::new(RAW_MODE)
            .help("Disables COBS encoding")
            .short('r')
            .action(ArgAction::SetTrue))
}

pub(crate) async fn terminal(
    _cmd: &mut Command,
    args: ArgMatches,
    options: RobotConnectionOptions,
) -> Result<(), CommandError> {
    let raw = args.get_flag(RAW_MODE);
    let mut brain = v5_serial::connection::connect_to_brain(options).await?;
    println!("Connected to brain");

    let mut stdin = tokio::io::stdin();
    loop {
        let mut in_buf = [0_u8; 256];
        let mut buffer = vec![0_u8; 256];
        let mut start = 0;
        tokio::select! {
            read = stdin.read(&mut in_buf) => {
                let read = read?;
                brain.connection.write_serial(&in_buf[..read]).await?;
            }
            read = brain.connection.read_serial(&mut buffer[start..]) => {
                let read = read?;
                if raw {
                    tokio::io::stdout().write_all(&buffer[..read]).await?;
                } else {
                    match corncobs::decode_buf(&buffer[..start + read], &mut in_buf) {
                        Ok(len) => {
                            if &in_buf[..4] == b"sout" {
                                tokio::io::stdout().write_all(&in_buf[4..len]).await?
                            } else if &in_buf[..4] == b"serr" {
                                tokio::io::stderr().write_all(&in_buf[4..len]).await?
                            } else {
                                tokio::io::stdout().write_all(&in_buf[..len]).await?
                            }
                        }
                        Err(err) => match err {
                            CobsError::Truncated => {
                                buffer.resize(buffer.len() + 256, 0);
                                start = buffer.len();
                            }
                            CobsError::Corrupt => return Err(CommandError::CommunicationError(CommunicationError::Eof)),
                        }
                    }
                }
            }
        }
    }
}
