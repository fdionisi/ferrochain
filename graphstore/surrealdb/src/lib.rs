use ferrochain::anyhow::{anyhow, Result};
use ferrochain::async_trait;
use ferrochain::graph_store::{GraphDocument, GraphStore};
use serde_json::{json, Value};
use std::sync::Arc;
use surrealdb::Surreal;

pub struct SurrealGraphStore {
    db: Arc<Surreal<surrealdb::engine::any::Any>>,
}

pub struct SurrealGraphStoreBuilder {
    db: Option<Arc<Surreal<surrealdb::engine::any::Any>>>,
}

impl SurrealGraphStore {
    pub fn builder() -> SurrealGraphStoreBuilder {
        SurrealGraphStoreBuilder { db: None }
    }

    pub async fn clear_database(&self) -> Result<()> {
        self.db.query("REMOVE DATABASE").await?;
        Ok(())
    }
}

#[async_trait]
impl GraphStore for SurrealGraphStore {
    async fn add_graph_documents(&self, docs: Vec<GraphDocument>) -> Result<()> {
        let mut all_nodes = Vec::new();
        let mut all_relationships = Vec::new();

        for doc in docs {
            all_nodes.extend(doc.nodes);
            all_relationships.extend(doc.relationships);
        }

        if !all_nodes.is_empty() {
            let query = "
                LET $nodes = $nodes;
                FOR node IN $nodes {
                    CREATE type::table(node.kind)
                    CONTENT {
                        id: node.id,
                        kind: node.kind,
                        ...node.properties
                    }
                };
            ";

            self.db
                .query(query)
                .bind(("nodes", json!(all_nodes)))
                .await?;
        }

        if !all_relationships.is_empty() {
            let query = "
                LET $relationships = $relationships;
                FOR rel IN $relationships {
                    RELATE (SELECT FROM type::table(rel.source.kind) WHERE id = rel.source.id)
                    ->${rel.kind}->
                    (SELECT FROM type::table(rel.target.kind) WHERE id = rel.target.id)
                    SET properties = rel.properties;
                };
            ";

            self.db
                .query(query)
                .bind(("relationships", json!(all_relationships)))
                .await?;
        }

        Ok(())
    }

    async fn query(&self, query: &str, params: Option<Value>) -> Result<Vec<GraphDocument>> {
        let mut query_builder = self.db.query(query);

        if let Some(params) = params {
            if let Value::Object(map) = params {
                for (key, value) in map {
                    query_builder = query_builder.bind((key, value));
                }
            } else {
                return Err(anyhow!("Parameters must be a JSON object"));
            }
        }

        let mut result = query_builder.await?;
        let response: Vec<GraphDocument> = result.take(0)?;
        Ok(response)
    }
}

impl SurrealGraphStoreBuilder {
    pub fn with_surrealdb(mut self, db: Arc<Surreal<surrealdb::engine::any::Any>>) -> Self {
        self.db = Some(db);
        self
    }

    pub async fn build(self) -> Result<SurrealGraphStore> {
        let db = self
            .db
            .ok_or_else(|| anyhow!("Database connection is required"))?;

        Ok(SurrealGraphStore { db })
    }
}
