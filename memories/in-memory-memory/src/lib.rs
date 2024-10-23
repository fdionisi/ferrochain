use std::sync::Arc;

use anyhow::Result;
use ferrochain::{futures::lock::Mutex, memory::Memory, message::Message};

#[derive(Default)]
pub struct InMemoryMemory {
    inner: Arc<Mutex<Vec<Message>>>,
}

impl InMemoryMemory {
    pub fn new() -> Self {
        Self::default()
    }
}

#[ferrochain::async_trait]
impl Memory for InMemoryMemory {
    async fn add_messages(&self, messages: Vec<Message>) -> Result<()> {
        let mut inner = self.inner.lock().await;
        inner.extend(messages);
        Ok(())
    }

    async fn messages(&self) -> Result<Vec<Message>> {
        let inner = self.inner.lock().await;
        Ok(inner.clone())
    }

    async fn clear(&self) -> Result<()> {
        let mut inner = self.inner.lock().await;
        inner.clear();
        Ok(())
    }
}
