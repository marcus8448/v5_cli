use std::collections::HashMap;
use std::path::Path;
use clap::{ArgMatches, Command};
use libloading::Symbol;

pub static mut DEFAULT_PLUGIN_REF: Option<Box<unsafe extern fn (plugins: &mut Vec<Box<dyn Plugin>>)>> = None;

#[macro_export]
macro_rules! export_plugin {
    ($name:expr) => {
        #[doc(hidden)]
        #[no_mangle]
        pub unsafe extern fn register_plugin() -> Box<dyn v5_core::plugin::Plugin> {
            return $name;
        }
    };
}

pub trait Plugin {
    fn get_name(&self) -> &'static str;
    fn create_commands(&self, command: Command, registry: &mut HashMap<&'static str, fn(ArgMatches)>) -> Command;

    fn register_serial_plugins(&self);
}

pub fn load_plugins() -> Vec<Box<dyn Plugin>> {
    let string = std::env::current_exe().unwrap().parent().unwrap().to_str().unwrap().to_owned() + "/plugins";
    let path = Path::new(&string);
    if !path.exists() {
        std::fs::create_dir(path).expect("failed to create plugins directory");
    }
    let mut plugins: Vec<Box<dyn Plugin>> = Vec::new();

    unsafe {
        if let Some(func) = &DEFAULT_PLUGIN_REF {
            func(&mut plugins);
        }
    }

    for entry in std::fs::read_dir(path).unwrap() {
        if let Ok(entry) = entry {
            unsafe {
                plugins.push((libloading::Library::new(entry.file_name()).expect("Failed to load plugin!").get(b"register_plugin\0").unwrap() as Symbol<unsafe extern fn () -> Box<dyn Plugin>>)())
            }
        } else {
            println!("Failed to read plugin: {}", entry.unwrap_err());
        }
    }
    plugins
}