use std::{
    pin::Pin,
    task::{Context, Poll},
};

use anyhow::Result;
use async_trait::async_trait;
use futures::{Stream, TryStreamExt};

use crate::message::{Content, Message};

pub trait CompletionModel {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn max_tokens(&self) -> usize;
    fn tool_use(&self) -> bool;
}

pub struct CompletionModelData {
    pub id: String,
    pub name: String,
    pub max_tokens: usize,
}

impl<C> From<C> for CompletionModelData
where
    C: CompletionModel,
{
    fn from(model: C) -> Self {
        Self {
            id: model.id().to_string(),
            name: model.name().to_string(),
            max_tokens: model.max_tokens(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct StreamEventEnvelope<Inner> {
    pub index: u64,
    pub event: StreamEvent<Inner>,
}

#[derive(Debug, Clone)]
pub enum StreamEvent<Inner> {
    Start {
        index: u64,
        model: String,
        role: String,
        inner: Inner,
    },
    Delta {
        index: u64,
        inner: Inner,
    },
    End {
        stop_reason: String,
    },
}

pub struct CompletionResponse(
    Pin<Box<dyn Stream<Item = Result<StreamEventEnvelope<Vec<Content>>>> + Send>>,
);

impl CompletionResponse {
    pub fn new(
        stream: Pin<Box<dyn Stream<Item = Result<StreamEventEnvelope<Vec<Content>>>> + Send>>,
    ) -> Self {
        Self(stream)
    }
}

impl From<Pin<Box<dyn Stream<Item = Result<StreamEventEnvelope<Vec<Content>>>> + Send>>>
    for CompletionResponse
{
    fn from(
        value: Pin<Box<dyn Stream<Item = Result<StreamEventEnvelope<Vec<Content>>>> + Send>>,
    ) -> Self {
        Self::new(value)
    }
}

impl Stream for CompletionResponse {
    type Item = Result<StreamEventEnvelope<Vec<Content>>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.0.as_mut().poll_next(cx)
    }
}

fn mut_get_or_insert<T>(vec: &mut Vec<T>, index: usize, val: T) -> &mut T {
    if index < vec.len() {
        vec.get_mut(index).unwrap()
    } else {
        vec.push(val);
        vec.last_mut().unwrap()
    }
}

impl Extend<StreamEventEnvelope<Vec<Content>>> for Vec<Message> {
    fn extend<T: IntoIterator<Item = StreamEventEnvelope<Vec<Content>>>>(&mut self, iter: T) {
        for event in iter.into_iter() {
            let index = event.index;
            let event = event.event;
            let message = mut_get_or_insert(self, index as usize, Message::default());
            match event {
                StreamEvent::Start { role, .. } => {
                    message.role = role;
                }
                StreamEvent::Delta { inner, .. } => {
                    for content in inner {
                        if let Some(Content::Text { text: last_text }) = message.content.last_mut()
                        {
                            if let Content::Text { text: new_text } = content {
                                last_text.push_str(&new_text);
                            } else {
                                message.content.push(content);
                            }
                        } else {
                            message.content.push(content);
                        }
                    }
                }
                _ => break,
            }
        }
    }
}

#[async_trait]
pub trait Completion: Send + Sync {
    async fn complete(&self, messages: Vec<Message>) -> Result<CompletionResponse>;
    async fn i(&self, messages: Vec<Message>) -> Result<Vec<Message>> {
        Ok(self.complete(messages).await?.try_collect().await?)
    }
}

#[async_trait]
pub trait StructuredCompletion {
    type Output;

    async fn structured_complete(
        &self,
        messages: Vec<Message>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent<Self::Output>>>>>>;
}
