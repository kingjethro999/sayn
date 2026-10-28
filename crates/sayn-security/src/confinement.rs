use std::fs;
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ConfinementError {
    #[error("Path escapes project root: {0}")]
    PathEscape(String),

    #[error("Absolute paths are not permitted in relative operations: {0}")]
    AbsolutePathForbidden(String),

    #[error("Symlink traversal outside root detected: {0}")]
    SymlinkEscape(String),

    #[error("IO error during confinement check: {0}")]
    Io(String),
}

/// Checks if a relative path is structurally safe (contains no `..`, prefix, or root components).
pub fn is_safe_relative_path(path: &Path) -> bool {
    if path.is_absolute() {
        return false;
    }
    for component in path.components() {
        match component {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return false,
            Component::Normal(_) | Component::CurDir => continue,
        }
    }
    true
}

/// Confines `relative_or_target` to `root`.
/// Returns the canonical or safe combined path if it strictly resides within `root`.
pub fn confine_path(root: &Path, relative_or_target: &Path) -> Result<PathBuf, ConfinementError> {
    if !is_safe_relative_path(relative_or_target) {
        return Err(ConfinementError::PathEscape(
            relative_or_target.display().to_string(),
        ));
    }

    let combined = root.join(relative_or_target);

    // Normalize path by components
    let mut normalized = PathBuf::new();
    for comp in combined.components() {
        match comp {
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err(ConfinementError::PathEscape(
                        relative_or_target.display().to_string(),
                    ));
                }
            }
            Component::CurDir => {}
            other => normalized.push(other.as_os_str()),
        }
    }

    // Verify normalized starts with root (if root exists or can be resolved)
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());

    // If the file exists, canonicalize to verify no symlink escapes
    if normalized.exists() {
        let canonical_normalized = normalized
            .canonicalize()
            .map_err(|e| ConfinementError::Io(e.to_string()))?;
        if !canonical_normalized.starts_with(&canonical_root) {
            return Err(ConfinementError::SymlinkEscape(
                relative_or_target.display().to_string(),
            ));
        }
    }

    Ok(normalized)
}

/// Safely writes content to `file_path` strictly within `root`, creating parent directories if needed.
pub fn safe_write_file(
    root: &Path,
    relative_path: &Path,
    content: &[u8],
) -> Result<PathBuf, ConfinementError> {
    let target = confine_path(root, relative_path)?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| ConfinementError::Io(e.to_string()))?;
    }
    fs::write(&target, content).map_err(|e| ConfinementError::Io(e.to_string()))?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_is_safe_relative_path() {
        assert!(is_safe_relative_path(Path::new("app/page.sayn")));
        assert!(is_safe_relative_path(Path::new(
            "src/components/Header.tsx"
        )));
        assert!(!is_safe_relative_path(Path::new("../escape.txt")));
        assert!(!is_safe_relative_path(Path::new("app/../../escape.txt")));
        assert!(!is_safe_relative_path(Path::new("/etc/passwd")));
    }

    #[test]
    fn test_confine_path_inside() {
        let dir = tempdir().unwrap();
        let safe = confine_path(dir.path(), Path::new("sub/file.txt")).unwrap();
        assert!(safe.starts_with(dir.path()));
    }

    #[test]
    fn test_confine_path_escape_rejected() {
        let dir = tempdir().unwrap();
        let result = confine_path(dir.path(), Path::new("../outside.txt"));
        assert!(result.is_err());
    }
}
