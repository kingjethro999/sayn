use sayn_core::diagnostic::SourceSpan;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DirectiveKind {
    Route(String),
    Uses(PathBuf),
    Depends(PathBuf),
    Target(String),
    Unknown { name: String, args: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Directive {
    pub kind: DirectiveKind,
    pub raw: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeBlock {
    pub language: Option<String>,
    pub content: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListItem {
    pub text: String,
    pub indent_level: usize,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Paragraph {
    pub text: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentBlock {
    Directive(Directive),
    Code(CodeBlock),
    ListItem(ListItem),
    Paragraph(Paragraph),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Section {
    pub title: String,
    pub blocks: Vec<ContentBlock>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceDocument {
    pub file_path: PathBuf,
    pub content_hash: String,
    pub sections: Vec<Section>,
    pub root_blocks: Vec<ContentBlock>,
}

impl SourceDocument {
    pub fn all_directives(&self) -> Vec<&Directive> {
        let mut directives = Vec::new();
        for block in &self.root_blocks {
            if let ContentBlock::Directive(d) = block {
                directives.push(d);
            }
        }
        for section in &self.sections {
            for block in &section.blocks {
                if let ContentBlock::Directive(d) = block {
                    directives.push(d);
                }
            }
        }
        directives
    }

    pub fn all_text(&self) -> String {
        let mut lines = Vec::new();
        for block in &self.root_blocks {
            match block {
                ContentBlock::Paragraph(p) => lines.push(p.text.clone()),
                ContentBlock::ListItem(li) => lines.push(li.text.clone()),
                ContentBlock::Code(c) => lines.push(c.content.clone()),
                ContentBlock::Directive(d) => lines.push(d.raw.clone()),
            }
        }
        for sec in &self.sections {
            lines.push(sec.title.clone());
            for block in &sec.blocks {
                match block {
                    ContentBlock::Paragraph(p) => lines.push(p.text.clone()),
                    ContentBlock::ListItem(li) => lines.push(li.text.clone()),
                    ContentBlock::Code(c) => lines.push(c.content.clone()),
                    ContentBlock::Directive(d) => lines.push(d.raw.clone()),
                }
            }
        }
        lines.join("\n")
    }
}
