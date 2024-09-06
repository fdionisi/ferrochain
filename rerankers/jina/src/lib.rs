use ferrochain::{
    anyhow::{anyhow, Result},
    document::{Document, StoredDocument},
    reranker::Reranker,
    vectorstore::Similarity,
};
use jina_sdk::{DocumentType, Jina, JinaBuilder, QueryType, RerankRequest, RerankerModel};

pub struct JinaReranker {
    client: Jina,
    model: RerankerModel,
    top_n: Option<usize>,
}

pub struct JinaRerankerBuilder {
    jina_builder: JinaBuilder,
    model: Option<RerankerModel>,
    top_n: Option<usize>,
}

impl JinaReranker {
    pub fn builder() -> JinaRerankerBuilder {
        JinaRerankerBuilder {
            jina_builder: Jina::builder(),
            model: None,
            top_n: None,
        }
    }
}

impl JinaRerankerBuilder {
    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.jina_builder = self.jina_builder.api_key(api_key);
        self
    }

    pub fn with_base_url(mut self, api_key: String) -> Self {
        self.jina_builder = self.jina_builder.base_url(api_key);
        self
    }

    pub fn with_model(mut self, model: RerankerModel) -> Self {
        self.model = Some(model);
        self
    }

    pub fn with_top_n(mut self, top_n: usize) -> Self {
        self.top_n = Some(top_n);
        self
    }

    pub fn build(self) -> Result<JinaReranker> {
        Ok(JinaReranker {
            client: self.jina_builder.build()?,
            model: self.model.ok_or_else(|| anyhow!("model is required"))?,
            top_n: self.top_n,
        })
    }
}

#[ferrochain::async_trait]
impl Reranker for JinaReranker {
    async fn rerank(&self, query: &str, docs: Vec<Document>) -> Result<Vec<Similarity>> {
        Ok(self
            .client
            .rerank(RerankRequest {
                model: match self.model {
                    RerankerModel::RerankerV2BaseMultilingual => {
                        RerankerModel::RerankerV2BaseMultilingual
                    }
                    RerankerModel::RerankerV1BaseEn => RerankerModel::RerankerV1BaseEn,
                    RerankerModel::RerankerV1TinyEn => RerankerModel::RerankerV1TinyEn,
                    RerankerModel::RerankerV1TurboEn => RerankerModel::RerankerV1TurboEn,
                    RerankerModel::ColbertV1En => RerankerModel::ColbertV1En,
                },
                query: QueryType::String(query.into()),
                documents: DocumentType::Strings(docs.into_iter().map(|doc| doc.content).collect()),
                top_n: self.top_n,
                return_documents: Some(true),
            })
            .await
            .map(|response| {
                response
                    .results
                    .into_iter()
                    .map(|result| Similarity {
                        score: result.relevance_score,
                        stored: StoredDocument {
                            id: result.index.to_string(),
                            document: Document {
                                content: result.document.text,
                                metadata: Default::default(),
                            },
                        },
                    })
                    .collect()
            })?)
    }
}
