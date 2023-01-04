use clap::{ArgMatches, Command};
use libloading::Library;
use log::error;
use std::collections::HashMap;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;

#[no_mangle]
pub static mut DEFAULT_PLUGIN_REF: Option<
    Box<unsafe extern "C" fn(plugins: &mut Vec<Box<dyn Plugin>>)>,
> = None;
static mut EXTERNAL_LIBRARIES: Vec<Library> = Vec::new(); // We DO NOT want to drop the library
pub const PORT: &str = "port";

#[macro_export]
macro_rules! export_plugin {
    ($name:expr) => {
        #[doc(hidden)]
        #[no_mangle]
        pub unsafe extern "C" fn register_plugin() -> Box<dyn v5_core::plugin::Plugin> {
            return $name;
        }
    };
}

pub trait Plugin {
    fn get_name(&self) -> &'static str;
    fn create_commands(
        &self,
        command: Command,
        registry: &mut HashMap<
            &'static str,
            Box<fn(ArgMatches) -> Pin<Box<dyn Future<Output = ()>>>>,
        >,
    ) -> Command;
}

pub fn load_plugins() -> Vec<Box<dyn Plugin>> {
    let string = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned()
        + "/plugins";
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
                let library = Library::new(entry.path()).expect("Failed to load plugin!");

                plugins.push((library
                    .get::<unsafe extern "C" fn() -> Box<dyn Plugin>>(b"register_plugin\0")
                    .expect("Failed to find exported plugin!"))(
                ));

                EXTERNAL_LIBRARIES.push(library);
            }
        } else {
            error!("Failed to read plugin: {}", entry.unwrap_err());
        }
    }
    plugins
}
