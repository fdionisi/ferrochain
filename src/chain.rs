use std::ops::BitOr;

use anyhow::Result;
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

#[async_trait]
pub trait Chain: Send + Sync {
    async fn run(&self, input: Value) -> Result<Value>;
}

#[async_trait]
pub trait StructuredChain: Chain {
    type Input: Serialize + Send;
    type Output: DeserializeOwned;

    async fn run(&self, input: Self::Input) -> Result<Self::Output> {
        let output = Chain::run(self, serde_json::to_value(&input)?).await?;
        Ok(serde_json::from_value(output)?)
    }
}

impl BitOr for Box<dyn Chain> {
    type Output = Box<dyn Chain>;

    fn bitor(self, rhs: Box<dyn Chain>) -> Self::Output {
        Box::new(Chained {
            current: self,
            next: rhs,
        })
    }
}

struct Chained {
    current: Box<dyn Chain>,
    next: Box<dyn Chain>,
}

#[async_trait]
impl Chain for Chained {
    async fn run(&self, input: Value) -> Result<Value> {
        let output = self.current.run(input).await?;
        self.next.run(output).await
    }
}
