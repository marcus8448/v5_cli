use std::collections::HashMap;
use v5_core::clap::{Arg, ArgAction, ArgMatches, Command};
use v5_core::plugin::Plugin;

fn main() {
    let mut command = Command::new("robot")
        .author("marcus8448")
        .about("Manages a connection with a Vex V5 robot")
        .arg(Arg::new("port")
            .help("Name of the serial port to use")
            .short('p')
            .global(true)
            .action(ArgAction::Set)
        )
        .arg(Arg::new("verbose")
            .help("Enables extra debug logging")
            .short('v')
            .global(true)
            .action(ArgAction::SetTrue)
        );

    unsafe {
        v5_core::plugin::DEFAULT_PLUGIN_REF = Some(Box::new(register_default_plugins));
    }

    let plugins = v5_core::plugin::load_plugins();
    let mut registry = HashMap::<&'static str, fn(ArgMatches)>::new();
    for plugin in plugins {
        command = plugin.create_commands(command, &mut registry);
    }

    let matches = command.get_matches();
    match matches.subcommand() {
        None => {
            if let Ok(path) = std::env::current_exe() {
                println!("No subcommand provided!\nUse `{} --help` to see usage.", path.file_name().unwrap().to_str().unwrap());
            } else {
                println!("No subcommand provided!\nUse `<program> --help` to see usage.");
            }
        }
        Some((name, matches)) => {
            registry.get(name).unwrap()(matches.clone());
        }
    }
}

#[no_mangle]
unsafe extern fn register_default_plugins(plugins: &mut Vec<Box<dyn Plugin>>) {
    plugins.push(Box::new(v5_upload::UploadPlugin::default()));
}
