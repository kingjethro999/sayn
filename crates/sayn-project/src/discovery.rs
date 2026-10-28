use std::path::{Path, PathBuf};

pub fn find_project_root(start_dir: &Path) -> Option<PathBuf> {
    let mut current = if start_dir.is_file() {
        start_dir.parent()?.to_path_buf()
    } else {
        start_dir.to_path_buf()
    };

    loop {
        if current.join("project.toml").is_file() || current.join(".sayn").is_dir() {
            return Some(current);
        }

        match current.parent() {
            Some(parent) if parent != current => {
                current = parent.to_path_buf();
            }
            _ => break,
        }
    }

    None
}
