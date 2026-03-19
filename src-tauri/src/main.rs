// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use iroh::{
    client::ShareTicketOptions,
    node::{MemNode, Node},
    rpc_protocol::{BlobDownloadRequest, SetTagOption, WrapOption},
    ticket::BlobTicket,
};
use iroh_bytes::{
    store::{ExportFormat, ExportMode},
    BlobFormat,
};
use std::{
    path::{Component, Path, PathBuf},
    str::FromStr,
};
use tauri::{AppHandle, Manager};
use tokio::sync::OnceCell;

static IROH_NODE: OnceCell<MemNode> = OnceCell::const_new();

async fn shared_node() -> Result<MemNode, String> {
    IROH_NODE
        .get_or_try_init(|| async {
            Node::memory()
                .spawn()
                .await
                .map_err(|e| format!("Erreur démarrage nœud: {}", e))
        })
        .await
        .cloned()
}

#[tauri::command]
fn get_file_info(path: String) -> Result<String, String> {
    match std::fs::metadata(&path) {
        Ok(metadata) => {
            let size_in_mb = metadata.len() as f64 / 1_048_576.0;
            Ok(format!("{:.2} Mo", size_in_mb))
        }
        Err(e) => Err(format!("Erreur : {}", e)),
    }
}

#[tauri::command]
async fn share_file(path: String) -> Result<String, String> {
    let node = shared_node().await?;
    let client = node.client();
    let path_buf = PathBuf::from(path);

    let add_outcome = client
        .blobs
        .add_from_path(
            path_buf,
            true,
            SetTagOption::Auto,
            WrapOption::Wrap { name: None },
        )
        .await
        .map_err(|e| format!("Erreur lecture fichier: {}", e))?
        .finish()
        .await
        .map_err(|e| format!("Erreur import fichier: {}", e))?;

    let ticket = client
        .blobs
        .share(
            add_outcome.hash,
            add_outcome.format,
            ShareTicketOptions::default(),
        )
        .await
        .map_err(|e| format!("Impossible de générer le ticket P2P: {}", e))?;

    Ok(ticket.to_string())
}

fn short_hash(ticket: &BlobTicket) -> String {
    ticket.hash().to_string().chars().take(12).collect()
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
        .unwrap_or_else(PathBuf::new);
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

    unreachable!("la boucle retourne dès qu'un chemin libre est trouvé")
}

#[tauri::command]
async fn receive_file(app: AppHandle, ticket: String) -> Result<String, String> {
    let ticket = BlobTicket::from_str(ticket.trim())
        .map_err(|e| format!("Ticket invalide : {}", e))?;

    let node = shared_node().await?;
    let client = node.client();
    let download_dir = app
        .path()
        .download_dir()
        .map_err(|e| format!("Impossible de localiser le dossier Téléchargements : {}", e))?;

    client
        .blobs
        .download(BlobDownloadRequest {
            hash: ticket.hash(),
            format: ticket.format(),
            peer: ticket.node_addr().clone(),
            tag: SetTagOption::Auto,
        })
        .await
        .map_err(|e| format!("Impossible de démarrer le téléchargement : {}", e))?
        .finish()
        .await
        .map_err(|e| format!("Échec du téléchargement P2P : {}", e))?;

    let saved_path = match ticket.format() {
        BlobFormat::Raw => {
            let file_name = format!("altsendme-{}", short_hash(&ticket));
            let destination = unique_path(download_dir.join(file_name));

            client
                .blobs
                .export(
                    ticket.hash(),
                    destination.clone(),
                    ExportFormat::Blob,
                    ExportMode::Copy,
                )
                .await
                .map_err(|e| format!("Impossible d'exporter le fichier : {}", e))?
                .finish()
                .await
                .map_err(|e| format!("Échec de l'export du fichier : {}", e))?;

            destination
        }
        BlobFormat::HashSeq => {
            let collection = client
                .blobs
                .get_collection(ticket.hash())
                .await
                .map_err(|e| format!("Impossible de lire la collection reçue : {}", e))?;
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
                    .map_err(|e| format!("Impossible d'exporter le fichier reçu : {}", e))?
                    .finish()
                    .await
                    .map_err(|e| format!("Échec de l'export du fichier reçu : {}", e))?;

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
                    .map_err(|e| format!("Impossible d'exporter les fichiers reçus : {}", e))?
                    .finish()
                    .await
                    .map_err(|e| format!("Échec de l'export des fichiers reçus : {}", e))?;

                destination
            }
        }
    };

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
