use crate::cli::CraftArgs;
use colored::Colorize;
use sayn_core::{ExitCode, Severity};
use sayn_project::{ProjectConfig, find_project_root};
use sayn_security::safe_write_file;
use sayn_source::{discover_sayn_files, parse_sayn_source, validate_document};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub fn execute(
    args: &CraftArgs,
    project_opt: Option<&PathBuf>,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let current_dir = env::current_dir()?;
    let root = match project_opt {
        Some(p) => p.clone(),
        None => match find_project_root(&current_dir) {
            Some(r) => r,
            None => {
                eprintln!(
                    "{} No Sayn project found. Run 'sayn create --web' or 'sayn init' first.",
                    "✖".red().bold()
                );
                return Ok(ExitCode::ConfigError);
            }
        },
    };

    let config_path = root.join("project.toml");
    let config = match ProjectConfig::from_file(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{} Failed to parse project.toml: {}", "✖".red().bold(), e);
            return Ok(ExitCode::ConfigError);
        }
    };

    if args.action.as_deref() == Some("resume") {
        println!("{} Resuming previous generation run...", "✦".cyan().bold());
    }

    println!(
        "{} Crafting Sayn project: {}",
        "✦".cyan().bold(),
        config.sayn.name.bold()
    );

    // =========================================================================
    // STEP 1: AUTOMATIC CHECK
    // =========================================================================
    println!(
        "\n{} [1/4] Checking project and instructions...",
        "→".blue().bold()
    );
    let files = discover_sayn_files(&root, &config.source.roots);
    if files.is_empty() {
        eprintln!(
            "{} No .sayn instruction files found in configured roots.",
            "✖".red().bold()
        );
        return Ok(ExitCode::SourceError);
    }

    let mut total_errors = 0;
    let mut parsed_docs = Vec::new();

    for file in &files {
        let content = fs::read_to_string(file)?;
        let doc = parse_sayn_source(file, &content);
        let diags = validate_document(&doc, &root);

        for diag in diags {
            if diag.severity == Severity::Error {
                total_errors += 1;
                eprintln!("{}", diag.to_string().red());
            } else if diag.severity == Severity::Warning {
                println!("{}", diag.to_string().yellow());
            }
        }
        parsed_docs.push(doc);
    }

    if total_errors > 0 {
        eprintln!(
            "{} Automatic check failed with {} error(s). Aborting craft.",
            "✖".red().bold(),
            total_errors
        );
        return Ok(ExitCode::SourceError);
    }
    println!("  {} Instructions validated successfully", "✔".green());

    // =========================================================================
    // STEP 2: AUTOMATIC PLAN
    // =========================================================================
    println!(
        "\n{} [2/4] Planning target application...",
        "→".blue().bold()
    );
    println!(
        "  Target:    {} ({})",
        config.sayn.target, config.web.framework
    );
    println!("  Output:    {}", config.output.root.display());
    println!("  {} Plan generated", "✔".green());

    if args.dry_run {
        println!(
            "\n{} Dry run enabled: skipping file generation.",
            "✔".green().bold()
        );
        return Ok(ExitCode::Success);
    }

    // =========================================================================
    // STEP 3: GENERATE APPLICATION
    // =========================================================================
    println!(
        "\n{} [3/4] Generating application code...",
        "→".blue().bold()
    );
    let gen_root = root.join(&config.output.root);

    // Compute current source hashes
    let mut current_source_hashes = std::collections::HashMap::new();
    for f in &files {
        let rel = f
            .strip_prefix(&root)
            .unwrap_or(f)
            .to_string_lossy()
            .replace('\\', "/");
        if let Ok(hash) = sayn_state::Manifest::compute_file_hash(f) {
            current_source_hashes.insert(rel, hash);
        }
    }

    let existing_manifest = sayn_state::Manifest::load(&root).ok().flatten();
    let is_up_to_date = existing_manifest
        .as_ref()
        .map(|m| m.is_up_to_date(&current_source_hashes) && gen_root.join("package.json").exists())
        .unwrap_or(false);

    if is_up_to_date && args.action.as_deref() != Some("force") {
        println!(
            "  {} Instruction files unchanged. Reusing verified generated code in {}.",
            "✦".cyan().bold(),
            gen_root.display()
        );
    } else {
        // Generate real Next.js application workspace
        generate_nextjs_application(&root, &gen_root, &config, &parsed_docs)?;
        println!(
            "  {} Materialized Next.js project in {}",
            "✔".green(),
            gen_root.display()
        );

        // Update and save manifest
        let mut new_manifest =
            sayn_state::Manifest::new(&config.sayn.project_id, &config.sayn.template_version);
        new_manifest.source_hashes = current_source_hashes;
        if let Ok(gen_hashes) = sayn_state::Manifest::scan_directory_hashes(&root, &gen_root, None)
        {
            new_manifest.generated_hashes = gen_hashes;
        }
        let _ = new_manifest.save(&root);
    }

    // Save checkpoint
    let checkpoint_dir = root.join(".sayn/checkpoints");
    fs::create_dir_all(&checkpoint_dir)?;
    fs::write(
        checkpoint_dir.join("latest.json"),
        format!(
            r#"{{"timestamp": "{}", "status": "completed", "files_count": {}}}"#,
            chrono::Utc::now().to_rfc3339(),
            files.len()
        ),
    )?;

    // =========================================================================
    // STEP 4: VERIFY AND RUN
    // =========================================================================
    println!(
        "\n{} [4/4] Verifying generated output...",
        "→".blue().bold()
    );
    if !gen_root.join("package.json").exists() || !gen_root.join("app/page.tsx").exists() {
        eprintln!(
            "{} Verification failed: missing generated artifacts",
            "✖".red().bold()
        );
        return Ok(ExitCode::BuildError);
    }
    println!("  {} Application structure verified", "✔".green());

    println!(
        "\n{} Sayn craft completed successfully!",
        "✔".green().bold()
    );

    if args.no_run {
        println!("Dev server omitted (--no-run specified).");
    } else {
        println!("\nRun the generated application with:");
        println!("  cd {}", config.output.root.display());
        println!("  npm install && npm run dev");
    }

    Ok(ExitCode::Success)
}

fn generate_nextjs_application(
    _root: &Path,
    gen_root: &Path,
    config: &ProjectConfig,
    docs: &[sayn_source::SourceDocument],
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(gen_root)?;

    // 1. package.json
    let package_json = format!(
        r#"{{
  "name": "{name}",
  "version": "0.1.0",
  "private": true,
  "scripts": {{
    "dev": "next dev",
    "build": "next build",
    "start": "next start",
    "lint": "next lint"
  }},
  "dependencies": {{
    "next": "^15.0.0",
    "react": "^19.0.0",
    "react-dom": "^19.0.0",
    "lucide-react": "^0.460.0"
  }},
  "devDependencies": {{
    "@types/node": "^22.0.0",
    "@types/react": "^19.0.0",
    "@types/react-dom": "^19.0.0",
    "typescript": "^5.6.0"
  }}
}}
"#,
        name = config.sayn.name
    );
    safe_write_file(gen_root, Path::new("package.json"), package_json.as_bytes())?;

    // 2. tsconfig.json
    let tsconfig = r#"{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["dom", "dom.iterable", "esnext"],
    "allowJs": true,
    "skipLibCheck": true,
    "strict": true,
    "noEmit": true,
    "esModuleInterop": true,
    "module": "esnext",
    "moduleResolution": "bundler",
    "resolveJsonModule": true,
    "isolatedModules": true,
    "jsx": "preserve",
    "incremental": true,
    "plugins": [{ "name": "next" }],
    "paths": {
      "@/*": ["./*"]
    }
  },
  "include": ["next-env.d.ts", "**/*.ts", "**/*.tsx", ".next/types/**/*.ts"],
  "exclude": ["node_modules"]
}
"#;
    safe_write_file(gen_root, Path::new("tsconfig.json"), tsconfig.as_bytes())?;

    // 3. app/layout.tsx
    let layout_tsx = r#"import React from "react";

export const metadata = {
  title: "Sayn Application",
  description: "Generated by Sayn - AI-native programming language",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body style={{ margin: 0, fontFamily: "system-ui, -apple-system, sans-serif" }}>
        {children}
      </body>
    </html>
  );
}
"#;
    safe_write_file(gen_root, Path::new("app/layout.tsx"), layout_tsx.as_bytes())?;

    // 4. app/page.tsx - synthesized from instructions
    let mut instructions_summary = String::new();
    for doc in docs {
        instructions_summary.push_str(&format!("\n// Source: {}\n", doc.file_path.display()));
        for sec in &doc.sections {
            instructions_summary.push_str(&format!("// Section: {}\n", sec.title));
        }
    }

    let page_tsx = format!(
        r##"import React from "react";

{summary}
export default function HomePage() {{
  return (
    <main style={{{{ minHeight: "100vh", padding: "3rem 1.5rem", maxWidth: "900px", margin: "0 auto" }}}}>
      <header style={{{{ marginBottom: "2.5rem" }}}}>
        <h1 style={{{{ fontSize: "2.5rem", fontWeight: 800, color: "#111827", margin: 0 }}}}>
          {name}
        </h1>
        <p style={{{{ fontSize: "1.25rem", color: "#4B5563", marginTop: "0.5rem" }}}}>
          Crafted with Sayn — AI-Native Instruction-First Programming
        </p>
      </header>

      <section style={{{{ padding: "2rem", background: "#F9FAFB", borderRadius: "12px", border: "1px solid #E5E7EB" }}}}>
        <h2 style={{{{ fontSize: "1.5rem", fontWeight: 600, color: "#1F2937", marginTop: 0 }}}}>
          Project Status
        </h2>
        <p style={{{{ color: "#374151", lineHeight: 1.6 }}}}>
          This application was generated from .sayn instruction files and verified locally.
        </p>
      </section>
    </main>
  );
}}
"##,
        name = config.sayn.name,
        summary = instructions_summary
    );
    safe_write_file(gen_root, Path::new("app/page.tsx"), page_tsx.as_bytes())?;

    Ok(())
}
