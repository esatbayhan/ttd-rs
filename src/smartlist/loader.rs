use std::fs;
use std::path::Path;

use super::parser::parse_list;
use super::types::SmartList;

/// Recursively walk `lists_dir` collecting `.list` files,
/// parse each one, sort results by filename alphabetically.
pub fn load_all(lists_dir: &Path) -> Vec<SmartList> {
    let mut paths = Vec::new();
    collect_list_files(lists_dir, &mut paths);

    // Sort by (parent directory, filename) so within-group ordering
    // is strictly alphabetical by filename, matching spec recommendation.
    paths.sort_by(|a, b| {
        let dir_a = a.parent().unwrap_or(Path::new(""));
        let dir_b = b.parent().unwrap_or(Path::new(""));
        dir_a.cmp(dir_b).then_with(|| {
            let name_a = a.file_name().unwrap_or_default();
            let name_b = b.file_name().unwrap_or_default();
            name_a.cmp(&name_b)
        })
    });

    paths
        .into_iter()
        .filter_map(|path| {
            let content = fs::read_to_string(&path).ok()?;
            let list = parse_list(&content, &path, lists_dir);
            Some(list)
        })
        .collect()
}

fn collect_list_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_list_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("list") {
            out.push(path);
        }
    }
}
