// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use futures_util::StreamExt;
use iroh::{
    client::ShareTicketOptions,
    node::{MemNode, Node},
    rpc_protocol::{BlobDownloadRequest, SetTagOption, WrapOption},
    ticket::BlobTicket,
};
use iroh_bytes::{
    get::db::DownloadProgress,
    provider::AddProgress,
    store::{ExportFormat, ExportMode},
    BlobFormat,
};
use serde::Serialize;
use std::{
    collections::HashMap,
    path::{Component, Path, PathBuf},
    str::FromStr,
    time::Instant,
};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::OnceCell;

static IROH_NODE: OnceCell<MemNode> = OnceCell::const_new();
const UPLOAD_PROGRESS_EVENT: &str = "upload-progress";
const DOWNLOAD_PROGRESS_EVENT: &str = "download-progress";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TransferProgressPayload {
    stage: &'static str,
    message: Option<String>,
    bytes_done: u64,
    total_bytes: Option<u64>,
    percent: Option<f64>,
    speed_bps: Option<f64>,
    eta_seconds: Option<u64>,
}

struct ProgressTracker {
    started_at: Instant,
    fixed_total_bytes: Option<u64>,
    item_sizes: HashMap<u64, u64>,
    item_offsets: HashMap<u64, u64>,
}

async fn shared_node() -> Result<MemNode, String> {
    IROH_NODE
        .get_or_try_init(|| async {
            Node::memory()
                .spawn()
                .await
                .map_err(|error| format!("Erreur démarrage nœud: {error}"))
        })
        .await
        .cloned()
}

impl ProgressTracker {
    fn new(fixed_total_bytes: Option<u64>) -> Self {
        Self {
            started_at: Instant::now(),
            fixed_total_bytes,
            item_sizes: HashMap::new(),
            item_offsets: HashMap::new(),
        }
    }

    fn register_item(&mut self, id: u64, size: u64) {
        self.item_sizes.insert(id, size);
        self.item_offsets.entry(id).or_insert(0);
    }

    fn mark_progress(&mut self, id: u64, offset: u64) {
        self.item_offsets.insert(id, offset);
    }

    fn mark_complete(&mut self, id: u64) {
        if let Some(size) = self.item_sizes.get(&id).copied() {
            self.item_offsets.insert(id, size);
        }
    }

    fn mark_local_complete(&mut self, id: u64, size: u64) {
        self.item_sizes.insert(id, size);
        self.item_offsets.insert(id, size);
    }

    fn bytes_done(&self) -> u64 {
        self.item_offsets
            .iter()
            .map(|(id, offset)| {
                let size = self.item_sizes.get(id).copied().unwrap_or(*offset);
                (*offset).min(size)
            })
            .sum()
    }

    fn total_bytes(&self) -> Option<u64> {
        self.fixed_total_bytes.or_else(|| {
            if self.item_sizes.is_empty() {
                None
            } else {
                Some(self.item_sizes.values().sum())
            }
        })
    }

    fn payload(&self, stage: &'static str, message: Option<String>) -> TransferProgressPayload {
        let bytes_done = self.bytes_done();
        let total_bytes = self.total_bytes();
        let percent = total_bytes
            .filter(|total| *total > 0)
            .map(|total| (bytes_done as f64 / total as f64) * 100.0);
        let elapsed = self.started_at.elapsed().as_secs_f64();
        let speed_bps = (elapsed > 0.0).then_some(bytes_done as f64 / elapsed);
        let eta_seconds = match (speed_bps, total_bytes) {
            (Some(speed), Some(total)) if speed > 0.0 && bytes_done < total => {
                Some(((total - bytes_done) as f64 / speed).ceil() as u64)
            }
            _ => None,
        };

        TransferProgressPayload {
            stage,
            message,
            bytes_done,
            total_bytes,
            percent,
            speed_bps,
            eta_seconds,
        }
    }
}

fn emit_progress(app: &AppHandle, event: &str, payload: TransferProgressPayload) {
    let _ = app.emit(event, payload);
}

fn sanitize_relative_path(name: &str) -> PathBuf {
    let mut sanitized = PathBuf::new();

    for component in Path::new(name).components() {
        if let Component::Normal(part) = component {
            sanitized.push(part);
        }
    }

    if sanitized.as_os_str().is_empty() {
        sanitized.push("fichier");
    }

    sanitized
}

fn unique_path(path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }

    let parent = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let stem = path
        .file_stem()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "fichier".to_string());
    let extension = path.extension().map(|value| value.to_string_lossy().into_owned());

    for index in 1.. {
        let file_name = match &extension {
            Some(extension) => format!("{stem} ({index}).{extension}"),
            None => format!("{stem} ({index})"),
        };
        let candidate = parent.join(file_name);
        if !candidate.exists() {
            return candidate;
        }
    }

    unreachable!("un chemin libre finit toujours par être trouvé")
}

fn short_hash(ticket: &BlobTicket) -> String {
    ticket.hash().to_string().chars().take(12).collect()
}

#[tauri::command]
fn get_file_info(path: String) -> Result<String, String> {
    let metadata = std::fs::metadata(&path).map_err(|error| format!("Erreur : {error}"))?;
    let size_in_mb = metadata.len() as f64 / 1_048_576.0;
    Ok(format!("{size_in_mb:.2} Mo"))
}

#[tauri::command]
async fn share_file(app: AppHandle, path: String) -> Result<String, String> {
    let node = shared_node().await?;
    let client = node.client();
    let path_buf = PathBuf::from(&path);
    let total_bytes = std::fs::metadata(&path_buf)
        .map(|metadata| metadata.len())
        .map_err(|error| format!("Erreur lecture fichier: {error}"))?;
    let mut tracker = ProgressTracker::new(Some(total_bytes));

    emit_progress(
        &app,
        UPLOAD_PROGRESS_EVENT,
        tracker.payload("starting", Some("Préparation du transfert...".to_string())),
    );

    let mut add_progress = client
        .blobs
        .add_from_path(
            path_buf,
            true,
            SetTagOption::Auto,
            WrapOption::Wrap { name: None },
        )
        .await
        .map_err(|error| format!("Erreur lecture fichier: {error}"))?;

    let mut import_result = None;
    while let Some(message) = add_progress.next().await {
        let message = message.map_err(|error| format!("Erreur import fichier: {error}"))?;
        match message {
            AddProgress::Found { id, size, .. } => {
                tracker.register_item(id, size);
                emit_progress(
                    &app,
                    UPLOAD_PROGRESS_EVENT,
                    tracker.payload("progress", Some("Analyse du fichier...".to_string())),
                );
            }
            AddProgress::Progress { id, offset } => {
                tracker.mark_progress(id, offset);
                emit_progress(
                    &app,
                    UPLOAD_PROGRESS_EVENT,
                    tracker.payload("progress", Some("Préparation du ticket...".to_string())),
                );
            }
            AddProgress::Done { id, .. } => {
                tracker.mark_complete(id);
                emit_progress(
                    &app,
                    UPLOAD_PROGRESS_EVENT,
                    tracker.payload("progress", Some("Finalisation du ticket...".to_string())),
                );
            }
            AddProgress::AllDone { hash, format, .. } => {
                import_result = Some((hash, format));
                break;
            }
            AddProgress::Abort(error) => {
                emit_progress(
                    &app,
                    UPLOAD_PROGRESS_EVENT,
                    tracker.payload("error", Some(error.to_string())),
                );
                return Err(format!("Erreur import fichier: {error}"));
            }
        }
    }

    let (hash, format) =
        import_result.ok_or_else(|| "Erreur import fichier: flux interrompu".to_string())?;

    let ticket = client
        .blobs
        .share(hash, format, ShareTicketOptions::default())
        .await
        .map_err(|error| format!("Impossible de générer le ticket P2P: {error}"))?;

    emit_progress(
        &app,
        UPLOAD_PROGRESS_EVENT,
        tracker.payload("finished", Some("Ticket sécurisé prêt.".to_string())),
    );

    Ok(ticket.to_string())
}

#[tauri::command]
async fn receive_file(app: AppHandle, ticket: String) -> Result<String, String> {
    let ticket =
        BlobTicket::from_str(ticket.trim()).map_err(|error| format!("Ticket invalide : {error}"))?;

    let node = shared_node().await?;
    let client = node.client();
    let download_dir = app
        .path()
        .download_dir()
        .map_err(|error| format!("Impossible de localiser le dossier Téléchargements : {error}"))?;
    let mut tracker = ProgressTracker::new(None);

    emit_progress(
        &app,
        DOWNLOAD_PROGRESS_EVENT,
        tracker.payload("starting", Some("Connexion au pair...".to_string())),
    );

    let mut download_progress = client
        .blobs
        .download(BlobDownloadRequest {
            hash: ticket.hash(),
            format: ticket.format(),
            peer: ticket.node_addr().clone(),
            tag: SetTagOption::Auto,
        })
        .await
        .map_err(|error| format!("Impossible de démarrer le téléchargement : {error}"))?;

    while let Some(message) = download_progress.next().await {
        let message = message.map_err(|error| format!("Échec du téléchargement P2P : {error}"))?;
        match message {
            DownloadProgress::Connected => {
                emit_progress(
                    &app,
                    DOWNLOAD_PROGRESS_EVENT,
                    tracker.payload("connected", Some("Connexion établie.".to_string())),
                );
            }
            DownloadProgress::Found { id, size, .. } => {
                tracker.register_item(id, size);
                emit_progress(
                    &app,
                    DOWNLOAD_PROGRESS_EVENT,
                    tracker.payload("progress", Some("Téléchargement en cours...".to_string())),
                );
            }
            DownloadProgress::FoundLocal { child, size, .. } => {
                tracker.mark_local_complete(child, size.value());
                emit_progress(
                    &app,
                    DOWNLOAD_PROGRESS_EVENT,
                    tracker.payload(
                        "progress",
                        Some("Vérification des données locales...".to_string()),
                    ),
                );
            }
            DownloadProgress::Progress { id, offset } => {
                tracker.mark_progress(id, offset);
                emit_progress(
                    &app,
                    DOWNLOAD_PROGRESS_EVENT,
                    tracker.payload("progress", Some("Téléchargement en cours...".to_string())),
                );
            }
            DownloadProgress::Done { id } => {
                tracker.mark_complete(id);
                emit_progress(
                    &app,
                    DOWNLOAD_PROGRESS_EVENT,
                    tracker.payload("progress", Some("Téléchargement en cours...".to_string())),
                );
            }
            DownloadProgress::FoundHashSeq { .. } => {}
            DownloadProgress::AllDone(_) => {
                emit_progress(
                    &app,
                    DOWNLOAD_PROGRESS_EVENT,
                    tracker.payload(
                        "saving",
                        Some("Enregistrement dans Téléchargements...".to_string()),
                    ),
                );
                break;
            }
            DownloadProgress::Abort(error) => {
                emit_progress(
                    &app,
                    DOWNLOAD_PROGRESS_EVENT,
                    tracker.payload("error", Some(error.to_string())),
                );
                return Err(format!("Échec du téléchargement P2P : {error}"));
            }
        }
    }

    let saved_path = match ticket.format() {
        BlobFormat::Raw => {
            let destination = unique_path(download_dir.join(format!(
                "altsendme-{}",
                short_hash(&ticket)
            )));

            client
                .blobs
                .export(
                    ticket.hash(),
                    destination.clone(),
                    ExportFormat::Blob,
                    ExportMode::Copy,
                )
                .await
                .map_err(|error| format!("Impossible d'exporter le fichier : {error}"))?
                .finish()
                .await
                .map_err(|error| format!("Échec de l'export du fichier : {error}"))?;

            destination
        }
        BlobFormat::HashSeq => {
            let collection = client
                .blobs
                .get_collection(ticket.hash())
                .await
                .map_err(|error| format!("Impossible de lire la collection reçue : {error}"))?;
            let entries: Vec<_> = collection
                .iter()
                .map(|(name, hash)| (name.clone(), *hash))
                .collect();

            if entries.len() == 1 {
                let (name, hash) = &entries[0];
                let destination = unique_path(download_dir.join(sanitize_relative_path(name)));

                client
                    .blobs
                    .export(
                        *hash,
                        destination.clone(),
                        ExportFormat::Blob,
                        ExportMode::Copy,
                    )
                    .await
                    .map_err(|error| format!("Impossible d'exporter le fichier reçu : {error}"))?
                    .finish()
                    .await
                    .map_err(|error| format!("Échec de l'export du fichier reçu : {error}"))?;

                destination
            } else {
                let destination =
                    unique_path(download_dir.join(format!("AltSendme-{}", short_hash(&ticket))));

                client
                    .blobs
                    .export(
                        ticket.hash(),
                        destination.clone(),
                        ExportFormat::Collection,
                        ExportMode::Copy,
                    )
                    .await
                    .map_err(|error| format!("Impossible d'exporter les fichiers reçus : {error}"))?
                    .finish()
                    .await
                    .map_err(|error| format!("Échec de l'export des fichiers reçus : {error}"))?;

                destination
            }
        }
    };

    emit_progress(
        &app,
        DOWNLOAD_PROGRESS_EVENT,
        tracker.payload("finished", Some("Fichier enregistré.".to_string())),
    );

    Ok(saved_path.display().to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_file_info,
            share_file,
            receive_file
        ])
        .run(tauri::generate_context!())
        .expect("Erreur lors du lancement de Tauri");
}
