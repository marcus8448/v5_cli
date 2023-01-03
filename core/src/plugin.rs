use clap::{ArgMatches, Command};
use libloading::Library;
use log::error;
use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

#[no_mangle]
pub static mut DEFAULT_PLUGIN_REF: Option<
    Box<unsafe extern "C" fn(plugins: &mut Vec<Box<dyn Plugin>>)>,
> = None;
static mut EXTERNAL_LIBRARIES: Vec<Library> = Vec::new(); // We DO NOT want to drop the libraries
pub const PORT: &str = "port";

pub type CommandRegistry =
    HashMap<&'static str, Box<fn(ArgMatches) -> Pin<Box<dyn Future<Output = ()>>>>>;
pub type CustomDataRegistry = HashMap<&'static str, Vec<Box<dyn Any>>>;

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

pub fn get_custom<T: 'static>(
    mut registry: CustomDataRegistry,
    id: &'static str,
) -> (CustomDataRegistry, Vec<T>) {
    let option = registry.remove(id);
    if let Some(vec) = option {
        let mut out: Vec<T> = Vec::with_capacity(vec.len());
        for x in vec {
            if let Ok(value) = x.downcast() {
                out.push(*value)
            }
        }
        out.shrink_to_fit();
        return (registry, out);
    }
    (registry, Vec::new())
}

pub trait Plugin {
    fn get_name(&self) -> &'static str;

    fn create_commands(&self, registry: &mut CommandRegistry) -> Option<Command>;

    fn register_custom(&self, registry: &mut CustomDataRegistry);

    fn take_custom(&self, registry: CustomDataRegistry) -> CustomDataRegistry;
}

pub fn load_plugins() -> Vec<Box<dyn Plugin>> {
    let path = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .join("plugins");
    if !path.exists() {
        std::fs::create_dir(&path).expect("failed to create plugins directory");
    } else if !path.is_dir() {
        error!("Expected plugin directory, but found a file instead!")
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
