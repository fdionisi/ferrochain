use thiserror::Error;

#[derive(Error, Debug)]
pub enum Neo4jGraphStoreError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] neo4rs::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    #[error("Operation failed: {0}")]
    OperationFailed(String),
}
