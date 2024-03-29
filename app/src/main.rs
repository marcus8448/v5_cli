use clap::{Arg, ArgAction, Command, value_parser};

use v5_serial::connection::RobotConnectionOptions;

mod competition;
mod daemon;
mod manage;
mod terminal;
mod upload;

const PORT: &str = "port";
const BLUETOOTH: &str = "bluetooth";
const DAEMON: &str = "daemon";
const DAEMON_PORT: &str = "daemon-port";
const MAC_ADDRESS: &str = "mac-address";
const PIN: &str = "pin";
const VERBOSE: &str = "verbose";

fn main() {
    env_logger::init();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to build runtime")
        .block_on(run())
}

async fn run() {
    let mut command = Command::new("robot")
        .author("marcus8448")
        .about("Manages a connection with a Vex V5 robot")
        .arg(
            Arg::new(PORT)
                .help("Name of the serial port to use")
                .short('p')
                .action(ArgAction::Set)
                .conflicts_with(BLUETOOTH)
                .conflicts_with(DAEMON),
        )
        .arg(
            Arg::new(BLUETOOTH)
                .help("Connect to brain via bluetooth instead of a serial port")
                .short('b')
                .action(ArgAction::SetTrue)
                .conflicts_with(PORT)
                .conflicts_with(DAEMON),
        )
        .arg(
            Arg::new(MAC_ADDRESS)
                .help("The MAC address of the brain to be used with bluetooth")
                .short('m')
                .action(ArgAction::Set)
                .requires(BLUETOOTH),
        )
        .arg(
            Arg::new(PIN)
                .help("The PIN of the brain to be used with bluetooth")
                .short('i')
                .action(ArgAction::Set)
                .requires(BLUETOOTH),
        )
        .arg(
            Arg::new(DAEMON)
                .help("Use a shared robot backend")
                .short('d')
                .action(ArgAction::SetTrue)
                .conflicts_with(BLUETOOTH)
                .conflicts_with(PORT),
        )
        .arg(
            Arg::new(DAEMON_PORT)
                .help("Daemon port number")
                .short('s')
                .default_value("5735")
                .value_parser(value_parser!(u16))
                .action(ArgAction::Set)
                .requires(DAEMON),
        )
        .arg(
            Arg::new(VERBOSE)
                .help("Enables extra debug logging")
                .short('v')
                .global(false)
                .action(ArgAction::SetTrue),
        )
        .subcommand(competition::command())
        .subcommand(manage::command())
        .subcommand(terminal::command())
        .subcommand(upload::command())
        .subcommand(daemon::command());
    command.build();

    let root = command.get_matches_mut();
    match root.subcommand() {
        None => {
            command.print_help().expect("failed to print help");
        }
        Some((name, matches)) => {
            let options = if root.get_flag(BLUETOOTH) {
                let mac_address: Option<&String> = root.get_one(MAC_ADDRESS);
                let pin: Option<&String> = root.get_one(PIN);

                RobotConnectionOptions::Bluetooth {
                    mac_address: mac_address.cloned(),
                    pin: pin.cloned(),
                }
            } else if root.get_flag(DAEMON) {
                RobotConnectionOptions::Daemon {
                    port: *root.get_one(DAEMON_PORT).expect("missing daemon port"),
                }
            } else {
                let port: Option<&String> = root.get_one(PORT);

                RobotConnectionOptions::Serial {
                    port: port.cloned(),
                }
            };

            match match name {
                competition::COMMAND => {
                    competition::competition(
                        command.find_subcommand_mut(name).expect("get subcommand"),
                        matches.clone(),
                        options,
                    )
                    .await
                }
                manage::COMMAND => {
                    manage::manage(
                        command.find_subcommand_mut(name).expect("get subcommand"),
                        matches.clone(),
                        options,
                    )
                    .await
                }
                terminal::COMMAND => {
                    terminal::terminal(
                        command.find_subcommand_mut(name).expect("get subcommand"),
                        matches.clone(),
                        options,
                    )
                    .await
                }
                upload::COMMAND => {
                    upload::upload(
                        command.find_subcommand_mut(name).expect("get subcommand"),
                        matches.clone(),
                        options,
                    )
                    .await
                }
                daemon::COMMAND => {
                    daemon::daemon(
                        command.find_subcommand_mut(name).expect("get subcommand"),
                        matches.clone(),
                        options,
                    )
                    .await
                }
                &_ => {
                    command.print_help().expect("print help");
                    return;
                }
            } {
                Ok(_) => {}
                Err(err) => println!("{}", err),
            };
        }
    }
}
