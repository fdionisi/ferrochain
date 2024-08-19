use std::{sync::Arc, u64};

use anyhow::Result;
use async_trait::async_trait;
use convert_case::Casing;
use indoc::formatdoc;

use crate::{
    document::{Document, StoredDocument},
    retriever::Retriever,
    tool::Tool,
};

#[derive(Clone, Debug, schemars::JsonSchema, serde::Serialize)]
pub struct Similarlity {
    #[serde(flatten)]
    pub stored: StoredDocument,
    pub score: f32,
}

#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn add_documents(&self, documents: &[Document]) -> Result<()>;
    async fn delete_documents(&self, ids: &[String]) -> Result<()>;
    async fn get_documents(&self, ids: &[String]) -> Result<Vec<StoredDocument>>;
    async fn search(&self, query: &str, limit: u64) -> Result<Vec<Similarlity>>;
}

#[async_trait]
impl<T: VectorStore> Retriever for T {
    async fn retrieve(&self, query: &str) -> Result<Vec<Document>> {
        let docs = self.search(query, u64::MAX).await?;
        Ok(docs.into_iter().map(|doc| doc.stored.document).collect())
    }
}

impl From<Similarlity> for StoredDocument {
    fn from(similarity_document: Similarlity) -> StoredDocument {
        similarity_document.stored
    }
}

pub struct VectorStoreTool {
    vector_store: Arc<dyn VectorStore>,
    name: String,
    description: String,
}

pub struct VectorStoreToolBuilder {
    vector_store: Option<Arc<dyn VectorStore>>,
    name: Option<String>,
    description: Option<String>,
}

impl VectorStoreTool {
    pub fn builder() -> VectorStoreToolBuilder {
        VectorStoreToolBuilder {
            vector_store: None,
            name: None,
            description: None,
        }
    }
}

impl VectorStoreToolBuilder {
    pub fn vector_store(mut self, vector_store: Arc<dyn VectorStore>) -> Self {
        self.vector_store = Some(vector_store);
        self
    }

    pub fn name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn build(self) -> VectorStoreTool {
        VectorStoreTool {
            vector_store: self.vector_store.unwrap(),
            name: self.name.unwrap(),
            description: self.description.unwrap(),
        }
    }
}

#[derive(schemars::JsonSchema, serde::Deserialize)]
pub struct VectorStoreToolInput {
    query: String,
    limit: u64,
}

#[async_trait]
impl Tool for VectorStoreTool {
    type Input = VectorStoreToolInput;
    type Output = Vec<Similarlity>;

    fn name(&self) -> String {
        format!(
            "vector_store_{}",
            self.name.to_case(convert_case::Case::Snake)
        )
    }

    fn description(&self) -> String {
        formatdoc! {"
            Useful for when you need to answer questions about {} and the sources used to construct the answer.
            Whenever you need information about {} you should ALWAYS use this.
            Input should be a fully formed question.

            Output is a json serialized dictionary list as follows:
            [{{ \"id\": \"...\", \"content\": \"...\", \"metadata\": {{ ... }}, \"score\": ... }}, ...]
        ", self.name, self.description}.into()
    }

    async fn execute(&self, input: serde_json::Value) -> Result<String> {
        let input: Self::Input = serde_json::from_value(input)?;
        let response = self.vector_store.search(&input.query, input.limit).await?;
        Ok(serde_json::to_string(&response)?)
    }
}
