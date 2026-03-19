use std::sync::Arc;

use crate::services::FileTransferService;

pub struct AppState {
    transfer_service: Arc<dyn FileTransferService>,
}

impl AppState {
    pub fn new(transfer_service: Arc<dyn FileTransferService>) -> Self {
        Self { transfer_service }
    }

    pub fn transfer_service(&self) -> &(dyn FileTransferService + Send + Sync) {
        self.transfer_service.as_ref()
    }
}
