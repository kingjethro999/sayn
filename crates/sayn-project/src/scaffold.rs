use crate::config::ProjectConfig;
use sayn_core::Target;
use std::fs;
use std::path::{Path, PathBuf};

const SAMPLE_PAGE_SAYN: &str = r#"Create a modern, responsive developer portfolio website.

My name is Jordan.
I build full-stack web applications and developer tools.

The design should feel modern, understated, and spacious.
Support light mode and dark mode.

Pages:
- Home page with an introduction and featured projects.
- Projects page with cards and links.
- About page with my experience and skills.
- Contact page with a validated form.

Use a clear type hierarchy and accessible focus indicators.
Prefer reusable components instead of duplicated page layouts.

The contact form must validate inputs and explain errors.
Do not invent testimonials, work experience, or personal information.
"#;

pub fn create_project(name: &str, target: Target, destination: &Path) -> Result<PathBuf, String> {
    if destination.exists() {
        if destination.is_file() {
            return Err(format!(
                "Destination '{}' already exists and is a file",
                destination.display()
            ));
        }
        let read_dir = fs::read_dir(destination).map_err(|e| e.to_string())?;
        if read_dir.count() > 0 {
            return Err(format!(
                "Destination directory '{}' is not empty",
                destination.display()
            ));
        }
    } else {
        fs::create_dir_all(destination).map_err(|e| e.to_string())?;
    }

    // 1. Create project.toml
    let config = ProjectConfig::new(name, target);
    let toml_str = config.to_toml_string().map_err(|e| e.to_string())?;
    fs::write(destination.join("project.toml"), toml_str).map_err(|e| e.to_string())?;

    // 2. Create .sayn directory
    let sayn_dir = destination.join(".sayn");
    fs::create_dir_all(&sayn_dir).map_err(|e| e.to_string())?;
    fs::write(
        sayn_dir.join(".gitignore"),
        "# Sayn internal workspace\ncache/\ncheckpoints/\n",
    )
    .map_err(|e| e.to_string())?;

    // 3. Create app/page.sayn
    let app_dir = destination.join("app");
    fs::create_dir_all(&app_dir).map_err(|e| e.to_string())?;
    fs::write(app_dir.join("page.sayn"), SAMPLE_PAGE_SAYN).map_err(|e| e.to_string())?;

    // 4. Create root .gitignore
    let gitignore_content = r#"# Environment and secrets
.env
.env.*
!.env.example

# Node / target dependencies
node_modules/
.next/
dist/

# Sayn cache
.sayn/cache/
.sayn/checkpoints/
"#;
    fs::write(destination.join(".gitignore"), gitignore_content).map_err(|e| e.to_string())?;

    Ok(destination.to_path_buf())
}

pub fn init_project(name: &str, target: Target, directory: &Path) -> Result<PathBuf, String> {
    if !directory.exists() {
        return Err(format!(
            "Directory '{}' does not exist",
            directory.display()
        ));
    }

    let config_path = directory.join("project.toml");
    if config_path.exists() {
        return Err("Project is already initialized (project.toml exists)".to_string());
    }

    let config = ProjectConfig::new(name, target);
    let toml_str = config.to_toml_string().map_err(|e| e.to_string())?;
    fs::write(config_path, toml_str).map_err(|e| e.to_string())?;

    let sayn_dir = directory.join(".sayn");
    if !sayn_dir.exists() {
        fs::create_dir_all(&sayn_dir).map_err(|e| e.to_string())?;
        fs::write(
            sayn_dir.join(".gitignore"),
            "# Sayn internal workspace\ncache/\ncheckpoints/\n",
        )
        .map_err(|e| e.to_string())?;
    }

    let app_dir = directory.join("app");
    if !app_dir.exists() {
        fs::create_dir_all(&app_dir).map_err(|e| e.to_string())?;
        fs::write(app_dir.join("page.sayn"), SAMPLE_PAGE_SAYN).map_err(|e| e.to_string())?;
    }

    Ok(directory.to_path_buf())
}
