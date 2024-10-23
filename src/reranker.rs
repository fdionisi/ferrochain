use anyhow::Result;
use async_trait::async_trait;

use crate::{document::Document, vector_store::Similarity};

#[async_trait]
pub trait Reranker: Send + Sync {
    async fn rerank(&self, query: &str, docs: Vec<Document>) -> Result<Vec<Similarity>>;
}
