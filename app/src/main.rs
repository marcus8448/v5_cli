use std::sync::OnceLock;

use v5_core::clap::{Arg, ArgAction, Command};
use v5_core::connection::RobotConnectionOptions;
use v5_core::error::CommandError;
use v5_core::plugin::Plugin;

const PORT: &str = "port";
const BLUETOOTH: &str = "bluetooth";
const MAC_ADDRESS: &str = "mac-address";
const PIN: &str = "pin";
const VERBOSE: &str = "verbose";

pub static BASE_COMMAND: OnceLock<Command> = OnceLock::new();

fn main() {
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
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new(BLUETOOTH)
                .help("Connect to brain via bluetooth instead of a serial port")
                .short('b')
                .action(ArgAction::SetTrue)
                .conflicts_with(PORT),
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
            Arg::new(VERBOSE)
                .help("Enables extra debug logging")
                .short('v')
                .global(false)
                .action(ArgAction::SetTrue),
        );

    unsafe {
        v5_core::plugin::DEFAULT_PLUGIN_REF = Some(Box::new(register_default_plugins));
    }

    let plugins = v5_core::plugin::load_plugins();
    let mut registry = v5_core::plugin::CommandRegistry::new();
    for plugin in plugins {
        command = plugin.create_commands(command, &mut registry);
    }

    BASE_COMMAND.set(command.clone()).unwrap();
    let root = command.get_matches_mut();
    match root.subcommand() {
        None => {
            command.print_help().unwrap();
        }
        Some((name, matches)) => {
            let options = if root.get_flag(BLUETOOTH) {
                let mac_address: Option<&String> = root.get_one(MAC_ADDRESS);
                let pin: Option<&String> = root.get_one(PIN);

                RobotConnectionOptions::Bluetooth {
                    mac_address: mac_address.cloned(),
                    pin: pin.cloned(),
                }
            } else {
                let port: Option<&String> = root.get_one(PORT);

                RobotConnectionOptions::Serial {
                    port: port.cloned(),
                }
            };

            match registry.get(name).unwrap()(matches.clone(), options).await {
                Ok(_) => {}
                Err(err) => match err {
                    CommandError::InvalidSubcommand => {
                        command.print_help().unwrap();
                    }
                    _ => {
                        println!("{}", err);
                    }
                },
            };
        }
    }
}

#[no_mangle]
unsafe extern "C" fn register_default_plugins(plugins: &mut Vec<Box<dyn Plugin>>) {
    plugins.push(Box::new(v5_upload::UploadPlugin {}));
    plugins.push(Box::new(v5_manage::ManagePlugin {}));
}
