use std::sync::Arc;

use surrealdb::{engine::any::Any, Surreal};

use ferrochain::{anyhow::Result, memory::Memory, message::Message};

pub struct SurrealDbMemory {
    db: Arc<Surreal<Any>>,
    table: String,
}

impl SurrealDbMemory {
    pub fn builder() -> SurrealDbMemoryBuilder {
        SurrealDbMemoryBuilder {
            db: None,
            table: None,
        }
    }
}

pub struct SurrealDbMemoryBuilder {
    db: Option<Arc<Surreal<Any>>>,
    table: Option<String>,
}

impl SurrealDbMemoryBuilder {
    pub fn with_surrealdb(mut self, db: Arc<Surreal<Any>>) -> Self {
        self.db = Some(db);
        self
    }

    pub fn with_table(mut self, table: String) -> Self {
        self.table = Some(table);
        self
    }

    pub fn build(self) -> Result<SurrealDbMemory> {
        let db = self
            .db
            .ok_or_else(|| ferrochain::anyhow::anyhow!("DB not set"))?;
        let table = self
            .table
            .ok_or_else(|| ferrochain::anyhow::anyhow!("Table not set"))?;

        Ok(SurrealDbMemory { db, table })
    }
}

#[ferrochain::async_trait]
impl Memory for SurrealDbMemory {
    async fn add_messages(&self, messages: Vec<Message>) -> Result<()> {
        let sql = format!("CREATE {} CONTENT $content", self.table);
        for message in messages {
            self.db
                .query(sql.clone())
                .bind(("content", message))
                .await?;
        }
        Ok(())
    }

    async fn messages(&self) -> Result<Vec<Message>> {
        let sql = format!("SELECT * FROM {}", self.table);
        let mut result = self.db.query(sql).await?;
        let messages: Vec<Message> = result.take(0)?;
        Ok(messages)
    }

    async fn clear(&self) -> Result<()> {
        let sql = format!("DELETE {}", self.table);
        self.db.query(sql).await?;
        Ok(())
    }
}
