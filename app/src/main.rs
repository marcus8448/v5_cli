use std::collections::HashMap;
use v5_core::clap::{Arg, ArgAction, ArgMatches, Command};
use v5_core::plugin::Plugin;
use v5_core::serial::print_out_ports;

fn main() {
    let mut command = Command::new("robot")
        .author("marcus8448")
        .about("Manages a connection with a Vex V5 robot")
        .arg(
            Arg::new("port")
                .help("Name of the serial port to use")
                .short('p')
                .global(true)
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("verbose")
                .help("Enables extra debug logging")
                .short('v')
                .action(ArgAction::SetTrue),
        );

    print_out_ports(None);

    unsafe {
        v5_core::plugin::DEFAULT_PLUGIN_REF = Some(Box::new(register_default_plugins));
    }

    let plugins = v5_core::plugin::load_plugins();
    let mut registry =
        HashMap::<&'static str, Box<fn(ArgMatches)>>::new();
    for plugin in plugins {
        command = plugin.create_commands(command, &mut registry);
    }

    let matches = command.get_matches_mut();
    match matches.subcommand() {
        None => {
            command.print_help().unwrap();
        }
        Some((name, matches)) => {
            registry.get(name).unwrap()(matches.clone());
        }
    }
}

#[no_mangle]
unsafe extern "C" fn register_default_plugins(plugins: &mut Vec<Box<dyn Plugin>>) {
    plugins.push(Box::new(v5_upload::UploadPlugin::default()));
    plugins.push(Box::new(v5_manage::ManagePlugin::default()));
}
