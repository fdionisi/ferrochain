use std::collections::HashMap;

use serde_json::Value;

/// A `Document` object contains information about some data. It has two attributes:
/// - `content`: a string containing the data itself.
/// - `metadata`: a map of key-value pairs containing additional information about the data.
#[derive(Clone, Debug, schemars::JsonSchema, serde::Serialize)]
pub struct Document {
    /// The plain-text representation of the document.
    pub content: String,
    /// Additional and arbitrary metadata about the document.
    pub metadata: HashMap<String, Value>,
}

/// A `StoredDocument` object is a representation of a `Document` object that has been stored in a database.
#[derive(Clone, Debug, schemars::JsonSchema, serde::Serialize)]
pub struct StoredDocument {
    pub id: String,
    #[serde(flatten)]
    pub document: Document,
}

impl ToString for Document {
    fn to_string(&self) -> String {
        format!(
            "Document {{ {} }}",
            serde_json::to_string_pretty(&self).expect("cannot convert Document to string"),
        )
    }
}

impl ToString for StoredDocument {
    fn to_string(&self) -> String {
        format!(
            "StoredDocument {{ {} }}",
            serde_json::to_string_pretty(&self).expect("cannot convert Document to string"),
        )
    }
}
