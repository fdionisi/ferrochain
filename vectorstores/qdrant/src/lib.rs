use std::sync::Arc;

use ferrochain::{
    anyhow::{anyhow, Result},
    document::{Document, StoredDocument},
    embedding::Embedder,
    vector_store::{Similarity, VectorStore},
};
pub use qdrant_client;
use qdrant_client::{
    qdrant::{
        CreateCollectionBuilder, DeletePointsBuilder, Distance, GetPointsBuilder, PointId,
        PointStruct, ScoredPoint, SearchPointsBuilder, UpsertPointsBuilder, VectorParamsBuilder,
    },
    Payload, Qdrant,
};
use serde_json::json;
use uuid::Uuid;

pub struct QdrantVectorStore {
    client: Arc<Qdrant>,
    collection_name: String,
    query_embedder: Arc<dyn Embedder>,
    document_embedder: Arc<dyn Embedder>,
    vector_size: u64,
}

#[derive(Clone)]
pub struct QdrantVectorStoreBuilder {
    client: Option<Arc<Qdrant>>,
    collection_name: Option<String>,
    query_embedder: Option<Arc<dyn Embedder>>,
    document_embedder: Option<Arc<dyn Embedder>>,
    vector_size: Option<u64>,
}

impl QdrantVectorStore {
    pub fn builder() -> QdrantVectorStoreBuilder {
        QdrantVectorStoreBuilder {
            client: None,
            collection_name: None,
            query_embedder: None,
            document_embedder: None,
            vector_size: None,
        }
    }
}

#[ferrochain::async_trait]
impl VectorStore for QdrantVectorStore {
    async fn ensure_index(&self) -> Result<()> {
        if !self.client.collection_exists(&self.collection_name).await? {
            self.client
                .create_collection(
                    CreateCollectionBuilder::new(&self.collection_name)
                        .vectors_config(
                            VectorParamsBuilder::new(self.vector_size, Distance::Cosine).build(),
                        )
                        .build(),
                )
                .await?;
        }
        Ok(())
    }

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

    async fn delete_documents(&self, ids: &[String]) -> Result<()> {
        self.client
            .delete_points(
                DeletePointsBuilder::new(&self.collection_name)
                    .points(ids.to_vec())
                    .wait(true),
            )
            .await?;
        Ok(())
    }

    async fn get_documents(&self, ids: &[String]) -> Result<Vec<StoredDocument>> {
        let points = self
            .client
            .get_points(
                GetPointsBuilder::new(
                    &self.collection_name,
                    ids.into_iter()
                        .map(|id| id.to_owned().into())
                        .collect::<Vec<PointId>>(),
                )
                .with_vectors(true)
                .with_payload(true),
            )
            .await?;

        let documents = points
            .result
            .into_iter()
            .map(|point| StoredDocument {
                id: serde_json::from_value::<Uuid>(point.payload["id"].clone().into_json())
                    .unwrap()
                    .to_string(),
                document: Document {
                    content: point.payload["content"].as_str().unwrap().to_string(),
                    metadata: serde_json::from_value(point.payload["metadata"].clone().into_json())
                        .unwrap(),
                },
            })
            .collect();

        Ok(documents)
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
    pub fn with_client(mut self, client: Arc<Qdrant>) -> Self {
        self.client = Some(client);
        self
    }

    pub fn with_collection_name<S>(mut self, collection_name: S) -> Self
    where
        S: Into<String>,
    {
        self.collection_name = Some(collection_name.into());
        self
    }

    pub fn with_embedder(self, embedder: Arc<dyn Embedder>) -> Self {
        self.with_query_embedder(embedder.clone())
            .with_document_embedder(embedder.clone())
    }

    pub fn with_query_embedder(mut self, embedder: Arc<dyn Embedder>) -> Self {
        self.query_embedder = Some(embedder);
        self
    }

    pub fn with_document_embedder(mut self, embedder: Arc<dyn Embedder>) -> Self {
        self.document_embedder = Some(embedder);
        self
    }

    pub fn with_vector_size(mut self, vector_size: u64) -> Self {
        self.vector_size = Some(vector_size);
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
            vector_size: self
                .vector_size
                .ok_or_else(|| anyhow!("vector_size is required"))?,
        })
    }
}
