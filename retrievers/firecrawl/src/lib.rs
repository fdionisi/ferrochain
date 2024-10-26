use std::collections::HashMap;

use ferrochain::{
    anyhow::{anyhow, Result},
    document::Document,
    retriever::Retriever,
};
pub use firecrawl::scrape::ScrapeFormats;
use firecrawl::{scrape::ScrapeOptions, FirecrawlApp};

pub struct FirecrawlRetriever {
    client: FirecrawlApp,
    formats: Option<Vec<ScrapeFormats>>,
}

pub struct FirecrawlRetrieverBuilder {
    api_key: Option<String>,
    formats: Option<Vec<ScrapeFormats>>,
}

impl FirecrawlRetriever {
    pub fn builder() -> FirecrawlRetrieverBuilder {
        FirecrawlRetrieverBuilder {
            api_key: None,
            formats: None,
        }
    }
}

#[ferrochain::async_trait]
impl Retriever for FirecrawlRetriever {
    async fn retrieve(&self, query: &str) -> Result<Vec<Document>> {
        let options = ScrapeOptions {
            formats: self
                .formats
                .clone()
                .unwrap_or_else(|| vec![ScrapeFormats::Markdown])
                .into(),
            ..Default::default()
        };

        Ok(self
            .client
            .scrape_url(query, options)
            .await
            .map(|response| {
                vec![Document {
                    content: response.markdown.unwrap_or_default(),
                    metadata: {
                        let mut metadata = HashMap::new();
                        metadata.insert("url".into(), serde_json::to_value(query).unwrap());

                        metadata
                    },
                }]
            })?)
    }
}

impl FirecrawlRetrieverBuilder {
    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }

    pub fn with_formats(mut self, formats: Vec<ScrapeFormats>) -> Self {
        self.formats = Some(formats);
        self
    }

    pub fn build(self) -> Result<FirecrawlRetriever> {
        let api_key = self.api_key.ok_or_else(|| anyhow!("API key is required"))?;
        let client = FirecrawlApp::new(&api_key)
            .map_err(|e| anyhow!("Failed to initialize FirecrawlApp: {}", e))?;

        Ok(FirecrawlRetriever {
            client,
            formats: self.formats,
        })
    }
}
