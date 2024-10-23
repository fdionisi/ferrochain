use anyhow::Result;
use serde_json::Value;

use crate::document::Document;

#[async_trait::async_trait]
pub trait GraphStore {
    async fn add_graph_documents(&self, docs: Vec<GraphDocument>) -> Result<()>;
    async fn query(&self, query: &str, params: Option<Value>) -> Result<Vec<GraphDocument>>;
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Node {
    pub id: String,
    pub kind: String,
    pub properties: Value,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Relationship {
    pub source: Node,
    pub target: Node,
    pub kind: String,
    pub properties: Value,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GraphDocument {
    pub nodes: Vec<Node>,
    pub relationships: Vec<Relationship>,
    #[serde(flatten)]
    pub document: Option<Document>,
}

impl TryFrom<Document> for GraphDocument {
    type Error = anyhow::Error;

    fn try_from(value: Document) -> Result<Self, Self::Error> {
        Ok(serde_json::from_value(serde_json::to_value(&value)?)?)
    }
}
