use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn discover_sayn_files(root: &Path, source_roots: &[String]) -> Vec<PathBuf> {
    let mut discovered = Vec::new();

    let roots_to_scan: Vec<PathBuf> = if source_roots.is_empty() {
        vec![root.to_path_buf()]
    } else {
        source_roots.iter().map(|sr| root.join(sr)).collect()
    };

    for scan_root in roots_to_scan {
        if !scan_root.exists() {
            continue;
        }

        for entry in WalkDir::new(&scan_root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !is_ignored(e.file_name().to_string_lossy().as_ref()))
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("sayn") {
                discovered.push(path.to_path_buf());
            }
        }
    }

    discovered.sort();
    discovered
}

fn is_ignored(name: &str) -> bool {
    name.starts_with('.')
        || name == "node_modules"
        || name == "target"
        || name == "dist"
        || name == ".next"
        || name == ".git"
}
