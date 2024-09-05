use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use ferrochain::{
    anyhow::{anyhow, Result},
    document::Document,
    document_loader::DocumentLoader,
};
use tree_sitter::{Node, Parser};
use tree_sitter_md::language;

pub struct MarkdownLoader {
    path: PathBuf,
}

impl From<PathBuf> for MarkdownLoader {
    fn from(path: PathBuf) -> Self {
        Self { path }
    }
}

impl From<&Path> for MarkdownLoader {
    fn from(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
        }
    }
}

#[ferrochain::async_trait]
impl DocumentLoader for MarkdownLoader {
    async fn load(&self) -> Result<Vec<Document>> {
        let file = tokio::fs::read_to_string(&self.path).await?;

        Ok(parse_markdown(&file)?)
    }
}

fn parse_markdown(content: &str) -> Result<Vec<Document>> {
    let mut parser = Parser::new();
    parser
        .set_language(&language())
        .map_err(|err| anyhow!("Error loading Markdown grammar: {}", err))?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| anyhow!("Couldn't parse input content"))?;
    let root_node = tree.root_node();

    let mut documents = Vec::new();

    let metadata = {
        let mut cursor = root_node.walk();
        cursor.goto_first_child();
        let node = cursor.node();
        if "minus_metadata" == node.kind() {
            let rng = node.byte_range();
            let fms = &content[rng];
            let fm = fms
                .lines()
                .skip(1)
                .take_while(|l| !l.eq(&"---"))
                .collect::<Vec<_>>()
                .join("\n");
            serde_yml::from_str::<HashMap<String, serde_json::Value>>(&fm)?
        } else {
            HashMap::new()
        }
    };

    extract_sections(
        &root_node,
        content,
        Vec::new(),
        String::new(),
        &metadata,
        &mut documents,
    );

    Ok(documents)
}

fn extract_sections(
    node: &Node,
    content: &str,
    mut headings: Vec<String>,
    mut current_section: String,
    metadata: &HashMap<String, serde_json::Value>,
    documents: &mut Vec<Document>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "atx_heading" => {
                if !current_section.is_empty() {
                    documents.push(Document {
                        content: construct_section_content(&headings, &current_section),
                        metadata: metadata.clone(),
                    });
                    current_section.clear();
                }

                let heading_content = &content[child.start_byte()..child.end_byte()];
                headings.push(heading_content.trim().to_string());
            }
            "paragraph" | "code_block" | "fenced_code_block" | "block_quote" | "list" => {
                let node_content = &content[child.start_byte()..child.end_byte()];
                if !current_section.is_empty() {
                    current_section.push_str("\n\n");
                }
                current_section.push_str(node_content);
            }
            "section" => {
                extract_sections(
                    &child,
                    content,
                    headings.clone(),
                    current_section,
                    metadata,
                    documents,
                );
                current_section = String::new();
            }
            _ => {}
        }
    }

    if !current_section.is_empty() {
        documents.push(Document {
            content: construct_section_content(&headings, &current_section),
            metadata: metadata.clone(),
        });
    }
}

fn construct_section_content(headings: &[String], content: &str) -> String {
    let mut result = String::new();
    for heading in headings {
        result.push_str(heading);
        result.push_str("\n\n");
    }
    result.push_str(content);
    result
}
