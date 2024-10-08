use std::sync::Arc;

use ferrochain::{
    anyhow::{anyhow, Result},
    document::{Document, StoredDocument},
    embedding::Embedder,
    vectorstore::{Similarity, VectorStore},
};
pub use qdrant_client;
use qdrant_client::{
    qdrant::{PointStruct, ScoredPoint, SearchPointsBuilder, UpsertPointsBuilder},
    Payload, Qdrant,
};
use serde_json::json;
use uuid::Uuid;

pub struct QdrantVectorStore {
    client: Qdrant,
    collection_name: String,
    query_embedder: Arc<dyn Embedder>,
    document_embedder: Arc<dyn Embedder>,
}

pub struct QdrantVectorStoreBuilder {
    client: Option<Qdrant>,
    collection_name: Option<String>,
    query_embedder: Option<Arc<dyn Embedder>>,
    document_embedder: Option<Arc<dyn Embedder>>,
}

impl QdrantVectorStore {
    pub fn builder() -> QdrantVectorStoreBuilder {
        QdrantVectorStoreBuilder {
            client: None,
            collection_name: None,
            query_embedder: None,
            document_embedder: None,
        }
    }
}

#[ferrochain::async_trait]
impl VectorStore for QdrantVectorStore {
    async fn add_documents(&self, documents: &[Document]) -> Result<()> {
        let vectors = self
            .document_embedder
            .embed(documents.iter().map(|d| d.content.clone()).collect())
            .await?;

        let points = documents
            .into_iter()
            .zip(vectors)
            .map(|(Document { content, metadata }, vector)| {
                let id = Uuid::new_v4();
                PointStruct::new(
                    id.to_string(),
                    vector.to_vec(),
                    Payload::try_from(json!({
                        "id": id,
                        "content": content,
                        "metadata": metadata
                    }))
                    .unwrap(),
                )
            })
            .collect::<Vec<PointStruct>>();

        self.client
            .upsert_points(UpsertPointsBuilder::new(&self.collection_name, points).wait(true))
            .await?;

        Ok(())
    }

    async fn delete_documents(&self, _ids: &[String]) -> Result<()> {
        todo!()
    }

    async fn get_documents(&self, _ids: &[String]) -> Result<Vec<StoredDocument>> {
        todo!()
    }

    async fn search(&self, query: &str, limit: u64) -> Result<Vec<Similarity>> {
        let embedded_query = self.query_embedder.embed(vec![query.to_string()]).await?;
        let embedded_query = embedded_query.first().unwrap();

        let search_response = self
            .client
            .search_points(
                SearchPointsBuilder::new(&self.collection_name, embedded_query.to_vec(), limit)
                    .with_payload(true),
            )
            .await?;

        let documents = search_response
            .result
            .into_iter()
            .map(|ScoredPoint { payload, score, .. }| Similarity {
                stored: StoredDocument {
                    id: serde_json::from_value::<Uuid>(payload["id"].clone().into_json())
                        .unwrap()
                        .to_string(),
                    document: Document {
                        content: payload["content"].as_str().unwrap().to_string(),
                        metadata: serde_json::from_value(payload["metadata"].clone().into_json())
                            .unwrap(),
                    },
                },
                score,
            })
            .collect();

        Ok(documents)
    }
}

impl QdrantVectorStoreBuilder {
    pub fn client(mut self, client: Qdrant) -> Self {
        self.client = Some(client);
        self
    }

    pub fn collection_name(mut self, collection_name: String) -> Self {
        self.collection_name = Some(collection_name);
        self
    }

    pub fn embedder(self, embedder: Arc<dyn Embedder>) -> Self {
        self.query_embedder(embedder.clone())
            .document_embedder(embedder.clone())
    }

    pub fn query_embedder(mut self, embedder: Arc<dyn Embedder>) -> Self {
        self.query_embedder = Some(embedder);
        self
    }

    pub fn document_embedder(mut self, embedder: Arc<dyn Embedder>) -> Self {
        self.document_embedder = Some(embedder);
        self
    }

    pub fn build(self) -> Result<QdrantVectorStore> {
        Ok(QdrantVectorStore {
            client: self.client.ok_or_else(|| anyhow!("client is required"))?,
            collection_name: self
                .collection_name
                .ok_or_else(|| anyhow!("collection_name is required"))?,
            query_embedder: self
                .query_embedder
                .ok_or_else(|| anyhow!("query_embedder is required"))?,
            document_embedder: self
                .document_embedder
                .ok_or_else(|| anyhow!("document_embedder is required"))?,
        })
    }
}
