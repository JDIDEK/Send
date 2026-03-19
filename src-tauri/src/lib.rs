mod commands;
mod error;
mod progress;
mod services;
mod state;
mod utils;

use std::sync::Arc;

use services::{FileTransferService, IrohFileTransferService, IrohNodeProvider};
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let transfer_service: Arc<dyn FileTransferService> =
        Arc::new(IrohFileTransferService::new(IrohNodeProvider::default()));

    tauri::Builder::default()
        .manage(AppState::new(transfer_service))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_file_info,
            commands::share_file,
            commands::receive_file
        ])
        .run(tauri::generate_context!())
        .expect("Erreur lors du lancement de Tauri");
}
