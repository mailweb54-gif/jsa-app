// Niente finestra console su Windows in versione finale
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    jsa_app_lib::run()
}
