pub mod error;

use ferrochain::anyhow::{anyhow, Result};
use ferrochain::async_trait;
use ferrochain::graph_store::{GraphDocument, GraphStore};
use neo4rs::{BoltMap, BoltString, BoltType, Graph};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

pub struct Neo4jGraphStore {
    graph: Arc<Graph>,
}

pub struct Neo4jGraphStoreBuilder {
    url: Option<String>,
    username: Option<String>,
    password: Option<String>,
}

impl Neo4jGraphStore {
    pub fn builder() -> Neo4jGraphStoreBuilder {
        Neo4jGraphStoreBuilder {
            url: None,
            username: None,
            password: None,
        }
    }

    pub async fn clear_database(&self) -> Result<()> {
        let query = "MATCH (n) DETACH DELETE n";
        self.graph
            .run(neo4rs::query(query))
            .await
            .map_err(|e| anyhow!("Failed to clear database: {}", e))?;
        Ok(())
    }
}

#[async_trait]
impl GraphStore for Neo4jGraphStore {
    async fn add_graph_documents(&self, docs: Vec<GraphDocument>) -> Result<()> {
        let mut all_nodes = Vec::new();
        let mut all_relationships = Vec::new();

        for doc in docs {
            all_nodes.extend(doc.nodes);
            all_relationships.extend(doc.relationships);
        }

        if !all_nodes.is_empty() {
            let query = "
                    UNWIND $nodes AS node
                    CREATE (n)
                    SET n = node.properties
                    WITH n, node
                    CALL apoc.create.addLabels(n, [node.kind] + node.additional_labels) YIELD node as _
                    RETURN count(*)
                ";
            let params = serde_json::json!({
                "nodes": all_nodes.iter().map(|node| {
                    let mut properties = node.properties.clone();
                    properties.as_object_mut().unwrap().insert("id".to_string(), node.id.clone().into());
                    serde_json::json!({
                        "kind": node.kind,
                        "properties": properties,
                        "additional_labels": node.properties.get("labels").cloned().unwrap_or(Value::Array(vec![]))
                    })
                }).collect::<Vec<_>>()
            });
            let params: BoltType = params.try_into()?;
            self.graph
                .run(neo4rs::query(query).param("nodes", params))
                .await
                .map_err(|e| anyhow!("Failed to batch insert nodes: {}", e))?;
        }

        if !all_relationships.is_empty() {
            let query = "
                UNWIND $relationships AS rel
                MATCH (source:Node {id: rel.source_id}), (target:Node {id: rel.target_id})
                CREATE (source)-[r:`rel.kind`]->(target)
                SET r = rel.properties
            ";
            let params = serde_json::json!({
                "relationships": all_relationships.iter().map(|rel| {
                    serde_json::json!({
                        "source_id": rel.source.id,
                        "target_id": rel.target.id,
                        "kind": rel.kind,
                        "properties": rel.properties
                    })
                }).collect::<Vec<_>>()
            });
            let params: BoltType = params.try_into()?;
            self.graph
                .run(neo4rs::query(query).param("relationships", params))
                .await
                .map_err(|e| anyhow!("Failed to batch insert relationships: {}", e))?;
        }

        Ok(())
    }

    async fn query(&self, query: &str, params: Option<Value>) -> Result<Vec<GraphDocument>> {
        let mut query_builder = neo4rs::query(query);

        if let Some(params) = params {
            if let Value::Object(map) = params {
                for (key, value) in map {
                    let value: BoltType = value.try_into()?;
                    query_builder = query_builder.param(&key, value);
                }
            } else {
                return Err(anyhow!("Parameters must be a JSON object"));
            }
        }

        let mut result = self
            .graph
            .execute(query_builder)
            .await
            .map_err(|e| anyhow!("Failed to execute query: {}", e))?;
        let mut docs = Vec::new();

        while let Some(row) = result.next().await? {
            let mut nodes = Vec::new();
            let mut relationships = Vec::new();

            for (_, value) in row.to::<HashMap<BoltString, BoltType>>()? {
                match value {
                    BoltType::Node(node) => {
                        nodes.push(ferrochain::graph_store::Node {
                            id: node.id.value.to_string(),
                            kind: node
                                .labels
                                .value
                                .first()
                                .cloned()
                                .map(|l| l.to_string())
                                .unwrap_or_default(),
                            properties: bolt_map_to_json_value(&node.properties),
                        });
                    }
                    BoltType::Relation(rel) => {
                        relationships.push(ferrochain::graph_store::Relationship {
                            source: ferrochain::graph_store::Node {
                                id: rel.start_node_id.value.to_string(),
                                kind: String::new(),
                                properties: Value::Null,
                            },
                            target: ferrochain::graph_store::Node {
                                id: rel.end_node_id.value.to_string(),
                                kind: String::new(),
                                properties: Value::Null,
                            },
                            kind: rel.typ.value,
                            properties: bolt_map_to_json_value(&rel.properties),
                        });
                    }
                    _ => {}
                }
            }

            docs.push(GraphDocument {
                document: None,
                nodes,
                relationships,
            });
        }

        Ok(docs)
    }
}

impl Neo4jGraphStoreBuilder {
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    pub fn with_username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    pub async fn build(self) -> Result<Neo4jGraphStore, anyhow::Error> {
        let url = self.url.ok_or_else(|| anyhow::anyhow!("URL is required"))?;
        let username = self
            .username
            .ok_or_else(|| anyhow::anyhow!("Username is required"))?;
        let password = self
            .password
            .ok_or_else(|| anyhow::anyhow!("Password is required"))?;

        let graph = Graph::new(&url, username, password).await?;
        Ok(Neo4jGraphStore {
            graph: Arc::new(graph),
        })
    }
}

fn bolt_map_to_json_value(bolt_map: &BoltMap) -> serde_json::Value {
    let mut json_map = serde_json::Map::with_capacity(bolt_map.len());
    for (key, value) in bolt_map.value.iter() {
        json_map.insert(key.value.to_string(), bolt_type_to_json_value(&value));
    }
    serde_json::Value::Object(json_map)
}

fn bolt_type_to_json_value(bolt_type: &BoltType) -> serde_json::Value {
    match bolt_type {
        BoltType::Null(_) => serde_json::Value::Null,
        BoltType::Boolean(bolt_boolean) => serde_json::Value::Bool(bolt_boolean.value),
        BoltType::Integer(bolt_integer) => serde_json::Value::Number(bolt_integer.value.into()),
        BoltType::Float(bolt_float) => json!(bolt_float.value),
        BoltType::String(bolt_string) => serde_json::Value::String(bolt_string.value.to_string()),
        BoltType::List(bolt_list) => {
            let json_values: Vec<serde_json::Value> =
                bolt_list.iter().map(bolt_type_to_json_value).collect();
            serde_json::Value::Array(json_values)
        }
        BoltType::Map(bolt_map) => bolt_map_to_json_value(bolt_map),
        _ => serde_json::Value::Null,
        // BoltType::Node(bolt_node) => todo!(),
        // BoltType::Relation(bolt_relation) => todo!(),
        // BoltType::UnboundedRelation(bolt_unbounded_relation) => todo!(),
        // BoltType::Point2D(bolt_point2_d) => todo!(),
        // BoltType::Point3D(bolt_point3_d) => todo!(),
        // BoltType::Bytes(bolt_bytes) => todo!(),
        // BoltType::Path(bolt_path) => todo!(),
        // BoltType::Duration(bolt_duration) => todo!(),
        // BoltType::Date(bolt_date) => todo!(),
        // BoltType::Time(bolt_time) => todo!(),
        // BoltType::LocalTime(bolt_local_time) => todo!(),
        // BoltType::DateTime(bolt_date_time) => todo!(),
        // BoltType::LocalDateTime(bolt_local_date_time) => todo!(),
        // BoltType::DateTimeZoneId(bolt_date_time_zone_id) => todo!(),
    }
}
