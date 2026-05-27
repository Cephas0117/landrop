// Prevents additional console window on Windows in release mode
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Both aws-lc-rs (Tauri) and ring (our crates) are compiled in; pick ring explicitly.
    rustls::crypto::ring::default_provider()
        .install_default()
        .unwrap_or_default();
    landrop_lib::run();
}
