use std::path::{Path, PathBuf};

use async_trait::async_trait;

use crate::{
    error::AppResult,
    progress::ProgressReporter,
};

#[derive(Debug, Clone)]
pub struct ShareFileRequest {
    path: PathBuf,
}

impl ShareFileRequest {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[derive(Debug, Clone)]
pub struct ReceiveFileRequest {
    ticket: String,
    download_dir: PathBuf,
}

impl ReceiveFileRequest {
    pub fn new(ticket: impl Into<String>, download_dir: impl Into<PathBuf>) -> Self {
        Self {
            ticket: ticket.into(),
            download_dir: download_dir.into(),
        }
    }

    pub fn ticket(&self) -> &str {
        &self.ticket
    }

    pub fn download_dir(&self) -> &Path {
        &self.download_dir
    }
}

#[async_trait]
pub trait FileTransferService: Send + Sync {
    fn get_file_info(&self, path: &Path) -> AppResult<String>;

    async fn share_file(
        &self,
        request: ShareFileRequest,
        reporter: &(dyn ProgressReporter + Send + Sync),
    ) -> AppResult<String>;

    async fn receive_file(
        &self,
        request: ReceiveFileRequest,
        reporter: &(dyn ProgressReporter + Send + Sync),
    ) -> AppResult<String>;
}
