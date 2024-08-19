use anyhow::Result;

use crate::{async_trait, document::Document};

/// Asynchronous trait for loading lists of `Document`.
///
/// Ferrochain can itegrate with various data sources to load data from: FS, Slack, Notion, Google Drive, etc.
/// Each `DocumentLoader` has its own specific parameters, but they can all be invoked in the same way with the `.load` method.
#[async_trait]
pub trait DocumentLoader: Send + Sync {
    /// Load a list of `Document` objects.
    async fn load(&self) -> Result<Vec<Document>>;
}
