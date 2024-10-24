use std::collections::HashMap;

use ferrochain::{anyhow::Result, document::Document, retriever::Retriever};
pub use tavily_sdk::search::{SearchDepth, Topic};
use tavily_sdk::{search::TavilySearchParams, Tavily, TavilyBuilder};

pub struct TavilyRetriever {
    client: Tavily,
    search_depth: Option<SearchDepth>,
    topic: Option<Topic>,
    include_domains: Option<Vec<String>>,
    exclude_domains: Option<Vec<String>>,
    days: Option<u32>,
    max_results: Option<u32>,
    include_images: Option<bool>,
    include_answer: Option<bool>,
    include_raw_content: Option<bool>,
}

pub struct TavilyRetrieverBuilder {
    tavily_builder: TavilyBuilder,
    search_depth: Option<SearchDepth>,
    topic: Option<Topic>,
    include_domains: Option<Vec<String>>,
    exclude_domains: Option<Vec<String>>,
    days: Option<u32>,
    max_results: Option<u32>,
    include_images: Option<bool>,
    include_answer: Option<bool>,
    include_raw_content: Option<bool>,
}

impl TavilyRetriever {
    pub fn builder() -> TavilyRetrieverBuilder {
        TavilyRetrieverBuilder {
            tavily_builder: Tavily::builder(),
            search_depth: None,
            topic: None,
            include_domains: None,
            exclude_domains: None,
            days: None,
            max_results: None,
            include_images: None,
            include_answer: None,
            include_raw_content: None,
        }
    }
}

#[ferrochain::async_trait]
impl Retriever for TavilyRetriever {
    async fn retrieve(&self, query: &str) -> Result<Vec<Document>> {
        Ok(self
            .client
            .search(TavilySearchParams {
                query: query.into(),
                search_depth: self.search_depth.clone(),
                topic: self.topic.clone(),
                days: self.days,
                max_results: self.max_results,
                include_images: self.include_images,
                include_answer: self.include_answer,
                include_raw_content: self.include_raw_content,
                include_domains: self.include_domains.clone(),
                exclude_domains: self.exclude_domains.clone(),
            })
            .await
            .map(|response| {
                let mut documents = vec![];

                if let Some(answer) = response.answer {
                    let mut metadata = HashMap::new();
                    metadata.insert("type".into(), "answer".into());

                    documents.push(Document {
                        content: answer,
                        metadata,
                    });
                }

                if let Some(images) = response.images {
                    let mut metadata = HashMap::new();
                    metadata.insert("type".into(), "image".into());

                    for image in images {
                        documents.push(Document {
                            content: image,
                            metadata: metadata.clone(),
                        });
                    }
                }

                for result in response.results {
                    let mut metadata = HashMap::new();
                    metadata.insert("type".into(), "document".into());
                    metadata.insert("url".into(), result.url.into());
                    metadata.insert("title".into(), result.title.into());
                    metadata.insert("published_date".into(), result.published_date.into());
                    metadata.insert("raw_content".into(), result.raw_content.into());

                    documents.push(Document {
                        content: result.content.clone(),
                        metadata,
                    });
                }

                documents
            })?)
    }
}

impl TavilyRetrieverBuilder {
    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.tavily_builder = self.tavily_builder.api_key(api_key);
        self
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.tavily_builder = self.tavily_builder.base_url(base_url);
        self
    }

    pub fn with_search_depth(mut self, search_depth: SearchDepth) -> Self {
        self.search_depth = Some(search_depth);
        self
    }

    pub fn with_topic(mut self, topic: Topic) -> Self {
        self.topic = Some(topic);
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

    pub fn with_days(mut self, days: u32) -> Self {
        self.days = Some(days);
        self
    }

    pub fn with_max_results(mut self, max_results: u32) -> Self {
        self.max_results = Some(max_results);
        self
    }

    pub fn with_include_images(mut self, include_images: bool) -> Self {
        self.include_images = Some(include_images);
        self
    }

    pub fn with_include_answer(mut self, include_answer: bool) -> Self {
        self.include_answer = Some(include_answer);
        self
    }

    pub fn with_include_raw_content(mut self, include_raw_content: bool) -> Self {
        self.include_raw_content = Some(include_raw_content);
        self
    }

    pub fn build(self) -> Result<TavilyRetriever> {
        Ok(TavilyRetriever {
            client: self.tavily_builder.build()?,
            search_depth: self.search_depth,
            topic: self.topic,
            include_domains: self.include_domains,
            exclude_domains: self.exclude_domains,
            days: self.days,
            max_results: self.max_results,
            include_images: self.include_images,
            include_answer: self.include_answer,
            include_raw_content: self.include_raw_content,
        })
    }
}
