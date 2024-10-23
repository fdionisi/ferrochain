use std::sync::Arc;

pub use anthropic::Model;
use anthropic::{
    messages::{ContentPart, CreateMessageRequest, Event, MessagesStream, ToolInputSchema},
    Anthropic, AnthropicBuilder,
};
use ferrochain::{
    anyhow::{anyhow, Result},
    completion::{Completion, CompletionResponse, StreamEvent, StreamEventEnvelope},
    futures::{lock::Mutex, StreamExt},
    message::{Content, ImageSource, Message, ToolUse},
    tool::{ToolDescriptor, ToolProvider},
};
use http_client::HttpClient;

pub struct AnthropicCompletion {
    sdk: Anthropic,
    model: Model,
    system: Option<Vec<Content>>,
    temperature: Option<f32>,
    max_tokens: usize,
    tool_provider: Option<ToolProvider>,
}

#[derive(Clone)]
pub struct AnthropicCompletionBuilder {
    builder: AnthropicBuilder,
    model: Option<Model>,
    system: Option<Vec<Content>>,
    temperature: Option<f32>,
    max_tokens: Option<usize>,
    tool_provider: Option<ToolProvider>,
}

impl AnthropicCompletion {
    pub fn builder() -> AnthropicCompletionBuilder {
        AnthropicCompletionBuilder {
            builder: Anthropic::builder(),
            model: None,
            system: None,
            temperature: None,
            max_tokens: None,
            tool_provider: None,
        }
    }
}

impl AnthropicCompletionBuilder {
    pub fn with_http_client(mut self, client: Arc<dyn HttpClient>) -> Self {
        self.builder.with_http_client(client);
        self
    }

    pub fn with_api_key<S>(mut self, api_key: S) -> Self
    where
        S: AsRef<str>,
    {
        self.builder.with_api_key(api_key);
        self
    }

    pub fn with_base_url<S>(mut self, base_url: S) -> Self
    where
        S: AsRef<str>,
    {
        self.builder.with_base_url(base_url);
        self
    }

    pub fn with_model(mut self, model: Model) -> Self {
        self.model = Some(model);
        self
    }

    pub fn with_system(mut self, system: Vec<Content>) -> Self {
        self.system = Some(system);
        self
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    pub fn with_tool_provider(mut self, tool_provider: ToolProvider) -> Self {
        self.tool_provider = Some(tool_provider);
        self
    }

    pub fn build(self) -> Result<AnthropicCompletion> {
        Ok(AnthropicCompletion {
            sdk: self.builder.build()?,
            model: self.model.ok_or_else(|| anyhow!("model is required"))?,
            system: self.system,
            temperature: self.temperature,
            max_tokens: self
                .max_tokens
                .ok_or_else(|| anyhow!("max_tokens is required"))?,
            tool_provider: self.tool_provider,
        })
    }
}

#[ferrochain::async_trait]
impl Completion for AnthropicCompletion {
    async fn complete(&self, messages: Vec<Message>) -> Result<CompletionResponse> {
        let messages: Vec<anthropic::messages::Message> = messages
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

        let mut s = self
            .sdk
            .messages_stream(CreateMessageRequest {
                model: self.model.to_string(),
                messages,
                max_tokens: self.max_tokens as u32,
                metadata: Default::default(),
                stop_sequences: None,
                system: self.system.to_owned().map(|parts| {
                    anthropic::messages::Content::Multi(
                        parts
                            .into_iter()
                            .map(ferrochain_content_to_anthropic)
                            .collect(),
                    )
                }),
                temperature: self.temperature,
                tools: self.tool_provider.clone().map(|tool_provider| {
                    tool_provider
                        .list()
                        .map(ferrochain_tool_descriptor_to_anthropic)
                        .collect()
                }),
                tool_choice: None,
                top_k: None,
                top_p: None,
            })
            .await?;

        Ok(async_stream::stream! {
            let tool_use = Arc::new(Mutex::new((None, None, String::new())));

            while let Some(item) = s.next().await {
                match item {
                    Ok(event) => match event {
                        Event::Ping => continue,
                        Event::MessageStart { message } => {
                            let content = message
                                .message_response
                                .content
                                .into_iter()
                                .map(|c| anthropic_content_to_ferrochain(&c))
                                .collect::<Vec<Content>>();

                            yield Ok(StreamEventEnvelope{ index: 0, event: StreamEvent::Start {
                                index: 0,
                                model: message.message_response.model,
                                role: message.message_response.role,
                                inner: content,
                            }})
                        }
                        Event::ContentBlockStart {
                            index,
                            content_block: delta,
                        } | Event::ContentBlockDelta { index, delta } => match delta {
                            ContentPart::ToolUse { id, name, .. } => {
                                tool_use.lock().await.0.replace(id);
                                tool_use.lock().await.1.replace(name);
                                continue
                            }
                            ContentPart::InputJsonDelta { partial_json } => {
                                let mut guard = tool_use.lock().await;
                                guard.2.push_str(&partial_json.to_string());
                                match serde_json::from_str(&guard.2) {
                                    Ok(value) => {
                                        yield Ok(StreamEventEnvelope { index: 0, event: StreamEvent::Delta {
                                            index,
                                            inner: vec![Content::ToolUse(ToolUse {
                                                id: guard.0.take().unwrap(),
                                                tool: guard.1.take().unwrap(),
                                                input: value,
                                            })],
                                        }});

                                        guard.2.clear();
                                    }
                                    _ => {
                                        continue;
                                    }
                                }
                            },
                            _ => yield Ok(StreamEventEnvelope { index: 0, event: StreamEvent::Delta {
                                index,
                                inner: vec![anthropic_content_to_ferrochain(&delta)],
                            }}),
                        },
                        // Event::ContentBlockDelta { index, delta } => match delta {
                        //     ContentPart::ToolUse { id, name, input } => {
                        //         tool_use.lock().await.0.replace(id);
                        //         tool_use.lock().await.1.replace(name);
                        //         tool_use.lock().await.2.push_str(&serde_json::to_string(&input)?);
                        //         continue
                        //     }
                        //     ContentPart::InputJsonDelta { partial_json } => {
                        //         let mut guard = tool_use.lock().await;
                        //         guard.2.push_str(&serde_json::to_string(&partial_json)?);
                        //         match serde_json::from_str(&guard.2) {
                        //             Ok(value) => {
                        //                 yield Ok(StreamEvent::Delta {
                        //                     index,
                        //                     inner: vec![Content::ToolUse {
                        //                         id: guard.0.take().unwrap(),
                        //                         tool: guard.1.take().unwrap(),
                        //                         input: value,
                        //                     }],
                        //                 });

                        //                 guard.2.clear();
                        //             }
                        //             _ => continue,
                        //         }
                        //     },
                        //     _ => yield Ok(StreamEvent::Delta {
                        //         index,
                        //         inner: vec![anthropic_content_to_ferrochain(&delta)],
                        //     }),
                        // },
                        Event::ContentBlockStop { .. } => continue,
                        Event::MessageDelta { delta, .. } => yield Ok(StreamEventEnvelope { index: 0, event: StreamEvent::End {
                            stop_reason: format!("{:?}", delta.stop_reason),
                        }}),
                        Event::MessageStop => continue,
                        Event::Error(err) => yield Err(anyhow!("{:?}", err)),
                    },
                    Err(err) => yield Err(err),
                }
            }
        }
        .boxed()
        .into())
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
        ContentPart::ToolUse { .. } | ContentPart::InputJsonDelta { .. } => unreachable!(),
        ContentPart::ToolResult { .. } => todo!(),
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
        ferrochain::message::Content::ToolUse(ferrochain::message::ToolUse { id, tool, input }) => {
            anthropic::messages::ContentPart::ToolUse {
                id,
                name: tool,
                input,
            }
        }
        ferrochain::message::Content::ToolResult(ferrochain::message::ToolResult {
            id,
            content,
        }) => anthropic::messages::ContentPart::ToolResult {
            tool_use_id: id,
            content,
        },
    }
}

fn ferrochain_tool_descriptor_to_anthropic(tool: ToolDescriptor) -> anthropic::messages::Tool {
    let input_schema_value = serde_json::to_value(&tool.input).unwrap();

    anthropic::messages::Tool {
        name: tool.name.clone(),
        description: Some(tool.description.clone()),
        input_schema: ToolInputSchema {
            kind: input_schema_value["type"].as_str().unwrap().to_string(),
            properties: input_schema_value["properties"].clone(),
            required: input_schema_value["required"]
                .as_array()
                .as_ref()
                .unwrap()
                .into_iter()
                .map(|e| serde_json::from_value(e.to_owned()).unwrap())
                .collect::<Vec<String>>(),
        },
    }
}
