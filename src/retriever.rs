use anyhow::Result;
use async_trait::async_trait;

use crate::document::Document;

#[async_trait]
pub trait Retriever: Send + Sync {
    async fn retrieve(&self, query: &str) -> Result<Vec<Document>>;
}
