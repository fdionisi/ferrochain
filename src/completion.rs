use std::pin::Pin;

use anyhow::Result;
use async_trait::async_trait;
use futures::Stream;

use crate::message::{Content, Message};

pub struct StreamEvent {
    pub event: String,
    pub data: StreamData,
}

pub struct StreamData {
    pub value: serde_json::Value,
    pub content: Content,
}

#[async_trait]
pub trait Completion: Send + Sync {
    async fn complete(
        &self,
        messages: Vec<Message>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>>>;
}
