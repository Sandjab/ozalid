// Pas de console au lancement sous Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    ozalid_lib::run()
}
