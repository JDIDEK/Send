use async_trait::async_trait;
use iroh::node::{MemNode, Node};
use tokio::sync::OnceCell;

use crate::error::{AppError, AppResult};

#[async_trait]
pub trait BlobNodeProvider: Send + Sync {
    async fn shared_node(&self) -> AppResult<MemNode>;
}

#[derive(Default)]
pub struct IrohNodeProvider {
    node: OnceCell<MemNode>,
}

#[async_trait]
impl BlobNodeProvider for IrohNodeProvider {
    async fn shared_node(&self) -> AppResult<MemNode> {
        self.node
            .get_or_try_init(|| async {
                Node::memory()
                    .spawn()
                    .await
                    .map_err(|error| AppError::context("Erreur démarrage nœud", error))
            })
            .await
            .cloned()
    }
}
