use std::pin::Pin;

pub use anthropic::Model;
use anthropic::{
    messages::{ContentPart, CreateMessageRequest, Event, MessagesStream},
    Anthropic, AnthropicBuilder,
};
use ferrochain::{
    anyhow::{anyhow, Result},
    completion::{Completion, CompletionRequest, StreamEvent},
    futures::{Stream, StreamExt},
    message::{Content, ImageSource},
};

pub struct AnthropicCompletion {
    sdk: Anthropic,
}

pub struct AnthropicCompletionBuilder {
    builder: AnthropicBuilder,
}

impl AnthropicCompletion {
    pub fn builder() -> AnthropicCompletionBuilder {
        AnthropicCompletionBuilder {
            builder: Anthropic::builder(),
        }
    }
}

impl AnthropicCompletionBuilder {
    pub fn api_key<S>(&mut self, api_key: S) -> &mut Self
    where
        S: AsRef<str>,
    {
        self.builder.api_key(api_key);
        self
    }

    pub fn base_url<S>(&mut self, base_url: S) -> &mut Self
    where
        S: AsRef<str>,
    {
        self.builder.base_url(base_url);
        self
    }

    pub fn build(&self) -> Result<AnthropicCompletion> {
        Ok(AnthropicCompletion {
            sdk: self.builder.build()?,
        })
    }
}

#[ferrochain::async_trait]
impl Completion for AnthropicCompletion {
    async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>>> {
        let messages: Vec<anthropic::messages::Message> = request
            .messages
            .into_iter()
            .map(|m| anthropic::messages::Message {
                role: match m.role.as_str() {
                    "user" => anthropic::messages::Role::User,
                    "assistant" => anthropic::messages::Role::Assistant,
                    _ => anthropic::messages::Role::User, // Default to User for unknown roles
                },
                content: anthropic::messages::Content::Multi(
                    m.content
                        .into_iter()
                        .map(ferrochain_content_to_anthropic)
                        .collect(),
                ),
            })
            .collect();

        Ok(self
            .sdk
            .messages_stream(CreateMessageRequest {
                model: request.model,
                messages,
                max_tokens: 8192,
                metadata: Default::default(),
                stop_sequences: None,
                system: request.system.map(|parts| {
                    anthropic::messages::Content::Multi(
                        parts
                            .into_iter()
                            .map(ferrochain_content_to_anthropic)
                            .collect(),
                    )
                }),
                temperature: request.temperature,
                tool_choice: None,
                tools: None,
                top_k: None,
                top_p: None,
            })
            .await?
            .filter_map(|item| async {
                match item {
                    Ok(event) => match event {
                        Event::Ping => None,
                        Event::MessageStart { message } => {
                            let content = message.message_response.content.first()?;

                            Some(Ok(StreamEvent::Start {
                                index: 0,
                                model: message.message_response.model,
                                role: message.message_response.role,
                                content: anthropic_content_to_ferrochain(content),
                            }))
                        }
                        Event::ContentBlockStart {
                            index,
                            content_block,
                        } => Some(Ok(StreamEvent::Delta {
                            index,
                            content: anthropic_content_to_ferrochain(&content_block),
                        })),
                        Event::ContentBlockDelta { index, delta } => Some(Ok(StreamEvent::Delta {
                            index,
                            content: anthropic_content_to_ferrochain(&delta),
                        })),
                        Event::ContentBlockStop { .. } => None,
                        Event::MessageDelta { delta, .. } => Some(Ok(StreamEvent::End {
                            stop_reason: format!("{:?}", delta.stop_reason),
                        })),
                        Event::MessageStop => None,
                        Event::Error(err) => Some(Err(anyhow!("{:?}", err))),
                    },
                    Err(err) => Some(Err(err)),
                }
            })
            .boxed())
    }
}

fn anthropic_content_to_ferrochain(content: &ContentPart) -> Content {
    match content {
        ContentPart::Text { text } | ContentPart::TextDelta { text } => Content::Text {
            text: text.to_owned(),
        },
        ContentPart::Image { source } => Content::Image {
            source: ImageSource::Base64 {
                data: source.data.to_owned(),
            },
        },
        ContentPart::ToolResult { .. } => todo!(),
        ContentPart::ToolUse { .. } => todo!(),
        ContentPart::InputJsonDelta { .. } => todo!(),
    }
}

fn ferrochain_content_to_anthropic(content: Content) -> ContentPart {
    match content {
        ferrochain::message::Content::Text { text } => {
            anthropic::messages::ContentPart::Text { text }
        }
        ferrochain::message::Content::Image { source } => {
            anthropic::messages::ContentPart::Image {
                source: anthropic::messages::ImageSource {
                    kind: "base64".to_string(),
                    media_type: anthropic::messages::MediaType::ImagePng, // Assuming PNG for simplicity
                    data: match source {
                        ferrochain::message::ImageSource::Base64 { data } => data,
                        ferrochain::message::ImageSource::Url { url } => url, // This is not correct, but we don't have a way to fetch the image data here
                    },
                },
            }
        }
    }
}