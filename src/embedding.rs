use anyhow::Result;

use crate::async_trait;

pub struct Embedding(Vec<f32>);

impl Embedding {
    pub fn to_vec(&self) -> Vec<f32> {
        self.0.clone()
    }
}

impl From<Embedding> for Vec<f32> {
    fn from(embedding: Embedding) -> Vec<f32> {
        embedding.to_vec()
    }
}

impl From<Vec<f32>> for Embedding {
    fn from(vector: Vec<f32>) -> Self {
        Self(vector)
    }
}

#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, chunks: Vec<String>) -> Result<Vec<Embedding>>;
}
