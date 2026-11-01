// Pas de console en version release sous Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Some(code) = prisme_lib::plugin_host_entry() {
        std::process::exit(code);
    }
    prisme_lib::run()
}
