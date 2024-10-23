use serde_json::Value;

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct Message {
    pub role: String,
    pub content: Vec<Content>,
    pub metadata: Option<Value>,
    pub name: Option<String>,
    pub id: Option<String>,
    pub created_at: Option<u64>,
}

impl Message {
    pub fn tool_use(&self) -> impl Iterator<Item = &ToolUse> + '_ {
        self.content.iter().filter_map(|content| match content {
            Content::ToolUse(tool_use) => Some(tool_use),
            _ => None,
        })
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ToolUse {
    pub id: String,
    pub tool: String,
    pub input: Value,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ToolResult {
    pub id: String,
    pub content: String,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Content {
    Text { text: String },
    Image { source: ImageSource },
    ToolUse(ToolUse),
    ToolResult(ToolResult),
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageSource {
    Base64 { data: String },
    Url { url: String },
}

impl<S> From<S> for Content
where
    S: AsRef<str>,
{
    fn from(value: S) -> Content {
        Content::Text {
            text: value.as_ref().into(),
        }
    }
}
