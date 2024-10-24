use ferrochain::{anyhow::Result, document::Document, splitter::Splitter};
use text_splitter::ChunkConfig;

pub struct MarkdownSplitter {
    capacity: usize,
    overlap: usize,
}

impl MarkdownSplitter {
    pub fn builder() -> MarkdownSplitterBuilder {
        MarkdownSplitterBuilder {
            capacity: None,
            overlap: None,
        }
    }
}

pub struct MarkdownSplitterBuilder {
    capacity: Option<usize>,
    overlap: Option<usize>,
}

impl MarkdownSplitterBuilder {
    pub fn with_capacity(mut self, capacity: usize) -> Self {
        self.capacity = Some(capacity);
        self
    }

    pub fn with_overlap(mut self, overlap: usize) -> Self {
        self.overlap = Some(overlap);
        self
    }

    pub fn build(self) -> MarkdownSplitter {
        MarkdownSplitter {
            capacity: self.capacity.unwrap(),
            overlap: self.overlap.unwrap(),
        }
    }
}

#[ferrochain::async_trait]
impl Splitter for MarkdownSplitter {
    async fn split(&self, docs: Vec<Document>) -> Result<Vec<Document>> {
        let splitter = text_splitter::MarkdownSplitter::new(
            ChunkConfig::new(self.capacity).with_overlap(self.overlap)?,
        );

        Ok(docs
            .iter()
            .flat_map(|d| {
                splitter.chunks(&d.content).map(|chunk| Document {
                    content: chunk.to_string(),
                    metadata: d.metadata.clone(),
                })
            })
            .collect())
    }
}
