use crate::analysis::analyze_requirements;
use sayn_ir::{FileOperation, GenerationPlan};
use sayn_project::ProjectConfig;
use sayn_source::SourceDocument;
use std::path::{Path, PathBuf};

pub fn build_generation_plan(
    root: &Path,
    config: &ProjectConfig,
    docs: &[SourceDocument],
) -> Result<GenerationPlan, Box<dyn std::error::Error>> {
    let analysis = analyze_requirements(root, docs);

    let mut operations = Vec::new();

    // Standard Next.js file operations
    operations.push(FileOperation::Create {
        path: PathBuf::from("package.json"),
        description: "Next.js project manifest and dependencies".into(),
    });

    operations.push(FileOperation::Create {
        path: PathBuf::from("tsconfig.json"),
        description: "TypeScript compiler options for Next.js bundler".into(),
    });

    operations.push(FileOperation::Create {
        path: PathBuf::from("app/layout.tsx"),
        description: "Root HTML layout, metadata, and global styles".into(),
    });

    // Route file operations
    for route in &analysis.routes {
        operations.push(FileOperation::Create {
            path: route.target_file.clone(),
            description: format!("Page entrypoint for route '{}'", route.route),
        });
    }

    // Component file operations
    for comp in &analysis.components {
        operations.push(FileOperation::Create {
            path: comp.file_path.clone(),
            description: comp.description.clone(),
        });
    }

    Ok(GenerationPlan {
        project_name: config.sayn.name.clone(),
        target: config.sayn.target,
        framework: config.web.framework,
        instructions_count: docs.len(),
        routes: analysis.routes,
        components: analysis.components,
        dependencies: analysis.dependencies,
        operations,
        token_estimate: analysis.token_estimate,
        warnings: analysis.warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sayn_source::parse_sayn_source;

    #[test]
    fn test_build_generation_plan() {
        use sayn_core::Target;
        let config = ProjectConfig::new("portfolio", Target::Web);
        let doc = parse_sayn_source(
            Path::new("app/page.sayn"),
            "Create a modern web application.",
        );

        let plan = build_generation_plan(Path::new("."), &config, &[doc]).unwrap();
        assert_eq!(plan.project_name, config.sayn.name);
        assert!(!plan.operations.is_empty());
        assert!(plan.operations.iter().any(|op| match op {
            FileOperation::Create { path, .. } => path == &PathBuf::from("package.json"),
            _ => false,
        }));
    }
}
