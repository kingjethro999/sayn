use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub struct ExportManager;

impl ExportManager {
    pub fn export_clean_application(gen_dir: &Path, target_dir: &Path) -> std::io::Result<usize> {
        if !gen_dir.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Generated directory '{}' does not exist", gen_dir.display()),
            ));
        }

        fs::create_dir_all(target_dir)?;

        let mut copied_files = 0;

        for entry in WalkDir::new(gen_dir)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                name != "node_modules" && name != ".next" && !name.starts_with(".git")
            })
            .filter_map(Result::ok)
        {
            let path = entry.path();
            let rel = path
                .strip_prefix(gen_dir)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

            if rel.as_os_str().is_empty() {
                continue;
            }

            let dest = target_dir.join(rel);

            if entry.file_type().is_dir() {
                fs::create_dir_all(&dest)?;
            } else if entry.file_type().is_file() {
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(path, &dest)?;
                copied_files += 1;
            }
        }

        Ok(copied_files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_export_clean() {
        let dir = tempdir().unwrap();
        let gen_dir = dir.path().join("gen");
        let export_target = dir.path().join("dist");

        fs::create_dir_all(&gen_dir).unwrap();
        fs::write(gen_dir.join("package.json"), "{}").unwrap();
        fs::create_dir_all(gen_dir.join("app")).unwrap();
        fs::write(gen_dir.join("app/page.tsx"), "// page").unwrap();

        // should be excluded
        fs::create_dir_all(gen_dir.join("node_modules")).unwrap();
        fs::write(gen_dir.join("node_modules/dummy.txt"), "dummy").unwrap();

        let count = ExportManager::export_clean_application(&gen_dir, &export_target).unwrap();
        assert_eq!(count, 2);
        assert!(export_target.join("package.json").exists());
        assert!(export_target.join("app/page.tsx").exists());
        assert!(!export_target.join("node_modules").exists());
    }
}
