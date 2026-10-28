use sayn_ir::{ComponentPlan, DependencyPlan, RoutePlan, TokenEstimate};
use sayn_source::{DirectiveKind, SourceDocument};
use std::path::{Path, PathBuf};

pub struct RequirementAnalysis {
    pub routes: Vec<RoutePlan>,
    pub components: Vec<ComponentPlan>,
    pub dependencies: Vec<DependencyPlan>,
    pub token_estimate: TokenEstimate,
    pub warnings: Vec<String>,
}

pub fn analyze_requirements(
    _root: &Path,
    docs: &[SourceDocument],
) -> RequirementAnalysis {
    let mut routes = Vec::new();
    let mut components = Vec::new();
    let mut dependencies = Vec::new();
    let warnings = Vec::new();

    let mut total_words = 0;
    let mut has_contact_form = false;
    let mut has_animation = false;
    let mut has_dark_mode = false;

    // Standard Next.js 15 baseline dependencies (matches create-next-app defaults)
    dependencies.push(DependencyPlan {
        name: "next".into(),
        version: "15.3.1".into(),
        is_dev: false,
    });
    dependencies.push(DependencyPlan {
        name: "react".into(),
        version: "^19.0.0".into(),
        is_dev: false,
    });
    dependencies.push(DependencyPlan {
        name: "react-dom".into(),
        version: "^19.0.0".into(),
        is_dev: false,
    });

    // Dev dependencies
    dependencies.push(DependencyPlan {
        name: "typescript".into(),
        version: "^5.0.0".into(),
        is_dev: true,
    });
    dependencies.push(DependencyPlan {
        name: "@types/react".into(),
        version: "^19.0.0".into(),
        is_dev: true,
    });
    dependencies.push(DependencyPlan {
        name: "@types/react-dom".into(),
        version: "^19.0.0".into(),
        is_dev: true,
    });
    dependencies.push(DependencyPlan {
        name: "@types/node".into(),
        version: "^22.0.0".into(),
        is_dev: true,
    });

    // Tailwind CSS v4
    dependencies.push(DependencyPlan {
        name: "tailwindcss".into(),
        version: "^4.0.0".into(),
        is_dev: true,
    });
    dependencies.push(DependencyPlan {
        name: "@tailwindcss/postcss".into(),
        version: "^4.0.0".into(),
        is_dev: true,
    });

    for doc in docs {
        let text = doc.all_text();
        let text_lower = text.to_lowercase();
        total_words += text.split_whitespace().count();

        if text_lower.contains("contact form") || text_lower.contains("contact page") {
            has_contact_form = true;
        }
        if text_lower.contains("animation") || text_lower.contains("animate") {
            has_animation = true;
        }
        if text_lower.contains("dark mode") || text_lower.contains("theme") {
            has_dark_mode = true;
        }

        // Directives
        let mut doc_has_route = false;
        for dir in doc.all_directives() {
            if let DirectiveKind::Route(ref value) = dir.kind {
                routes.push(RoutePlan {
                    route: value.clone(),
                    source_instruction: doc.file_path.clone(),
                    target_file: PathBuf::from(format!("app{}/page.tsx", if value == "/" { "" } else { value })),
                });
                doc_has_route = true;
            }
        }

        if !doc_has_route {
            // Default route based on filename
            let file_stem = doc.file_path.file_stem().and_then(|s| s.to_str()).unwrap_or("page");
            let route_name = if file_stem == "page" || file_stem == "index" {
                "/".to_string()
            } else {
                format!("/{}", file_stem)
            };

            routes.push(RoutePlan {
                route: route_name.clone(),
                source_instruction: doc.file_path.clone(),
                target_file: PathBuf::from(format!("app{}/page.tsx", if route_name == "/" { "" } else { &route_name })),
            });
        }

        // Sections to components
        for section in &doc.sections {
            let heading_clean = section.title.trim_start_matches('#').trim();
            if !heading_clean.is_empty() {
                let comp_name = heading_clean
                    .split_whitespace()
                    .map(|w| {
                        let mut chars = w.chars();
                        match chars.next() {
                            Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                            None => String::new(),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("");

                let valid_name = comp_name
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect::<String>();

                if !valid_name.is_empty() && !components.iter().any(|c: &ComponentPlan| c.name == valid_name) {
                    components.push(ComponentPlan {
                        name: valid_name.clone(),
                        description: format!("Component derived from section '{}'", heading_clean),
                        file_path: PathBuf::from(format!("components/{}.tsx", valid_name)),
                    });
                }
            }
        }
    }

    // Default primary components if not explicitly named
    if !components.iter().any(|c| c.name.to_lowercase().contains("nav")) {
        components.push(ComponentPlan {
            name: "Navbar".into(),
            description: "Responsive top navigation bar".into(),
            file_path: PathBuf::from("components/Navbar.tsx"),
        });
    }

    if has_contact_form && !components.iter().any(|c| c.name.to_lowercase().contains("contact")) {
        components.push(ComponentPlan {
            name: "ContactForm".into(),
            description: "Interactive validated contact form".into(),
            file_path: PathBuf::from("components/ContactForm.tsx"),
        });
    }

    if has_dark_mode && !components.iter().any(|c| c.name.to_lowercase().contains("theme")) {
        components.push(ComponentPlan {
            name: "ThemeToggle".into(),
            description: "Dark and light theme toggle button".into(),
            file_path: PathBuf::from("components/ThemeToggle.tsx"),
        });
    }

    if has_animation {
        dependencies.push(DependencyPlan {
            name: "framer-motion".into(),
            version: "^11.0.0".into(),
            is_dev: false,
        });
    }

    // Calculate token estimates
    let input_min = 2000 + total_words * 3;
    let input_max = 5000 + total_words * 6;
    let output_min = 3500 + components.len() * 1200;
    let output_max = 8000 + components.len() * 2500;

    RequirementAnalysis {
        routes,
        components,
        dependencies,
        token_estimate: TokenEstimate {
            input_min,
            input_max,
            output_min,
            output_max,
        },
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sayn_source::parse_sayn_source;

    #[test]
    fn test_analyze_requirements() {
        let doc_content = r#"
# Developer Portfolio
Create a modern portfolio with dark mode and a contact form.

## Projects
Showcase full-stack applications.
"#;
        let doc = parse_sayn_source(Path::new("app/page.sayn"), doc_content);
        let analysis = analyze_requirements(Path::new("."), &[doc]);

        assert!(!analysis.routes.is_empty());
        assert!(analysis.components.iter().any(|c| c.name == "Navbar"));
        assert!(analysis.components.iter().any(|c| c.name == "ContactForm"));
        assert!(analysis.components.iter().any(|c| c.name == "ThemeToggle"));
        assert!(analysis.token_estimate.input_min > 0);
    }
}
