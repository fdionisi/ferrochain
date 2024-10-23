use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use convert_case::Casing;
use indoc::formatdoc;

use crate::{document::Document, tool::Tool};

#[async_trait]
pub trait Retriever: Send + Sync {
    async fn retrieve(&self, query: &str) -> Result<Vec<Document>>;
}

pub struct RetrieverTool {
    retriever: Arc<dyn Retriever>,
    name: String,
    description: String,
}

pub struct RetrieverToolBuilder {
    retriever: Option<Arc<dyn Retriever>>,
    name: Option<String>,
    description: Option<String>,
}

impl RetrieverTool {
    pub fn builder() -> RetrieverToolBuilder {
        RetrieverToolBuilder {
            retriever: None,
            name: None,
            description: None,
        }
    }
}

impl RetrieverToolBuilder {
    pub fn with_retriever(mut self, retriever: Arc<dyn Retriever>) -> Self {
        self.retriever = Some(retriever);
        self
    }

    pub fn with_name<S>(mut self, name: S) -> Self
    where
        S: Into<String>,
    {
        self.name = Some(name.into());
        self
    }

    pub fn with_description<S>(mut self, description: S) -> Self
    where
        S: Into<String>,
    {
        self.description = Some(description.into());
        self
    }

    pub fn build(self) -> RetrieverTool {
        RetrieverTool {
            retriever: self.retriever.unwrap(),
            name: self.name.unwrap(),
            description: self.description.unwrap(),
        }
    }
}

#[derive(schemars::JsonSchema, serde::Deserialize)]
pub struct RetrieverToolInput {
    /// The input retriever question.
    query: String,
}

#[async_trait]
impl Tool for RetrieverTool {
    type Input = RetrieverToolInput;
    type Output = Vec<Document>;

    fn name(&self) -> String {
        format!("retriever_{}", self.name.to_case(convert_case::Case::Snake))
    }

    fn description(&self) -> String {
        formatdoc! {"
            Useful for when you need to answer questions about {} and the sources used to construct the answer.
            Whenever you need information about {} you should ALWAYS use this.
            Input MUST be a fully formed question. You must always provide the input.

            Think before defining the input. You should think in between the <thinking> tags.

            Output is a json serialized dictionary list as follows:
            [{{ \"content\": \"...\", \"metadata\": {{ ... }}, \"score\": ... }}, ...]
        ", self.name, self.description}.into()
    }

    async fn execute(&self, input: serde_json::Value) -> Result<String> {
        let input: Self::Input = serde_json::from_value(input)?;
        let response = self.retriever.retrieve(&input.query).await?;
        Ok(serde_json::to_string(&response)?)
    }
}
