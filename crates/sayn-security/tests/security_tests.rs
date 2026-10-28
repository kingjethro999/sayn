use sayn_security::{is_safe_relative_path, safe_write_file};
use std::path::Path;
use tempfile::tempdir;

#[test]
fn test_security_rejects_parent_traversal() {
    let bad_paths = [
        "../secret.txt",
        "app/../../etc/passwd",
        "sub/dir/../../../root.txt",
        "..",
        "./../escape",
    ];

    for bad in bad_paths {
        assert!(
            !is_safe_relative_path(Path::new(bad)),
            "Path '{}' should be flagged as unsafe",
            bad
        );
    }
}

#[test]
fn test_security_rejects_absolute_paths() {
    let abs_paths = [
        "/etc/passwd",
        "/var/run/docker.sock",
        "/home/user/.ssh/id_rsa",
    ];

    for abs in abs_paths {
        assert!(
            !is_safe_relative_path(Path::new(abs)),
            "Absolute path '{}' should be rejected",
            abs
        );
    }
}

#[test]
fn test_security_confines_writes_to_root() {
    let temp = tempdir().unwrap();
    let root = temp.path();

    let safe_relative = Path::new("nested/dir/app.tsx");
    let written =
        safe_write_file(root, safe_relative, b"export default function App() {}").unwrap();
    assert!(written.exists());
    assert!(written.starts_with(root));

    // Attempt escaping write
    let escape = Path::new("../outside.txt");
    let result = safe_write_file(root, escape, b"malicious content");
    assert!(result.is_err());
    assert!(!root.parent().unwrap().join("outside.txt").exists());
}
