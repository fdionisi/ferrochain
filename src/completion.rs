use std::pin::Pin;

use anyhow::Result;
use futures::Stream;

use crate::message::{Content, Message};

pub struct CompletionOptions {
    pub system: Option<String>,
    pub model: String,
    pub max_tokens: usize,
    pub temperature: Option<f32>,
}

#[derive(Debug)]
pub struct CompletionResponse {
    pub id: String,
    pub model: String,
    pub role: String,
    pub content: Vec<Content>,
    pub stop_reason: String,
    pub stop_sequence: Option<String>,
}

pub trait Completion: Send + Sync {
    fn complete(&self, messages: Vec<Message>)
        -> Pin<Box<dyn Stream<Item = Result<Message>> + '_>>;
}
