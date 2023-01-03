use v5_core::clap::{Arg, ArgAction, Command};
use v5_core::log::error;
use v5_core::plugin::{CommandRegistry, CustomDataRegistry, Plugin};
use v5_core::tokio;

fn main() {
    env_logger::init();

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
                .global(true)
                .action(ArgAction::SetTrue),
        );

    unsafe {
        v5_core::plugin::DEFAULT_PLUGIN_REF = Some(Box::new(register_default_plugins));
    }

    let plugins = v5_core::plugin::load_plugins();
    let mut registry = CommandRegistry::new();
    for plugin in &plugins {
        if let Some(cmd) = plugin.create_commands(&mut registry) {
            assert!(registry.contains_key(&cmd.get_name()));
            command = command.subcommand(cmd);
        }
    }
    let mut data_registry = CustomDataRegistry::new();
    for plugin in &plugins {
        plugin.register_custom(&mut data_registry);
    }
    for plugin in &plugins {
        data_registry = plugin.take_custom(data_registry);
    }

    let matches = command.get_matches();
    match matches.subcommand() {
        None => {
            if let Ok(path) = std::env::current_exe() {
                error!(
                    "No subcommand provided!\nUse `{} --help` to see usage.",
                    path.file_name().unwrap().to_str().unwrap()
                );
            } else {
                error!("No subcommand provided!\nUse `<program> --help` to see usage.");
            }
        }
        Some((name, matches)) => {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(registry.get(name).unwrap()(matches.clone()));
        }
    }
}

#[no_mangle]
unsafe extern "C" fn register_default_plugins(plugins: &mut Vec<Box<dyn Plugin>>) {
    plugins.push(Box::new(v5_upload::UploadPlugin::default()));
    plugins.push(Box::new(v5_manage::ManagePlugin::default()));
}
