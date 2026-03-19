use async_trait::async_trait;
use futures_util::StreamExt;
use iroh::{
    client::ShareTicketOptions,
    rpc_protocol::{BlobDownloadRequest, ProviderService, SetTagOption, WrapOption},
    ticket::BlobTicket,
};
use iroh_bytes::{
    get::db::DownloadProgress,
    provider::AddProgress,
    store::{ExportFormat, ExportMode},
    BlobFormat,
};
use std::{path::Path, str::FromStr};
use quic_rpc::ServiceConnection;

use crate::{
    error::{AppError, AppResult},
    progress::{
        LinearProgressMetricsCalculator, ProgressReporter, ProgressTracker, TransferStage,
    },
    utils::{sanitize_relative_path, unique_path},
};

use super::{
    contracts::{FileTransferService, ReceiveFileRequest, ShareFileRequest},
    node_provider::BlobNodeProvider,
};

pub struct IrohFileTransferService<P> {
    node_provider: P,
}

impl<P> IrohFileTransferService<P> {
    pub fn new(node_provider: P) -> Self {
        Self { node_provider }
    }
}

#[async_trait]
impl<P> FileTransferService for IrohFileTransferService<P>
where
    P: BlobNodeProvider + Send + Sync,
{
    fn get_file_info(&self, path: &Path) -> AppResult<String> {
        let metadata =
            std::fs::metadata(path).map_err(|error| AppError::context("Erreur", error))?;
        let size_in_mb = metadata.len() as f64 / 1_048_576.0;
        Ok(format!("{size_in_mb:.2} Mo"))
    }

    async fn share_file(
        &self,
        request: ShareFileRequest,
        reporter: &(dyn ProgressReporter + Send + Sync),
    ) -> AppResult<String> {
        let node = self.node_provider.shared_node().await?;
        let client = node.client();
        let total_bytes = std::fs::metadata(request.path())
            .map_err(|error| AppError::context("Erreur lecture fichier", error))?
            .len();
        let mut tracker = ProgressTracker::new(Some(total_bytes), LinearProgressMetricsCalculator);

        reporter.report(tracker.snapshot(
            TransferStage::Starting,
            Some("Préparation du transfert...".to_string()),
        ));

        let mut add_progress = client
            .blobs
            .add_from_path(
                request.path().to_path_buf(),
                true,
                SetTagOption::Auto,
                WrapOption::Wrap { name: None },
            )
            .await
            .map_err(|error| AppError::context("Erreur lecture fichier", error))?;

        let mut imported = None;
        while let Some(message) = add_progress.next().await {
            let message =
                message.map_err(|error| AppError::context("Erreur import fichier", error))?;
            match message {
                AddProgress::Found { id, size, .. } => {
                    tracker.register_item(id, size);
                    reporter.report(tracker.snapshot(
                        TransferStage::Progress,
                        Some("Analyse du fichier...".to_string()),
                    ));
                }
                AddProgress::Progress { id, offset } => {
                    tracker.mark_progress(id, offset);
                    reporter.report(tracker.snapshot(
                        TransferStage::Progress,
                        Some("Préparation du ticket...".to_string()),
                    ));
                }
                AddProgress::Done { id, .. } => {
                    tracker.mark_complete(id);
                    reporter.report(tracker.snapshot(
                        TransferStage::Progress,
                        Some("Finalisation du ticket...".to_string()),
                    ));
                }
                AddProgress::AllDone { hash, format, .. } => {
                    imported = Some((hash, format));
                    break;
                }
                AddProgress::Abort(error) => {
                    reporter.report(
                        tracker.snapshot(TransferStage::Error, Some(error.to_string())),
                    );
                    return Err(AppError::message(format!("Erreur import fichier: {error}")));
                }
            }
        }

        let (hash, format) =
            imported.ok_or_else(|| AppError::message("Erreur import fichier: flux interrompu"))?;

        let ticket = client
            .blobs
            .share(hash, format, ShareTicketOptions::default())
            .await
            .map_err(|error| AppError::context("Impossible de générer le ticket P2P", error))?;

        reporter.report(tracker.snapshot(
            TransferStage::Finished,
            Some("Ticket sécurisé prêt.".to_string()),
        ));

        Ok(ticket.to_string())
    }

    async fn receive_file(
        &self,
        request: ReceiveFileRequest,
        reporter: &(dyn ProgressReporter + Send + Sync),
    ) -> AppResult<String> {
        let ticket = BlobTicket::from_str(request.ticket())
            .map_err(|error| AppError::message(format!("Ticket invalide : {error}")))?;
        let node = self.node_provider.shared_node().await?;
        let client = node.client();
        let mut tracker = ProgressTracker::new(None, LinearProgressMetricsCalculator);

        reporter.report(tracker.snapshot(
            TransferStage::Starting,
            Some("Connexion au pair...".to_string()),
        ));

        let mut download_progress = client
            .blobs
            .download(BlobDownloadRequest {
                hash: ticket.hash(),
                format: ticket.format(),
                peer: ticket.node_addr().clone(),
                tag: SetTagOption::Auto,
            })
            .await
            .map_err(|error| AppError::context("Impossible de démarrer le téléchargement", error))?;

        while let Some(message) = download_progress.next().await {
            let message = message
                .map_err(|error| AppError::context("Échec du téléchargement P2P", error))?;
            match message {
                DownloadProgress::Connected => {
                    reporter.report(tracker.snapshot(
                        TransferStage::Connected,
                        Some("Connexion établie.".to_string()),
                    ));
                }
                DownloadProgress::Found { id, size, .. } => {
                    tracker.register_item(id, size);
                    reporter.report(tracker.snapshot(
                        TransferStage::Progress,
                        Some("Téléchargement en cours...".to_string()),
                    ));
                }
                DownloadProgress::FoundLocal { child, size, .. } => {
                    tracker.mark_local_complete(child, size.value());
                    reporter.report(tracker.snapshot(
                        TransferStage::Progress,
                        Some("Vérification des données locales...".to_string()),
                    ));
                }
                DownloadProgress::Progress { id, offset } => {
                    tracker.mark_progress(id, offset);
                    reporter.report(tracker.snapshot(
                        TransferStage::Progress,
                        Some("Téléchargement en cours...".to_string()),
                    ));
                }
                DownloadProgress::Done { id } => {
                    tracker.mark_complete(id);
                    reporter.report(tracker.snapshot(
                        TransferStage::Progress,
                        Some("Téléchargement en cours...".to_string()),
                    ));
                }
                DownloadProgress::FoundHashSeq { .. } => {}
                DownloadProgress::AllDone(_) => {
                    reporter.report(tracker.snapshot(
                        TransferStage::Saving,
                        Some("Enregistrement dans Téléchargements...".to_string()),
                    ));
                    break;
                }
                DownloadProgress::Abort(error) => {
                    reporter.report(
                        tracker.snapshot(TransferStage::Error, Some(error.to_string())),
                    );
                    return Err(AppError::message(format!("Échec du téléchargement P2P : {error}")));
                }
            }
        }

        let saved_path = self
            .export_received_content(&client, &ticket, request.destination_path())
            .await?;

        reporter.report(tracker.snapshot(
            TransferStage::Finished,
            Some("Fichier enregistré.".to_string()),
        ));

        Ok(saved_path.display().to_string())
    }
}

impl<P> IrohFileTransferService<P>
where
    P: BlobNodeProvider + Send + Sync,
{
    async fn export_received_content<C>(
        &self,
        client: &iroh::client::Iroh<C>,
        ticket: &BlobTicket,
        destination_path: &Path,
    ) -> AppResult<std::path::PathBuf>
    where
        C: ServiceConnection<ProviderService>,
    {
        match ticket.format() {
            BlobFormat::Raw => self.export_raw_blob(client, ticket, destination_path).await,
            BlobFormat::HashSeq => self.export_collection_blob(client, ticket, destination_path).await,
        }
    }

    async fn export_raw_blob<C>(
        &self,
        client: &iroh::client::Iroh<C>,
        ticket: &BlobTicket,
        destination_path: &Path,
    ) -> AppResult<std::path::PathBuf>
    where
        C: ServiceConnection<ProviderService>,
    {
        let destination = unique_path(destination_path.to_path_buf());

        client
            .blobs
            .export(
                ticket.hash(),
                destination.clone(),
                ExportFormat::Blob,
                ExportMode::Copy,
            )
            .await
            .map_err(|error| AppError::context("Impossible d'exporter le fichier", error))?
            .finish()
            .await
            .map_err(|error| AppError::context("Échec de l'export du fichier", error))?;

        Ok(destination)
    }

    async fn export_collection_blob<C>(
        &self,
        client: &iroh::client::Iroh<C>,
        ticket: &BlobTicket,
        destination_path: &Path,
    ) -> AppResult<std::path::PathBuf>
    where
        C: ServiceConnection<ProviderService>,
    {
        let collection = client
            .blobs
            .get_collection(ticket.hash())
            .await
            .map_err(|error| AppError::context("Impossible de lire la collection reçue", error))?;
        let entries: Vec<_> = collection
            .iter()
            .map(|(name, hash)| (name.clone(), *hash))
            .collect();

        if entries.len() == 1 {
            let (name, hash) = &entries[0];
            let destination = if destination_path.extension().is_some() {
                unique_path(destination_path.to_path_buf())
            } else {
                unique_path(destination_path.join(sanitize_relative_path(name)))
            };

            client
                .blobs
                .export(
                    *hash,
                    destination.clone(),
                    ExportFormat::Blob,
                    ExportMode::Copy,
                )
                .await
                .map_err(|error| AppError::context("Impossible d'exporter le fichier reçu", error))?
                .finish()
                .await
                .map_err(|error| AppError::context("Échec de l'export du fichier reçu", error))?;

            Ok(destination)
        } else {
            let destination = unique_path(destination_path.to_path_buf());

            client
                .blobs
                .export(
                    ticket.hash(),
                    destination.clone(),
                    ExportFormat::Collection,
                    ExportMode::Copy,
                )
                .await
                .map_err(|error| {
                    AppError::context("Impossible d'exporter les fichiers reçus", error)
                })?
                .finish()
                .await
                .map_err(|error| AppError::context("Échec de l'export des fichiers reçus", error))?;

            Ok(destination)
        }
    }
}
