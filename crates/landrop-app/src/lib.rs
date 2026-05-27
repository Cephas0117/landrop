pub mod commands;
pub mod events;
pub mod dto;
pub mod state;

pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
