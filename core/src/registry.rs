// use crate::plugin::Plugin;
// use std::collections::HashMap;
//
// struct PluginRegistry {
//     map: HashMap<&'static str, Box<dyn Plugin>>,
// }
//
// impl PluginRegistry {
//     pub fn register(&mut self, id: &'static str, plugin: Box<dyn Plugin>) {
//         if self.map.contains_key(id) {
//             panic!("duplicate plugin with id: '{}'", id)
//         }
//         self.map.insert(id, plugin);
//     }
// }
