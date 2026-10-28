use crate::ast::{DirectiveKind, SourceDocument};
use sayn_core::diagnostic::Diagnostic;
use std::path::Path;

pub fn validate_document(doc: &SourceDocument, project_root: &Path) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let text_lower = doc.all_text().to_lowercase();

    // Check for obvious contradictions
    let has_no_js = text_lower.contains("never use javascript")
        || text_lower.contains("do not use javascript")
        || text_lower.contains("no javascript");
    let mentions_next = text_lower.contains("next.js")
        || text_lower.contains("nextjs")
        || text_lower.contains("react");

    if has_no_js && mentions_next {
        diagnostics.push(
            Diagnostic::error(
                "E101",
                "Contradictory requirements: project requires Next.js / React but also forbids JavaScript",
            )
            .with_suggestion(
                "Remove either the Next.js target requirement or the prohibition against JavaScript",
            ),
        );
    }

    let forbids_backend = text_lower.contains("do not create any backend")
        || text_lower.contains("no backend functionality");
    let requires_backend = text_lower.contains("submit messages to my backend api")
        || text_lower.contains("create backend api")
        || text_lower.contains("create the api endpoints");

    if forbids_backend && requires_backend {
        diagnostics.push(
            Diagnostic::warning(
                "W102",
                "Conflicting instructions: backend API functionality is requested while backend creation is prohibited",
            )
            .with_suggestion("Clarify whether the backend API already exists externally or should be generated"),
        );
    }

    // Check directives
    for directive in doc.all_directives() {
        match &directive.kind {
            DirectiveKind::Unknown { name, .. } => {
                diagnostics.push(
                    Diagnostic::warning("W103", format!("Unknown directive '@{}'", name))
                        .with_span(directive.span.clone())
                        .with_suggestion(
                            "Supported directives are @route, @uses, @depends, @target",
                        ),
                );
            }
            DirectiveKind::Uses(rel_path) => {
                let resolved = project_root.join(rel_path);
                if !resolved.exists() {
                    diagnostics.push(
                        Diagnostic::warning(
                            "W104",
                            format!(
                                "Referenced asset '{}' does not exist on disk",
                                rel_path.display()
                            ),
                        )
                        .with_span(directive.span.clone())
                        .with_suggestion(
                            "Ensure the asset file exists before running 'sayn craft'",
                        ),
                    );
                }
            }
            DirectiveKind::Depends(rel_path) => {
                let resolved = project_root.join(rel_path);
                if !resolved.exists() {
                    diagnostics.push(
                        Diagnostic::error(
                            "E105",
                            format!(
                                "Referenced dependency file '{}' does not exist",
                                rel_path.display()
                            ),
                        )
                        .with_span(directive.span.clone())
                        .with_suggestion("Check that the dependent .sayn file path is correct"),
                    );
                }
            }
            _ => {}
        }
    }

    diagnostics
}
