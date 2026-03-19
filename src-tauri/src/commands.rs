use std::path::Path;

use tauri::{AppHandle, Emitter, State};

use crate::{
    progress::{ProgressReporter, TransferProgress, DOWNLOAD_PROGRESS_EVENT, UPLOAD_PROGRESS_EVENT},
    services::{ReceiveFileRequest, ShareFileRequest},
    state::AppState,
};

struct TauriProgressReporter {
    app: AppHandle,
    event_name: &'static str,
}

impl TauriProgressReporter {
    fn new(app: AppHandle, event_name: &'static str) -> Self {
        Self { app, event_name }
    }
}

impl ProgressReporter for TauriProgressReporter {
    fn report(&self, progress: TransferProgress) {
        let _ = self.app.emit(self.event_name, progress);
    }
}

#[tauri::command]
pub fn get_file_info(state: State<'_, AppState>, path: String) -> Result<String, String> {
    state
        .transfer_service()
        .get_file_info(Path::new(&path))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn share_file(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<String, String> {
    let reporter = TauriProgressReporter::new(app, UPLOAD_PROGRESS_EVENT);

    state
        .transfer_service()
        .share_file(ShareFileRequest::new(path), &reporter)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn receive_file(
    app: AppHandle,
    state: State<'_, AppState>,
    ticket: String,
    destination_path: String,
) -> Result<String, String> {
    let reporter = TauriProgressReporter::new(app, DOWNLOAD_PROGRESS_EVENT);

    state
        .transfer_service()
        .receive_file(ReceiveFileRequest::new(ticket, destination_path), &reporter)
        .await
        .map_err(|error| error.to_string())
}
