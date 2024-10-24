use std::{collections::HashMap, sync::Arc};

pub use exa_sdk::SearchKind;
use exa_sdk::{Exa, ExaBuilder, SearchContent, SearchContentText, SearchRequest};
use ferrochain::{anyhow::Result, document::Document, retriever::Retriever};
use http_client::HttpClient;

pub struct ExaRetriever {
    client: Exa,
    use_autoprompt: Option<bool>,
    kind: Option<SearchKind>,
    num_results: Option<u32>,
    include_domains: Option<Vec<String>>,
    exclude_domains: Option<Vec<String>>,
}

pub struct ExaRetrieverBuilder {
    exa_builder: ExaBuilder,
    use_autoprompt: Option<bool>,
    kind: Option<SearchKind>,
    num_results: Option<u32>,
    include_domains: Option<Vec<String>>,
    exclude_domains: Option<Vec<String>>,
}

impl ExaRetriever {
    pub fn builder() -> ExaRetrieverBuilder {
        ExaRetrieverBuilder {
            exa_builder: Exa::builder(),
            use_autoprompt: None,
            kind: None,
            num_results: None,
            include_domains: None,
            exclude_domains: None,
        }
    }
}

#[ferrochain::async_trait]
impl Retriever for ExaRetriever {
    async fn retrieve(&self, query: &str) -> Result<Vec<Document>> {
        Ok(self
            .client
            .search(SearchRequest {
                query: query.into(),
                contents: Some(SearchContent {
                    text: Some(SearchContentText {
                        include_html_tags: Some(false),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                use_autoprompt: self.use_autoprompt,
                kind: self.kind.clone(),
                num_results: self.num_results,
                include_domains: self.include_domains.clone(),
                exclude_domains: self.exclude_domains.clone(),
                // start_crawl_date: (),
                // end_crawl_date: (),
                // start_published_date: (),
                // end_published_date: (),
                // include_text: (),
                // exclude_text: (),
                ..Default::default()
            })
            .await
            .map(|response| {
                response
                    .results
                    .into_iter()
                    .map(|result| {
                        let mut metadata = HashMap::new();
                        if let Some(author) = result.author {
                            if !author.is_empty() {
                                metadata.insert(
                                    "author".into(),
                                    serde_json::to_value(&author).unwrap(),
                                );
                            }
                        }

                        if let Some(published_date) = result.published_date {
                            if !published_date.is_empty() {
                                metadata.insert(
                                    "published_date".into(),
                                    serde_json::to_value(&published_date).unwrap(),
                                );
                            }
                        }

                        if let Some(score) = result.score {
                            metadata.insert("score".into(), serde_json::to_value(&score).unwrap());
                        }

                        if let Some(highlights) = result.highlights {
                            metadata.insert(
                                "highlights".into(),
                                serde_json::to_value(&highlights).unwrap(),
                            );
                        }

                        if let Some(highlight_scores) = result.highlight_scores {
                            metadata.insert(
                                "highlight_scores".into(),
                                serde_json::to_value(&highlight_scores).unwrap(),
                            );
                        }

                        metadata.insert("id".into(), serde_json::to_value(&result.id).unwrap());
                        metadata.insert("url".into(), serde_json::to_value(&result.url).unwrap());

                        Document {
                            content: result.text.unwrap(),
                            metadata,
                        }
                    })
                    .collect()
            })?)
    }
}

impl ExaRetrieverBuilder {
    pub fn with_http_client(mut self, http_client: Arc<dyn HttpClient>) -> Self {
        self.exa_builder = self.exa_builder.with_http_client(http_client);
        self
    }

    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.exa_builder = self.exa_builder.with_api_key(api_key);
        self
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.exa_builder = self.exa_builder.with_base_url(base_url);
        self
    }

    pub fn with_use_autoprompt(mut self, use_autoprompt: bool) -> Self {
        self.use_autoprompt = Some(use_autoprompt);
        self
    }

    pub fn with_kind(mut self, kind: SearchKind) -> Self {
        self.kind = Some(kind);
        self
    }

    pub fn with_num_results(mut self, num_results: u32) -> Self {
        self.num_results = Some(num_results);
        self
    }

    pub fn with_include_domains(mut self, include_domains: Vec<String>) -> Self {
        self.include_domains = Some(include_domains);
        self
    }

    pub fn with_exclude_domains(mut self, exclude_domains: Vec<String>) -> Self {
        self.exclude_domains = Some(exclude_domains);
        self
    }

    pub fn build(self) -> Result<ExaRetriever> {
        Ok(ExaRetriever {
            client: self.exa_builder.build()?,
            use_autoprompt: self.use_autoprompt,
            kind: self.kind,
            num_results: self.num_results,
            include_domains: self.include_domains,
            exclude_domains: self.exclude_domains,
        })
    }
}
