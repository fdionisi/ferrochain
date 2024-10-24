use std::sync::Arc;

use ferrochain::{
    anyhow::{anyhow, Result},
    document::{Document, StoredDocument},
    reranker::Reranker,
    vector_store::Similarity,
};
use http_client::HttpClient;
pub use voyageai_sdk::RerankModel;
use voyageai_sdk::{RerankRequest, VoyageAi, VoyageAiBuilder};

pub struct VoyageAiReranker {
    client: VoyageAi,
    model: RerankModel,
    top_k: Option<u32>,
    truncation: Option<bool>,
}

pub struct VoyageAiRerankerBuilder {
    voyageai_builder: VoyageAiBuilder,
    model: Option<RerankModel>,
    top_k: Option<u32>,
    truncation: Option<bool>,
}

impl VoyageAiReranker {
    pub fn builder() -> VoyageAiRerankerBuilder {
        VoyageAiRerankerBuilder {
            voyageai_builder: VoyageAi::builder(),
            model: None,
            top_k: None,
            truncation: None,
        }
    }
}

#[ferrochain::async_trait]
impl Reranker for VoyageAiReranker {
    async fn rerank(&self, query: &str, docs: Vec<Document>) -> Result<Vec<Similarity>> {
        Ok(self
            .client
            .rerank(RerankRequest {
                query: query.into(),
                documents: docs.into_iter().map(|d| d.content).collect(),
                model: match self.model {
                    RerankModel::RerankLite1 => RerankModel::RerankLite1,
                    RerankModel::Rerank1 => RerankModel::Rerank1,
                },
                top_k: self.top_k,
                return_documents: Some(true),
                truncation: self.truncation,
            })
            .await
            .map(|result| {
                result
                    .data
                    .into_iter()
                    .map(|data| Similarity {
                        score: data.relevance_score as f32,
                        stored: StoredDocument {
                            id: data.index.to_string(),
                            document: Document {
                                content: data.document.unwrap(),
                                metadata: Default::default(),
                            },
                        },
                    })
                    .collect()
            })?)
    }
}

impl VoyageAiRerankerBuilder {
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

    pub fn with_model(mut self, model: RerankModel) -> Self {
        self.model = Some(model);
        self
    }

    pub fn with_top_k(mut self, top_k: u32) -> Self {
        self.top_k = Some(top_k);
        self
    }

    pub fn with_truncation(mut self, truncation: bool) -> Self {
        self.truncation = Some(truncation);
        self
    }

    pub fn build(self) -> Result<VoyageAiReranker> {
        Ok(VoyageAiReranker {
            client: self.voyageai_builder.build()?,
            model: self.model.ok_or_else(|| anyhow!("model is required"))?,
            top_k: self.top_k,
            truncation: self.truncation,
        })
    }
}
