use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotInfo {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub file_count: usize,
    pub total_bytes: u64,
}

pub struct SnapshotManager;

impl SnapshotManager {
    pub fn snapshots_dir(project_root: &Path) -> PathBuf {
        project_root.join(".sayn/snapshots")
    }

    pub fn create_snapshot(
        project_root: &Path,
        gen_dir: &Path,
        custom_id: Option<&str>,
    ) -> std::io::Result<SnapshotInfo> {
        if !gen_dir.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No generated directory to snapshot",
            ));
        }

        let id = match custom_id {
            Some(id) => id.to_string(),
            None => {
                let uuid = uuid::Uuid::new_v4().to_string();
                format!("snap_{}", &uuid[..8])
            }
        };

        let snap_dir = Self::snapshots_dir(project_root).join(&id);
        let content_dir = snap_dir.join("content");
        fs::create_dir_all(&content_dir)?;

        let mut file_count = 0;
        let mut total_bytes = 0;

        for entry in WalkDir::new(gen_dir)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                name != "node_modules" && name != ".next"
            })
            .filter_map(Result::ok)
        {
            let path = entry.path();
            let rel = path
                .strip_prefix(gen_dir)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            let dest = content_dir.join(rel);

            if entry.file_type().is_dir() {
                fs::create_dir_all(&dest)?;
            } else if entry.file_type().is_file() {
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent)?;
                }
                let bytes = fs::copy(path, &dest)?;
                file_count += 1;
                total_bytes += bytes;
            }
        }

        let info = SnapshotInfo {
            id: id.clone(),
            timestamp: Utc::now(),
            file_count,
            total_bytes,
        };

        let meta_file = snap_dir.join("snapshot.json");
        let meta_json = serde_json::to_string_pretty(&info)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(meta_file, meta_json)?;

        Ok(info)
    }

    pub fn rollback(
        project_root: &Path,
        gen_dir: &Path,
        snapshot_id: &str,
    ) -> std::io::Result<SnapshotInfo> {
        let snap_dir = Self::snapshots_dir(project_root).join(snapshot_id);
        if !snap_dir.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Snapshot '{}' not found", snapshot_id),
            ));
        }

        let meta_file = snap_dir.join("snapshot.json");
        let meta_json = fs::read_to_string(&meta_file)?;
        let info: SnapshotInfo = serde_json::from_str(&meta_json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let content_dir = snap_dir.join("content");
        if !content_dir.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Corrupted snapshot: content directory missing",
            ));
        }

        // Clean existing generated files (except node_modules)
        if gen_dir.exists() {
            for entry in fs::read_dir(gen_dir)? {
                let entry = entry?;
                let file_name = entry.file_name();
                if file_name != "node_modules" && file_name != ".next" {
                    let path = entry.path();
                    if path.is_dir() {
                        let _ = fs::remove_dir_all(&path);
                    } else {
                        let _ = fs::remove_file(&path);
                    }
                }
            }
        } else {
            fs::create_dir_all(gen_dir)?;
        }

        // Copy snapshot content into gen_dir
        for entry in WalkDir::new(&content_dir)
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();
            let rel = path
                .strip_prefix(&content_dir)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            let dest = gen_dir.join(rel);

            if entry.file_type().is_dir() {
                fs::create_dir_all(&dest)?;
            } else if entry.file_type().is_file() {
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(path, &dest)?;
            }
        }

        Ok(info)
    }

    pub fn list_snapshots(project_root: &Path) -> std::io::Result<Vec<SnapshotInfo>> {
        let snap_dir = Self::snapshots_dir(project_root);
        let mut list = Vec::new();
        if !snap_dir.exists() {
            return Ok(list);
        }

        for entry in fs::read_dir(snap_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let meta_file = path.join("snapshot.json");
                if let Some(info) = fs::read_to_string(&meta_file)
                    .ok()
                    .and_then(|c| serde_json::from_str::<SnapshotInfo>(&c).ok())
                {
                    list.push(info);
                }
            }
        }

        list.sort_by_key(|a| std::cmp::Reverse(a.timestamp));
        Ok(list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_snapshot_and_rollback() {
        let dir = tempdir().unwrap();
        let project_root = dir.path();
        let gen_dir = project_root.join(".sayn/generated");
        fs::create_dir_all(&gen_dir).unwrap();

        fs::write(gen_dir.join("file1.txt"), "hello v1").unwrap();

        let snap = SnapshotManager::create_snapshot(project_root, &gen_dir, Some("v1")).unwrap();
        assert_eq!(snap.id, "v1");
        assert_eq!(snap.file_count, 1);

        // Modify gen_dir
        fs::write(gen_dir.join("file1.txt"), "hello v2").unwrap();
        assert_eq!(
            fs::read_to_string(gen_dir.join("file1.txt")).unwrap(),
            "hello v2"
        );

        // Rollback
        SnapshotManager::rollback(project_root, &gen_dir, "v1").unwrap();
        assert_eq!(
            fs::read_to_string(gen_dir.join("file1.txt")).unwrap(),
            "hello v1"
        );
    }
}
