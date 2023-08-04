use v5_core::clap::{Arg, ArgAction, Command};
use v5_core::plugin::Plugin;

const PORT: &'static str = "port";
const BLUETOOTH: &'static str = "bluetooth";
const MAC_ADDRESS: &'static str = "mac-address";
const PIN: &'static str = "pin";
const VERBOSE: &'static str = "verbose";

fn main() {
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
                .conflicts_with("port"),
        )
        .arg(
            Arg::new(MAC_ADDRESS)
                .help("Connect to brain via bluetooth instead of a serial port")
                .short('m')
                .action(ArgAction::Set)
                .requires("bluetooth"),
        )
        .arg(
            Arg::new(PIN)
                .help("Connect the PIN of the brain to be used with bluetooth")
                .short('i')
                .global(true)
                .action(ArgAction::Set)
                .requires("bluetooth"),
        )
        .arg(
            Arg::new(VERBOSE)
                .help("Enables extra debug logging")
                .short('v')
                .global(true)
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

    let matches = command.get_matches_mut();
    match matches.subcommand() {
        None => {
            command.print_help().unwrap();
        }
        Some((name, matches)) => {
            if matches.get_flag(BLUETOOTH) {
                let mac_address: Option<&String> = matches.get_one(MAC_ADDRESS);
                let pin: Option<&String> = matches.get_one(PIN);

                tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(async {
                        let robot =
                            v5_core::connection::bluetooth::connect_to_robot(mac_address, pin)
                                .await;
                        registry.get(name).unwrap()(matches.clone(), robot.expect("Robot"));
                    });
            } else {
                let port: Option<&String> = matches.get_one(PORT);
                registry.get(name).unwrap()(
                    matches.clone(),
                    v5_core::connection::serial::connect_to_robot(port),
                );
            }
        }
    }
}

#[no_mangle]
unsafe extern "C" fn register_default_plugins(plugins: &mut Vec<Box<dyn Plugin>>) {
    plugins.push(Box::new(v5_upload::UploadPlugin::default()));
    plugins.push(Box::new(v5_manage::ManagePlugin::default()));
}
