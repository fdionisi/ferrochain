use crate::{document::Document, embedding::Embedder, vector_store::VectorStore};
use anyhow::Result;
use std::sync::Arc;

pub struct CodeEmbeddingPipeline {
    embedder: Arc<dyn Embedder>,
    vector_store: Arc<dyn VectorStore>,
}

impl CodeEmbeddingPipeline {
    pub fn new(embedder: Arc<dyn Embedder>, vector_store: Arc<dyn VectorStore>) -> Self {
        CodeEmbeddingPipeline {
            embedder,
            vector_store,
        }
    }

    pub async fn embed_code(&self, code: &str) -> Result<()> {
        let _embedding = self.embedder.embed(vec![code.to_string()]).await?;
        let document = Document {
            content: code.to_string(),
            metadata: Default::default(),
        };
        self.vector_store.add_documents(&[document]).await?;
        Ok(())
    }

    pub async fn search_similar_code(&self, query: &str, limit: u64) -> Result<Vec<Document>> {
        let similarities = self.vector_store.search(query, limit).await?;
        Ok(similarities
            .into_iter()
            .map(|s| s.stored.document)
            .collect())
    }
}
