use std::fs;
use std::path::{Path, PathBuf};

pub struct CleanManager;

pub struct CleanReport {
    pub cleaned_paths: Vec<PathBuf>,
}

impl CleanManager {
    pub fn clean_disposable_state(project_root: &Path) -> std::io::Result<CleanReport> {
        let disposable_dirs = [
            project_root.join(".sayn/cache"),
            project_root.join(".sayn/tmp"),
            project_root.join(".sayn/staging"),
            project_root.join(".sayn/logs"),
        ];

        let mut cleaned_paths = Vec::new();

        for dir in &disposable_dirs {
            if dir.exists() {
                fs::remove_dir_all(dir)?;
                cleaned_paths.push(dir.clone());
            }
        }

        Ok(CleanReport { cleaned_paths })
    }
}
