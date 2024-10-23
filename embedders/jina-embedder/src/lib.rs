use std::sync::Arc;

use ferrochain::{
    anyhow::{anyhow, Result},
    embedding::{Embedder, Embedding},
};
use http_client::HttpClient;
use jina_sdk::{
    EmbeddingType, EmbeddingsInput, EmbeddingsModel, EmbeddingsRequest, Jina, JinaBuilder,
};

pub struct JinaEmbedder {
    client: Jina,
    model: EmbeddingsModel,
    normalized: Option<bool>,
    _embedding_type: Option<EmbeddingType>,
}

pub struct JinaEmbedderBuilder {
    jina_builder: JinaBuilder,
    model: Option<EmbeddingsModel>,
    normalized: Option<bool>,
    embedding_type: Option<EmbeddingType>,
}

impl JinaEmbedder {
    pub fn builder() -> JinaEmbedderBuilder {
        JinaEmbedderBuilder {
            jina_builder: Jina::builder(),
            model: None,
            normalized: None,
            embedding_type: None,
        }
    }
}

#[ferrochain::async_trait]
impl Embedder for JinaEmbedder {
    async fn embed(&self, chunks: Vec<String>) -> Result<Vec<Embedding>> {
        Ok(self
            .client
            .embeddings(EmbeddingsRequest {
                model: match self.model {
                    EmbeddingsModel::ClipV1 => EmbeddingsModel::ClipV1,
                    EmbeddingsModel::EmbeddingsV2BaseEn => EmbeddingsModel::EmbeddingsV2BaseEn,
                    EmbeddingsModel::EmbeddingsV2BaseEs => EmbeddingsModel::EmbeddingsV2BaseEs,
                    EmbeddingsModel::EmbeddingsV2BaseDe => EmbeddingsModel::EmbeddingsV2BaseDe,
                    EmbeddingsModel::EmbeddingsV2BaseZh => EmbeddingsModel::EmbeddingsV2BaseZh,
                    EmbeddingsModel::EmbeddingsV2BaseCode => EmbeddingsModel::EmbeddingsV2BaseCode,
                },
                input: EmbeddingsInput::StringArray(chunks),
                embedding_type: None,
                normalized: self.normalized,
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

impl JinaEmbedderBuilder {
    pub fn with_http_client(mut self, http_client: Arc<dyn HttpClient>) -> Self {
        self.jina_builder = self.jina_builder.with_http_client(http_client);
        self
    }

    pub fn with_api_key<S>(mut self, api_key: S) -> Self
    where
        S: AsRef<str>,
    {
        self.jina_builder = self.jina_builder.with_api_key(api_key);
        self
    }

    pub fn with_base_url<S>(mut self, base_url: S) -> Self
    where
        S: AsRef<str>,
    {
        self.jina_builder = self.jina_builder.with_base_url(base_url);
        self
    }

    pub fn with_model(mut self, model: EmbeddingsModel) -> Self {
        self.model = Some(model);
        self
    }

    pub fn with_normalized(mut self, normalized: bool) -> Self {
        self.normalized = Some(normalized);
        self
    }

    pub fn with_embedding_type(mut self, embedding_type: EmbeddingType) -> Self {
        self.embedding_type = Some(embedding_type);
        self
    }

    pub fn build(self) -> Result<JinaEmbedder> {
        Ok(JinaEmbedder {
            client: self.jina_builder.build()?,
            model: self.model.ok_or_else(|| anyhow!("Model is required"))?,
            normalized: self.normalized,
            _embedding_type: self.embedding_type,
        })
    }
}
