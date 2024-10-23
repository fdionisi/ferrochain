use std::sync::Arc;

use ferrochain::{
    anyhow::{anyhow, Result},
    embedding::{Embedder, Embedding},
};
use http_client::HttpClient;
use voyageai_sdk::{EmbeddingInput, EmbeddingRequest, VoyageAi, VoyageAiBuilder};
pub use voyageai_sdk::{EmbeddingInputType, EmbeddingModel};

pub struct VoyageAiEmbedder {
    client: VoyageAi,
    model: EmbeddingModel,
    input_type: Option<EmbeddingInputType>,
    truncation: Option<bool>,
}

#[derive(Clone)]
pub struct VoyageAiEmbedderBuilder {
    voyageai_builder: VoyageAiBuilder,
    model: Option<EmbeddingModel>,
    input_type: Option<EmbeddingInputType>,
    truncation: Option<bool>,
}

impl VoyageAiEmbedder {
    pub fn builder() -> VoyageAiEmbedderBuilder {
        VoyageAiEmbedderBuilder {
            voyageai_builder: VoyageAi::builder(),
            model: None,
            input_type: None,
            truncation: None,
        }
    }
}

#[ferrochain::async_trait]
impl Embedder for VoyageAiEmbedder {
    async fn embed(&self, chunks: Vec<String>) -> Result<Vec<Embedding>> {
        Ok(self
            .client
            .embeddings(EmbeddingRequest {
                model: self.model.clone(),
                input: EmbeddingInput::Multiple(chunks),
                input_type: self.input_type.clone(),
                truncation: self.truncation,
                encoding_format: None,
            })
            .await
            .map(|response| {
                response
                    .data
                    .into_iter()
                    .map(|d| d.embedding.into())
                    .collect()
            })?)
    }
}

impl VoyageAiEmbedderBuilder {
    pub fn with_http_client(mut self, http_client: Arc<dyn HttpClient>) -> Self {
        self.voyageai_builder = self.voyageai_builder.with_http_client(http_client);
        self
    }

    pub fn with_api_key<S>(mut self, api_key: S) -> Self
    where
        S: AsRef<str>,
    {
        self.voyageai_builder = self.voyageai_builder.with_api_key(api_key);
        self
    }

    pub fn with_base_url<S>(mut self, base_url: S) -> Self
    where
        S: AsRef<str>,
    {
        self.voyageai_builder = self.voyageai_builder.with_base_url(base_url);
        self
    }

    pub fn with_model(mut self, model: EmbeddingModel) -> Self {
        self.model = Some(model);
        self
    }

    pub fn with_input_type(mut self, input_type: EmbeddingInputType) -> Self {
        self.input_type = Some(input_type);
        self
    }

    pub fn with_truncation(mut self, truncation: bool) -> Self {
        self.truncation = Some(truncation);
        self
    }

    pub fn build(self) -> Result<VoyageAiEmbedder> {
        Ok(VoyageAiEmbedder {
            client: self.voyageai_builder.build()?,
            model: self.model.ok_or_else(|| anyhow!("model is required"))?,
            input_type: self.input_type,
            truncation: self.truncation,
        })
    }
}
