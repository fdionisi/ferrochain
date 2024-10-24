use ferrochain::{anyhow::Result, document::Document, splitter::Splitter};
use tree_sitter::{Node, Parser};
use tree_sitter_language::LanguageFn;

pub struct CodeSplitter {
    language: String,
    max_chunk_size: usize,
}

pub struct CodeSplitterBuilder {
    language: Option<String>,
    max_chunk_size: Option<usize>,
}

impl CodeSplitterBuilder {
    pub fn new() -> Self {
        CodeSplitterBuilder {
            language: None,
            max_chunk_size: None,
        }
    }

    pub fn language(mut self, language: String) -> Self {
        self.language = Some(language);
        self
    }

    pub fn max_chunk_size(mut self, size: usize) -> Self {
        self.max_chunk_size = Some(size);
        self
    }

    pub fn build(self) -> Result<CodeSplitter> {
        Ok(CodeSplitter {
            language: self.language.unwrap_or_else(|| "rust".to_string()),
            max_chunk_size: self.max_chunk_size.unwrap_or(500),
        })
    }
}

impl CodeSplitter {
    fn create_document_with_context(
        &self,
        node: Node,
        source: &str,
        context: &str,
    ) -> Result<Document> {
        let mut content = String::with_capacity(context.len() + node.byte_range().len());
        content.push_str(context);
        content.push_str(node.utf8_text(source.as_bytes())?);
        Ok(Document {
            content,
            metadata: Default::default(),
        })
    }

    pub fn builder() -> CodeSplitterBuilder {
        CodeSplitterBuilder::new()
    }

    fn get_class_name(&self, class_node: Node, source: &str) -> Result<String> {
        class_node
            .named_children(&mut class_node.walk())
            .find(|child| child.kind() == "identifier")
            .map(|child| Ok(child.utf8_text(source.as_bytes())?.to_string()))
            .unwrap_or(Ok(String::new()))
    }

    fn create_method_chunk(&self, node: Node, source: &str, class_name: &str) -> Result<Document> {
        let mut content = String::with_capacity(class_name.len() + node.byte_range().len() + 16);
        content.push_str("class ");
        content.push_str(class_name);
        content.push_str(" {\n");
        content.push_str(node.utf8_text(source.as_bytes())?);
        content.push_str("\n}");
        Ok(Document {
            content,
            metadata: Default::default(),
        })
    }

    fn get_language(&self) -> Result<LanguageFn> {
        match self.language.as_str() {
            "rust" => Ok(tree_sitter_rust::LANGUAGE),
            "python" => Ok(tree_sitter_python::LANGUAGE),
            "javascript" => Ok(tree_sitter_javascript::LANGUAGE),
            _ => Err(ferrochain::anyhow::anyhow!(
                "Unsupported language: {}",
                self.language
            )),
        }
    }

    fn split_node(&self, node: Node, source: &str) -> Result<Vec<Document>> {
        let node_text = node.utf8_text(source.as_bytes())?;
        if node_text.len() <= self.max_chunk_size {
            return Ok(vec![Document {
                content: node_text.to_string(),
                metadata: Default::default(),
            }]);
        }

        let mut chunks = Vec::new();
        let mut current_chunk = String::with_capacity(self.max_chunk_size);

        for child in node.children(&mut node.walk()) {
            let child_text = child.utf8_text(source.as_bytes())?;
            if current_chunk.len() + child_text.len() > self.max_chunk_size
                && !current_chunk.is_empty()
            {
                chunks.push(Document {
                    content: current_chunk,
                    metadata: Default::default(),
                });
                current_chunk = String::with_capacity(self.max_chunk_size);
            }
            current_chunk.push_str(child_text);
        }

        if !current_chunk.is_empty() {
            chunks.push(Document {
                content: current_chunk,
                metadata: Default::default(),
            });
        }

        Ok(chunks)
    }

    fn split_rust(&self, node: Node, source: &str) -> Result<Vec<Document>> {
        let mut chunks = Vec::new();
        let mut context = String::new();
        let mut current_doc_comments = String::new();

        for child in node.named_children(&mut node.walk()) {
            match child.kind() {
                "struct_item" | "enum_item" | "trait_item" => {
                    let mut content = String::new();

                    for attr in child.children(&mut child.walk()) {
                        if attr.kind() == "attribute" {
                            content.push_str(attr.utf8_text(source.as_bytes())?);
                            content.push('\n');
                        }
                    }

                    if let Some(visibility) = child.child_by_field_name("visibility") {
                        content.push_str(visibility.utf8_text(source.as_bytes())?);
                        content.push(' ');
                    }

                    content.push_str(child.utf8_text(source.as_bytes())?);
                    chunks.push(Document {
                        content: format!("{}{}{}", current_doc_comments, context, content),
                        metadata: Default::default(),
                    });
                    current_doc_comments.clear();
                    context.clear();
                }
                "impl_item" => {
                    let mut impl_chunks = Vec::new();
                    let mut function_context = String::new();
                    let mut impl_type = String::new();
                    let mut type_parameters = String::new();

                    for impl_child in child.named_children(&mut child.walk()) {
                        if impl_child.kind() == "declaration_list" {
                            for decl_child in impl_child.named_children(&mut impl_child.walk()) {
                                if decl_child.kind() == "function_item" {
                                    let function_content =
                                        decl_child.utf8_text(source.as_bytes())?;

                                    let indented_function_context = function_context
                                        .lines()
                                        .map(|line| format!("    {}", line))
                                        .collect::<Vec<_>>()
                                        .join("\n");

                                    let impl_declaration = if type_parameters.is_empty() {
                                        format!("impl {}", impl_type)
                                    } else {
                                        format!("impl{} {}", type_parameters, impl_type)
                                    };

                                    impl_chunks.push(Document {
                                        content: format!(
                                            "{} {{\n{}{}\n}}",
                                            impl_declaration,
                                            indented_function_context,
                                            function_content
                                        ),
                                        metadata: Default::default(),
                                    });
                                    function_context.clear();
                                } else if decl_child.kind().contains("comment") {
                                    function_context
                                        .push_str(decl_child.utf8_text(source.as_bytes())?);
                                    function_context.push('\n');
                                }
                            }

                            impl_type.clear();
                            type_parameters.clear();
                        } else if impl_child.kind() == "type_parameters" {
                            type_parameters = impl_child.utf8_text(source.as_bytes())?.to_string();
                        } else {
                            let child_text = impl_child.utf8_text(source.as_bytes())?.to_string();
                            if !impl_type.is_empty() {
                                impl_type.push_str(" for ");
                            }
                            impl_type.push_str(&child_text);
                        }
                    }

                    chunks.extend(impl_chunks);
                    current_doc_comments.clear();
                    context.clear();
                }
                "function_item" | "const_item" | "static_item" | "type_item"
                | "macro_definition" => {
                    let mut content = String::new();

                    if let Some(visibility) = child.child_by_field_name("visibility") {
                        content.push_str(visibility.utf8_text(source.as_bytes())?);
                        content.push(' ');
                    }

                    content.push_str(child.utf8_text(source.as_bytes())?);
                    chunks.push(Document {
                        content: format!("{}{}{}", current_doc_comments, context, content),
                        metadata: Default::default(),
                    });
                    current_doc_comments.clear();
                    context.clear();
                }
                "attribute_item" | "use_declaration" | "mod_item" => {
                    context.push_str(child.utf8_text(source.as_bytes())?);
                    context.push('\n');
                }
                "line_comment" if child.utf8_text(source.as_bytes())?.starts_with("///") => {
                    current_doc_comments.push_str(child.utf8_text(source.as_bytes())?);
                }
                _ => {}
            }
        }

        Ok(chunks)
    }

    fn split_python(&self, node: Node, source: &str) -> Result<Vec<Document>> {
        let mut chunks = Vec::new();
        for child in node.named_children(&mut node.walk()) {
            if child.kind() == "function_definition" || child.kind() == "class_definition" {
                chunks.extend(self.split_node(child, source)?);
            }
        }
        Ok(chunks)
    }

    fn split_javascript(&self, node: Node, source: &str) -> Result<Vec<Document>> {
        let mut chunks = Vec::new();
        let mut context = String::new();

        for child in node.named_children(&mut node.walk()) {
            match child.kind() {
                "function_declaration" | "class_declaration" => {
                    chunks.push(self.create_document_with_context(child, source, &context)?);
                }
                "method_definition" => {
                    if let Some(parent) = child.parent() {
                        if parent.kind() == "class_body" {
                            let class_name =
                                self.get_class_name(parent.parent().unwrap(), source)?;
                            let method_chunk =
                                self.create_method_chunk(child, source, &class_name)?;
                            chunks.push(method_chunk);
                        } else {
                            chunks
                                .push(self.create_document_with_context(child, source, &context)?);
                        }
                    }
                }
                "import_statement" | "export_statement" => {
                    context.push_str(child.utf8_text(source.as_bytes())?);
                    context.push('\n');
                }
                "variable_declaration" if child.parent().unwrap().kind() == "program" => {
                    context.push_str(child.utf8_text(source.as_bytes())?);
                    context.push('\n');
                }
                _ => {}
            }
        }

        Ok(chunks)
    }
}

#[ferrochain::async_trait]
impl Splitter for CodeSplitter {
    async fn split(&self, docs: Vec<Document>) -> Result<Vec<Document>> {
        let mut parser = Parser::new();
        let language = self.get_language()?;

        parser.set_language(&language.into())?;

        let mut split_docs = Vec::new();
        for doc in docs {
            let tree = parser
                .parse(&doc.content, None)
                .ok_or_else(|| ferrochain::anyhow::anyhow!("Failed to parse document"))?;

            let root_node = tree.root_node();
            let chunks = match self.language.as_str() {
                "rust" => self.split_rust(root_node, &doc.content)?,
                "python" => self.split_python(root_node, &doc.content)?,
                "javascript" => self.split_javascript(root_node, &doc.content)?,
                _ => self.split_node(root_node, &doc.content)?,
            };

            split_docs.extend(chunks);
        }

        Ok(split_docs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rust_code_splitting() -> Result<()> {
        let splitter = CodeSplitter::builder()
            .language("rust".to_string())
            .build()?;

        let input = Document {
            content: r#"
/// A person with a name and age
#[derive(Debug, Clone)]
pub(crate) struct Person {
    pub name: String,
    age: u32,
}

impl Person {
    /// Creates a new Person
    pub fn new(name: String, age: u32) -> Self {
        Self { name, age }
    }

    /// Greets the person
    fn greet(&self) {
        println!("Hello, my name is {} and I'm {} years old.", self.name, self.age);
    }
}
            "#
            .trim()
            .to_string(),
            metadata: Default::default(),
        };

        let split_docs = splitter.split(vec![input]).await?;
        dbg!(&split_docs);

        assert_eq!(split_docs.len(), 3);
        assert_eq!(
            split_docs[0].content.lines().next().unwrap(),
            "/// A person with a name and age"
        );
        assert_eq!(
            split_docs[1].content.lines().next().unwrap(),
            "impl Person {"
        );
        assert_eq!(
            split_docs[2].content.lines().next().unwrap(),
            "impl Person {"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_rust_enum_splitting() -> Result<()> {
        let splitter = CodeSplitter::builder()
            .language("rust".to_string())
            .build()?;

        let input = Document {
            content: r#"
/// An enum representing different shapes
pub enum Shape {
    Circle(f64),
    Rectangle(f64, f64),
    Triangle(f64, f64, f64),
}

impl Shape {
    /// Calculates the area of the shape
    pub fn area(&self) -> f64 {
        match self {
            Shape::Circle(r) => std::f64::consts::PI * r * r,
            Shape::Rectangle(w, h) => w * h,
            Shape::Triangle(a, b, c) => {
                let s = (a + b + c) / 2.0;
                (s * (s - a) * (s - b) * (s - c)).sqrt()
            }
        }
    }
}
            "#
            .trim()
            .to_string(),
            metadata: Default::default(),
        };

        let split_docs = splitter.split(vec![input]).await?;
        dbg!(&split_docs);

        assert_eq!(split_docs.len(), 2);
        assert_eq!(
            split_docs[0].content.lines().next().unwrap(),
            "/// An enum representing different shapes"
        );
        assert_eq!(
            split_docs[1].content.lines().next().unwrap(),
            "impl Shape {"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_rust_trait_splitting() -> Result<()> {
        let splitter = CodeSplitter::builder()
            .language("rust".to_string())
            .build()?;

        let input = Document {
            content: r#"
/// A trait for printable objects
pub trait Printable {
    fn print(&self);
}

impl Printable for String {
    fn print(&self) {
        println!("{}", self);
    }
}

impl<T: std::fmt::Debug> Printable for Vec<T> {
    /// Some comment
    fn print(&self) {
        println!("{:?}", self);
    }
}
            "#
            .trim()
            .to_string(),
            metadata: Default::default(),
        };

        let split_docs = splitter.split(vec![input]).await?;
        dbg!(&split_docs);

        assert_eq!(split_docs.len(), 3);
        assert_eq!(
            split_docs[0].content.lines().next().unwrap(),
            "/// A trait for printable objects"
        );
        assert_eq!(
            split_docs[1].content.lines().next().unwrap(),
            "impl Printable for String {"
        );
        assert_eq!(
            split_docs[2].content.lines().next().unwrap(),
            "impl<T: std::fmt::Debug> Printable for Vec<T> {"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_rust_top_level_extraction() -> Result<()> {
        let splitter = CodeSplitter::builder()
            .language("rust".to_string())
            .build()?;

        let input = Document {
            content: r#"
/// A constant value
const PI: f64 = 3.14159;

/// A type alias
type Point = (f64, f64);

/// A struct representing a circle
#[derive(Debug, Clone, Copy)]
pub struct Circle {
    radius: f64,
}

/// A function to calculate area
pub fn calculate_area(radius: f64) -> f64 {
    PI * radius * radius
}

/// A macro for creating a point
#[macro_export]
macro_rules! point {
    ($x:expr, $y:expr) => {
        Point($x, $y)
    };
}
            "#
            .trim()
            .to_string(),
            metadata: Default::default(),
        };

        let split_docs = splitter.split(vec![input]).await?;
        dbg!(&split_docs);

        assert_eq!(split_docs.len(), 5);
        assert_eq!(
            split_docs[0].content.lines().next().unwrap(),
            "/// A constant value"
        );
        assert_eq!(
            split_docs[1].content.lines().next().unwrap(),
            "/// A type alias"
        );
        assert_eq!(
            split_docs[2].content.lines().next().unwrap(),
            "/// A struct representing a circle"
        );
        assert_eq!(
            split_docs[3].content.lines().next().unwrap(),
            "/// A function to calculate area"
        );
        assert_eq!(
            split_docs[4].content.lines().next().unwrap(),
            "/// A macro for creating a point"
        );

        Ok(())
    }
}
