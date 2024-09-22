use std::pin::Pin;

use anyhow::Result;
use async_trait::async_trait;
use futures::Stream;

use crate::message::{Content, Message};

// pub struct StreamEvent {
//     pub event: String,
//     pub data: StreamData,
// }

// pub struct StreamData {
//     pub value: serde_json::Value,
//     pub content: Content,
// }

#[derive(Debug)]
pub enum StreamEvent {
    Start {
        index: u64,
        model: String,
        role: String,
        content: Content,
    },
    Delta {
        index: u64,
        content: Content,
    },
    End {
        stop_reason: String,
    },
}

// #[derive(Default)]
// pub struct CompletionRequest {
//     pub model: String,
//     pub messages: Vec<Message>,
//     pub system: Option<Vec<Content>>,
//     pub temperature: Option<f32>,
// }

#[async_trait]
pub trait Completion: Send + Sync {
    async fn complete(
        &self,
        messages: Vec<Message>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>>>;
}
