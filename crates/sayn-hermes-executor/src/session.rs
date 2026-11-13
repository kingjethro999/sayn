use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    Running,
    Completed,
    Failed,
    Interrupted,
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionStatus::Running => write!(f, "running"),
            SessionStatus::Completed => write!(f, "completed"),
            SessionStatus::Failed => write!(f, "failed"),
            SessionStatus::Interrupted => write!(f, "interrupted"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSession {
    pub session_id: String,
    pub project_id: String,
    pub run_id: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub model: String,
    pub provider: String,
    pub status: SessionStatus,
    pub source_hashes: HashMap<String, String>,
    pub applied_operations: Vec<String>,
    pub operations_count: usize,
    pub notes: Option<String>,
}

pub struct SessionManager;

impl SessionManager {
    pub fn sessions_dir(project_root: &Path) -> PathBuf {
        project_root.join(".sayn/sessions")
    }

    pub fn save(project_root: &Path, session: &ExecutionSession) -> std::io::Result<()> {
        let dir = Self::sessions_dir(project_root);
        fs::create_dir_all(&dir)?;

        let session_file = dir.join(format!("{}.json", session.run_id));
        let json = serde_json::to_string_pretty(session)?;
        fs::write(session_file, json)?;

        // Also update latest pointer
        let latest_file = dir.join("latest.json");
        fs::write(latest_file, serde_json::to_string_pretty(session)?)?;

        Ok(())
    }

    pub fn load_latest(project_root: &Path) -> Option<ExecutionSession> {
        let latest_file = Self::sessions_dir(project_root).join("latest.json");
        if latest_file.exists() {
            if let Ok(content) = fs::read_to_string(&latest_file) {
                if let Ok(session) = serde_json::from_str::<ExecutionSession>(&content) {
                    return Some(session);
                }
            }
        }
        None
    }

    pub fn list(project_root: &Path) -> Vec<ExecutionSession> {
        let dir = Self::sessions_dir(project_root);
        if !dir.exists() {
            return Vec::new();
        }

        let mut sessions = Vec::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json")
                    && path.file_name().and_then(|n| n.to_str()) != Some("latest.json")
                {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(s) = serde_json::from_str::<ExecutionSession>(&content) {
                            sessions.push(s);
                        }
                    }
                }
            }
        }

        // Sort by started_at descending
        sessions.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        sessions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_save_and_load_session() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let session = ExecutionSession {
            session_id: "sess-1".into(),
            project_id: "proj-1".into(),
            run_id: "run-100".into(),
            started_at: "2026-10-28T12:00:00Z".into(),
            completed_at: Some("2026-10-28T12:01:00Z".into()),
            model: "nous-hermes-3".into(),
            provider: "nous".into(),
            status: SessionStatus::Completed,
            source_hashes: HashMap::new(),
            applied_operations: vec!["Wrote app/page.tsx".into()],
            operations_count: 1,
            notes: None,
        };

        SessionManager::save(root, &session).unwrap();

        let latest = SessionManager::load_latest(root).expect("latest should exist");
        assert_eq!(latest.run_id, "run-100");
        assert_eq!(latest.status, SessionStatus::Completed);

        let list = SessionManager::list(root);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].run_id, "run-100");
    }
}
