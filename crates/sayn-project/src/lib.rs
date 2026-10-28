pub mod config;
pub mod discovery;
pub mod doctor;
pub mod scaffold;

pub use config::ProjectConfig;
pub use discovery::find_project_root;
pub use doctor::{DiagnosticCheck, DoctorReport, run_diagnostics};
pub use scaffold::{create_project, init_project};

#[cfg(test)]
mod tests {
    use super::*;
    use sayn_core::Target;
    use tempfile::tempdir;

    #[test]
    fn test_create_project_and_discover() {
        let dir = tempdir().unwrap();
        let project_dir = dir.path().join("my-app");
        let created = create_project("my-app", Target::Web, &project_dir).unwrap();
        assert!(created.join("project.toml").exists());
        assert!(created.join("app/page.sayn").exists());
        assert!(created.join(".sayn").is_dir());

        let discovered = find_project_root(&created).unwrap();
        assert_eq!(discovered, project_dir);

        let config = ProjectConfig::from_file(&created.join("project.toml")).unwrap();
        assert_eq!(config.sayn.name, "my-app");
        assert_eq!(config.sayn.target, Target::Web);
    }
}
