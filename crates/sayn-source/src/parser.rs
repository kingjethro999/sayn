use crate::ast::*;
use sayn_core::diagnostic::SourceSpan;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub fn parse_sayn_source(path: &Path, content: &str) -> SourceDocument {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let content_hash = hex::encode(hasher.finalize());

    let lines: Vec<&str> = content.lines().collect();
    let mut root_blocks = Vec::new();
    let mut sections = Vec::new();
    let mut current_section: Option<Section> = None;

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        let line_num = i + 1;

        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        // Code fence
        if trimmed.starts_with("```") {
            let start_line = line_num;
            let lang = trimmed.trim_start_matches('`').trim();
            let language = if lang.is_empty() {
                None
            } else {
                Some(lang.to_string())
            };

            let mut code_lines = Vec::new();
            i += 1;
            while i < lines.len() {
                let code_line = lines[i];
                if code_line.trim().starts_with("```") {
                    break;
                }
                code_lines.push(code_line);
                i += 1;
            }
            let end_line = i + 1;
            let code_content = code_lines.join("\n");
            let span = SourceSpan::new(path, start_line, 1, end_line, 1);
            let code_block = ContentBlock::Code(CodeBlock {
                language,
                content: code_content,
                span,
            });

            if let Some(sec) = &mut current_section {
                sec.blocks.push(code_block);
            } else {
                root_blocks.push(code_block);
            }
            i += 1;
            continue;
        }

        // Directive: starts with @
        if trimmed.starts_with('@') {
            let span = SourceSpan::new(path, line_num, 1, line_num, line.len());
            let directive = parse_directive(trimmed, path, span);
            let block = ContentBlock::Directive(directive);

            if let Some(sec) = &mut current_section {
                sec.blocks.push(block);
            } else {
                root_blocks.push(block);
            }
            i += 1;
            continue;
        }

        // Section header: ends with `:` (and is short) or starts with `#`
        if is_section_header(trimmed) {
            if let Some(prev) = current_section.take() {
                sections.push(prev);
            }
            let title = trimmed
                .trim_start_matches('#')
                .trim()
                .trim_end_matches(':')
                .to_string();
            let span = SourceSpan::new(path, line_num, 1, line_num, line.len());
            current_section = Some(Section {
                title,
                blocks: Vec::new(),
                span,
            });
            i += 1;
            continue;
        }

        // List item: starts with `- ` or `* `
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            let indent_level = line.chars().take_while(|c| c.is_whitespace()).count() / 2;
            let text = trimmed[2..].trim().to_string();
            let span = SourceSpan::new(path, line_num, 1, line_num, line.len());
            let block = ContentBlock::ListItem(ListItem {
                text,
                indent_level,
                span,
            });

            if let Some(sec) = &mut current_section {
                sec.blocks.push(block);
            } else {
                root_blocks.push(block);
            }
            i += 1;
            continue;
        }

        // Default: Paragraph
        let span = SourceSpan::new(path, line_num, 1, line_num, line.len());
        let block = ContentBlock::Paragraph(Paragraph {
            text: trimmed.to_string(),
            span,
        });

        if let Some(sec) = &mut current_section {
            sec.blocks.push(block);
        } else {
            root_blocks.push(block);
        }
        i += 1;
    }

    if let Some(last) = current_section {
        sections.push(last);
    }

    SourceDocument {
        file_path: path.to_path_buf(),
        content_hash,
        sections,
        root_blocks,
    }
}

fn is_section_header(trimmed: &str) -> bool {
    if trimmed.starts_with('#') {
        return true;
    }
    if trimmed.ends_with(':') && !trimmed.contains('\n') && trimmed.split_whitespace().count() <= 4
    {
        let first_word = trimmed.split_whitespace().next().unwrap_or("");
        // Common section headings in Sayn
        matches!(
            first_word.to_lowercase().trim_end_matches(':'),
            "goal"
                | "pages"
                | "data"
                | "behavior"
                | "style"
                | "assets"
                | "integrations"
                | "constraints"
                | "acceptance"
                | "layout"
                | "components"
                | "security"
        )
    } else {
        false
    }
}

fn parse_directive(trimmed: &str, _path: &Path, span: SourceSpan) -> Directive {
    let raw = trimmed.to_string();
    let parts: Vec<&str> = trimmed.splitn(2, char::is_whitespace).collect();
    let name = parts[0].trim_start_matches('@');
    let args = parts.get(1).map(|s| s.trim()).unwrap_or("");

    let kind = match name {
        "route" => DirectiveKind::Route(args.to_string()),
        "uses" => DirectiveKind::Uses(PathBuf::from(args)),
        "depends" => DirectiveKind::Depends(PathBuf::from(args)),
        "target" => DirectiveKind::Target(args.to_string()),
        _ => DirectiveKind::Unknown {
            name: name.to_string(),
            args: args.to_string(),
        },
    };

    Directive { kind, raw, span }
}
