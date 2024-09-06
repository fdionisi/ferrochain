use ferrochain::{
    anyhow::{anyhow, Result},
    embedding::{Embedder, Embedding},
};
use voyageai_sdk::{
    EmbeddingsInput, EmbeddingsInputType, EmbeddingsModel, EmbeddingsRequest, VoyageAi,
    VoyageAiBuilder,
};

pub struct VoyageAiEmbedder {
    client: VoyageAi,
    model: EmbeddingsModel,
    input_type: Option<EmbeddingsInputType>,
    truncation: Option<bool>,
}

pub struct VoyageAiEmbedderBuilder {
    voyageai_builder: VoyageAiBuilder,
    model: Option<EmbeddingsModel>,
    input_type: Option<EmbeddingsInputType>,
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
            .embeddings(EmbeddingsRequest {
                model: match self.model {
                    EmbeddingsModel::Voyage2 => EmbeddingsModel::Voyage2,
                    EmbeddingsModel::VoyageLarge2 => EmbeddingsModel::VoyageLarge2,
                    EmbeddingsModel::VoyageFinance2 => EmbeddingsModel::VoyageFinance2,
                    EmbeddingsModel::VoyageMultilingual2 => EmbeddingsModel::VoyageMultilingual2,
                    EmbeddingsModel::VoyageLaw2 => EmbeddingsModel::VoyageLaw2,
                    EmbeddingsModel::VoyageCode2 => EmbeddingsModel::VoyageCode2,
                },
                input: EmbeddingsInput::Multiple(chunks),
                input_type: self.input_type.as_ref().map(|input_type| match input_type {
                    EmbeddingsInputType::Query => EmbeddingsInputType::Query,
                    EmbeddingsInputType::Document => EmbeddingsInputType::Document,
                }),
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
    pub fn api_key(mut self, api_key: String) -> Self {
        self.voyageai_builder = self.voyageai_builder.api_key(api_key);
        self
    }

    pub fn base_url(mut self, base_url: String) -> Self {
        self.voyageai_builder = self.voyageai_builder.base_url(base_url);
        self
    }

    pub fn model(mut self, model: EmbeddingsModel) -> Self {
        self.model = Some(model);
        self
    }

    pub fn input_type(mut self, input_type: EmbeddingsInputType) -> Self {
        self.input_type = Some(input_type);
        self
    }

    pub fn truncation(mut self, truncation: bool) -> Self {
        self.truncation = Some(truncation);
        self
    }

    pub async fn build(self) -> Result<VoyageAiEmbedder> {
        Ok(VoyageAiEmbedder {
            client: self.voyageai_builder.build()?,
            model: self.model.ok_or_else(|| anyhow!("model is required"))?,
            input_type: self.input_type,
            truncation: self.truncation,
        })
    }
}
