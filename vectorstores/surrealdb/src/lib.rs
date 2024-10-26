use std::sync::Arc;

use ferrochain::{
    anyhow::{anyhow, Result},
    document::{Document, StoredDocument},
    embedding::Embedder,
    vector_store::{Similarity, VectorStore},
};
use surrealdb::{engine::any::Any, RecordId, Surreal};
use uuid::Uuid;

pub struct SurrealVectorStore {
    client: Arc<Surreal<Any>>,
    collection_name: String,
    query_embedder: Arc<dyn Embedder>,
    document_embedder: Arc<dyn Embedder>,
    vector_size: u64,
}

#[derive(Clone)]
pub struct SurrealVectorStoreBuilder {
    client: Option<Arc<Surreal<Any>>>,
    collection_name: Option<String>,
    query_embedder: Option<Arc<dyn Embedder>>,
    document_embedder: Option<Arc<dyn Embedder>>,
    vector_size: Option<u64>,
}

impl SurrealVectorStore {
    pub fn builder() -> SurrealVectorStoreBuilder {
        SurrealVectorStoreBuilder {
            client: None,
            collection_name: None,
            query_embedder: None,
            document_embedder: None,
            vector_size: None,
        }
    }
}

#[ferrochain::async_trait]
impl VectorStore for SurrealVectorStore {
    async fn ensure_index(&self) -> Result<()> {
        let mut resp = self
            .client
            .query("DEFINE TABLE $table")
            .bind(("table", self.collection_name.clone()))
            .await?;
        resp.take::<Vec<()>>(0)?;

        let mut resp = self
            .client
            .query("DEFINE FIELD vector ON $table TYPE array VALUE $size")
            .bind(("table", self.collection_name.clone()))
            .bind(("size", self.vector_size))
            .await?;
        resp.take::<Vec<()>>(0)?;

        Ok(())
    }

    async fn add_documents(&self, documents: &[Document]) -> Result<()> {
        let vectors = self
            .document_embedder
            .embed(documents.iter().map(|d| d.content.clone()).collect())
            .await?;

        for (doc, vector) in documents.iter().zip(vectors) {
            let record_id =
                RecordId::from((self.collection_name.clone(), Uuid::new_v4().to_string()));
            let stored_doc = StoredDocument {
                id: record_id.to_string(),
                document: doc.clone(),
            };
            let mut resp = self.client
                .query("CREATE type::thing($table, $id) SET vector = $vector, content = $content, metadata = $metadata")
                .bind(("table", self.collection_name.clone()))
                .bind(("id", stored_doc.id.clone()))
                .bind(("vector", vector))
                .bind(("content", stored_doc.document.content.clone()))
                .bind(("metadata", stored_doc.document.metadata.clone()))
                .await?;
            resp.take::<Vec<()>>(0)?;
        }

        Ok(())
    }

    async fn delete_documents(&self, ids: &[String]) -> Result<()> {
        for id in ids.to_vec().into_iter() {
            let mut resp = self
                .client
                .query("DELETE FROM $table WHERE id = $id")
                .bind(("table", self.collection_name.clone()))
                .bind(("id", id))
                .await?;
            resp.take::<Vec<()>>(0)?;
        }
        Ok(())
    }

    async fn get_documents(&self, ids: &[String]) -> Result<Vec<StoredDocument>> {
        let mut documents = Vec::new();

        for id in ids.to_vec().into_iter() {
            let mut result = self
                .client
                .query("SELECT * FROM $table WHERE id = $id")
                .bind(("table", self.collection_name.clone()))
                .bind(("id", id.clone()))
                .await?;

            let Some(stored_doc): Option<StoredDocument> = result.take(0)? else {
                continue;
            };

            documents.push(stored_doc);
        }

        Ok(documents)
    }

    async fn search(&self, query: &str, limit: u64) -> Result<Vec<Similarity>> {
        let embedded_queries = self.query_embedder.embed(vec![query.to_string()]).await?;
        let embedded_query = embedded_queries.first().take().unwrap();

        let mut results = self.client
            .query("SELECT *, vector::similarity(vector, $query) as score FROM $table ORDER BY score DESC LIMIT $limit")
            .bind(("table", self.collection_name.clone()))
            .bind(("query", embedded_query.to_vec()))
            .bind(("limit", limit))
            .await?;

        let documents: Vec<(StoredDocument, f32)> = results.take(0)?;

        Ok(documents
            .into_iter()
            .map(|(doc, score)| Similarity { stored: doc, score })
            .collect())
    }
}

impl SurrealVectorStoreBuilder {
    pub fn with_client(mut self, client: Arc<Surreal<Any>>) -> Self {
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

    pub fn build(self) -> Result<SurrealVectorStore> {
        Ok(SurrealVectorStore {
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
