use sayn_core::Target;
use sayn_project::{ProjectConfig, create_project, find_project_root, init_project};
use sayn_source::{discover_sayn_files, parse_sayn_source, validate_document};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_scaffolding_and_validation_workflow() {
    let temp = tempdir().unwrap();
    let project_dir = temp.path().join("my-web-app");

    // 1. Scaffold project
    let created =
        create_project("my-web-app", Target::Web, &project_dir).expect("Project creation failed");
    assert!(created.join("project.toml").exists());
    assert!(created.join("app/page.sayn").exists());
    assert!(created.join(".gitignore").exists());
    assert!(created.join(".sayn").is_dir());

    // 2. Discover root
    let root = find_project_root(&created).expect("Project root discovery failed");
    assert_eq!(root, project_dir);

    // 3. Parse config
    let config =
        ProjectConfig::from_file(&root.join("project.toml")).expect("Failed to parse project.toml");
    assert_eq!(config.sayn.name, "my-web-app");
    assert_eq!(config.sayn.target, Target::Web);

    // 4. Discover and validate .sayn files
    let files = discover_sayn_files(&root, &config.source.roots);
    assert_eq!(files.len(), 1);

    let content = fs::read_to_string(&files[0]).expect("Failed to read page.sayn");
    let doc = parse_sayn_source(&files[0], &content);
    let diags = validate_document(&doc, &root);
    assert!(
        diags.is_empty(),
        "Expected clean validation for template instructions"
    );
}

#[test]
fn test_conflict_detection_on_existing_directory() {
    let temp = tempdir().unwrap();
    let occupied = temp.path().join("occupied");
    fs::create_dir_all(&occupied).unwrap();
    fs::write(occupied.join("existing_file.txt"), "hello").unwrap();

    let result = create_project("occupied", Target::Web, &occupied);
    assert!(
        result.is_err(),
        "Should detect non-empty destination directory"
    );
}

#[test]
fn test_init_in_existing_directory() {
    let temp = tempdir().unwrap();
    let target_dir = temp.path().join("existing-workspace");
    fs::create_dir_all(&target_dir).unwrap();

    let inited = init_project("existing-workspace", Target::Web, &target_dir)
        .expect("Failed to init existing workspace");
    assert!(inited.join("project.toml").exists());
    assert!(inited.join(".sayn").is_dir());
}
