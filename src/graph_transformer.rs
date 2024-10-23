use anyhow::Result;

use crate::{document::Document, graph_store::GraphDocument};

#[async_trait::async_trait]
pub trait GraphTransformer {
    async fn convert_to_graph_document(&self, docs: Vec<Document>) -> Result<Vec<GraphDocument>>;
}
