use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Manifest {
    pub project_id: String,
    pub version: String,
    pub timestamp: DateTime<Utc>,
    pub source_hashes: HashMap<String, String>,
    pub generated_hashes: HashMap<String, String>,
}

impl Manifest {
    pub fn new(project_id: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            project_id: project_id.into(),
            version: version.into(),
            timestamp: Utc::now(),
            source_hashes: HashMap::new(),
            generated_hashes: HashMap::new(),
        }
    }

    pub fn compute_sha256(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hex::encode(hasher.finalize())
    }

    pub fn compute_file_hash(path: &Path) -> std::io::Result<String> {
        let bytes = fs::read(path)?;
        Ok(Self::compute_sha256(&bytes))
    }

    pub fn scan_directory_hashes(
        root: &Path,
        dir: &Path,
        extensions: Option<&[&str]>,
    ) -> std::io::Result<HashMap<String, String>> {
        let mut hashes = HashMap::new();
        if !dir.exists() {
            return Ok(hashes);
        }

        for entry in WalkDir::new(dir)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !name.starts_with('.') && name != "node_modules" && name != "target"
            })
            .filter_map(Result::ok)
        {
            if entry.file_type().is_file() {
                let path = entry.path();
                if let Some(exts) = extensions {
                    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                    if !exts.contains(&ext) {
                        continue;
                    }
                }
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .replace('\\', "/");
                let hash = Self::compute_file_hash(path)?;
                hashes.insert(rel, hash);
            }
        }

        Ok(hashes)
    }

    pub fn is_up_to_date(&self, current_source_hashes: &HashMap<String, String>) -> bool {
        if self.source_hashes.len() != current_source_hashes.len() {
            return false;
        }

        for (path, current_hash) in current_source_hashes {
            match self.source_hashes.get(path) {
                Some(saved_hash) if saved_hash == current_hash => continue,
                _ => return false,
            }
        }

        true
    }

    pub fn manifest_path(project_root: &Path) -> PathBuf {
        project_root.join(".sayn/state/manifest.json")
    }

    pub fn load(project_root: &Path) -> std::io::Result<Option<Self>> {
        let path = Self::manifest_path(project_root);
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(path)?;
        let manifest = serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(Some(manifest))
    }

    pub fn save(&self, project_root: &Path) -> std::io::Result<()> {
        let path = Self::manifest_path(project_root);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_sha256() {
        let hash = Manifest::compute_sha256(b"hello world");
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_is_up_to_date() {
        let mut manifest = Manifest::new("proj-1", "0.0.4");
        manifest
            .source_hashes
            .insert("app/page.sayn".into(), "hash123".into());

        let mut current = HashMap::new();
        current.insert("app/page.sayn".into(), "hash123".into());
        assert!(manifest.is_up_to_date(&current));

        current.insert("app/page.sayn".into(), "hash456".into());
        assert!(!manifest.is_up_to_date(&current));
    }
}
