use std::{collections::HashMap, hash::Hash, sync::Arc};

use anyhow::{bail, Result};
use async_trait::async_trait;
use schemars::{schema::RootSchema, schema_for, JsonSchema};
use serde_json::Value;

use crate::message::{ToolResult, ToolUse};

#[derive(Debug)]
pub struct ToolDescriptor {
    pub name: String,
    pub description: String,
    pub input: RootSchema,
    pub output: RootSchema,
    pub external: bool,
}

#[async_trait]
pub trait Tool {
    type Input: JsonSchema;
    type Output: JsonSchema;

    fn name(&self) -> String;

    fn description(&self) -> String;

    fn schema(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: self.name().to_string(),
            description: self.description().to_string(),
            input: schema_for!(Self::Input),
            output: schema_for!(Self::Output),
            external: false,
        }
    }

    async fn execute(&self, input: Value) -> Result<String>;
}

#[async_trait]
trait AnyTool: Send + Sync {
    fn schema(&self) -> ToolDescriptor;
    async fn execute(&self, input: Value) -> Result<String>;
}

#[async_trait]
impl<T: Tool + Send + Sync + 'static> AnyTool for T {
    fn schema(&self) -> ToolDescriptor {
        Tool::schema(self)
    }

    async fn execute(&self, input: Value) -> Result<String> {
        Tool::execute(self, input).await
    }
}

impl<I, O> Hash for dyn Tool<Input = I, Output = O>
where
    I: JsonSchema,
    O: JsonSchema,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write(self.name().as_bytes())
    }
}

#[derive(Clone)]
pub struct ToolProvider(HashMap<String, Arc<dyn AnyTool>>);

impl ToolProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<T>(&mut self, tool: T)
    where
        T: Tool + Send + Sync + 'static,
    {
        self.0
            .insert(tool.name().to_string(), Arc::new(tool) as Arc<dyn AnyTool>);
    }

    pub async fn execute(&self, tool_use: &ToolUse) -> Result<ToolResult> {
        let Some(tool) = self.0.get(&tool_use.tool) else {
            bail!("Tool not found: {}", tool_use.tool)
        };

        let result = tool.execute(tool_use.input.clone()).await?;

        Ok(ToolResult {
            id: tool_use.id.clone(),
            content: result,
        })
    }

    pub fn list(&self) -> impl Iterator<Item = ToolDescriptor> + '_ {
        self.0.values().map(|tool| tool.schema())
    }
}

impl Default for ToolProvider {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    struct EchoTool;

    #[async_trait]
    impl Tool for EchoTool {
        type Input = ();
        type Output = ();

        fn name(&self) -> String {
            "echo".into()
        }

        fn description(&self) -> String {
            "Echoes the input".into()
        }

        async fn execute(&self, input: Value) -> Result<String> {
            Ok(input.to_string())
        }
    }

    #[tokio::test]
    async fn test_echo_tool() {
        let mut provider = ToolProvider::new();

        provider.register(EchoTool);

        let input = json!({"message": "Hello, world!"});
        let output = provider
            .execute(&ToolUse {
                id: "1".into(),
                tool: "echo".into(),
                input: input.clone(),
            })
            .await
            .unwrap();

        assert_eq!(input, output.content);
    }
}
