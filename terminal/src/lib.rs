use std::collections::HashMap;
use v5_core::clap::{ArgMatches, Command};
use v5_core::error::Result;
use v5_core::export_plugin;
use v5_core::plugin::{get_custom, CommandRegistry, CustomDataRegistry, Plugin, PORT};

const TERMINAL: &str = "terminal";

export_plugin!(Box::new(TerminalPlugin::default()));

pub struct TerminalPlugin {
    serial_plugins: Vec<Box<dyn SerialPlugin>>,
}

pub type PacketHandlerRegistry = HashMap<u8, Box<dyn PacketHandler>>;

pub trait PacketHandler {
    fn handle(&self, data: &[u8]) -> Result<()>;
}

pub trait SerialPlugin {
    fn tick_serial(&self);

    fn register_handlers(&self, registry: &mut PacketHandlerRegistry);
}

impl Default for TerminalPlugin {
    fn default() -> Self {
        TerminalPlugin {
            serial_plugins: Vec::new(),
        }
    }
}

impl Plugin for TerminalPlugin {
    fn get_name(&self) -> &'static str {
        TERMINAL
    }

    fn create_commands(&self, registry: &mut CommandRegistry) -> Option<Command> {
        registry.insert(TERMINAL, Box::new(|f| Box::pin(terminal(f))));
        Some(Command::new(TERMINAL).about("Opens a terminal to the robot"))
    }

    fn register_custom(&self, _: &mut CustomDataRegistry) {}

    fn take_custom(&self, registry: CustomDataRegistry) -> CustomDataRegistry {
        let (reg, custom): (CustomDataRegistry, Vec<Box<dyn SerialPlugin>>) =
            get_custom(registry, "");
        let mut registry = PacketHandlerRegistry::new();
        for plugin in custom {
            plugin.register_handlers(&mut registry);
        }
        reg
    }
}

async fn terminal(args: ArgMatches) {
    let brain =
        v5_core::serial::connect_to_user(args.get_one(PORT).map(|f: &String| f.to_string()));
}
