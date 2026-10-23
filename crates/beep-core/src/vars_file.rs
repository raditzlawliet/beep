//! `.beep/vars.json` folder cascade loader.
//!
//! Walks from the project root down to the `.http` file's directory. In each folder,
//! `.beep/*_vars.json` load alphabetically, then `.beep/vars.json` last.
//! Later files and deeper folders win.

use std::fs;
use std::path::{Path, PathBuf};

use crate::context::VarStore;

const VARS_DIR: &str = ".beep";
const VARS_FILE: &str = "vars.json";
const VARS_SUFFIX: &str = "_vars.json";

/// Load the merged folder variables for a `.http` file in `file_dir`.
///
/// With `project_root` set and containing `file_dir`, every folder from the root down to
/// `file_dir` is read. Otherwise only `file_dir` is read.
pub fn load_folder_vars(file_dir: &Path, project_root: Option<&Path>) -> Result<VarStore, String> {
    let mut store = VarStore::new();
    for dir in cascade_dirs(file_dir, project_root) {
        for file in vars_files(&dir.join(VARS_DIR))? {
            load_vars_file(&file, &mut store)?;
        }
    }
    Ok(store)
}

/// Folders to read, shallowest first.
fn cascade_dirs(file_dir: &Path, project_root: Option<&Path>) -> Vec<PathBuf> {
    let Some(root) = project_root.filter(|r| file_dir.starts_with(r)) else {
        return vec![file_dir.to_path_buf()];
    };
    let mut dirs: Vec<PathBuf> = file_dir
        .ancestors()
        .take_while(|d| d.starts_with(root))
        .map(Path::to_path_buf)
        .collect();
    dirs.reverse();
    dirs
}

/// Variable files in one `.beep` folder, in load order.
fn vars_files(beep_dir: &Path) -> Result<Vec<PathBuf>, String> {
    if !beep_dir.is_dir() {
        return Ok(Vec::new());
    }
    let entries = fs::read_dir(beep_dir)
        .map_err(|e| format!("Cannot read '{}': {e}", beep_dir.display()))?;

    let mut suffixed: Vec<String> = Vec::new();
    let mut has_main = false;
    for entry in entries.flatten() {
        if !entry.path().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name == VARS_FILE {
            has_main = true;
        } else if name.ends_with(VARS_SUFFIX) {
            suffixed.push(name);
        }
    }
    suffixed.sort();

    let mut files: Vec<PathBuf> = suffixed.iter().map(|n| beep_dir.join(n)).collect();
    if has_main {
        files.push(beep_dir.join(VARS_FILE));
    }
    Ok(files)
}

/// Merge one JSON vars file into `store`. Strings as-is, numbers/bools as text,
/// null/objects/arrays ignored.
fn load_vars_file(path: &Path, store: &mut VarStore) -> Result<(), String> {
    let text = fs::read_to_string(path)
        .map_err(|e| format!("Cannot read vars file '{}': {e}", path.display()))?;
    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("Invalid JSON in vars file '{}': {e}", path.display()))?;
    let serde_json::Value::Object(map) = json else {
        return Err(format!(
            "Vars file '{}' must contain a JSON object",
            path.display()
        ));
    };

    for (key, value) in map {
        let text = match value {
            serde_json::Value::String(s) => s,
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            _ => continue,
        };
        store.set(key, text);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fresh temp dir per test.
    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("beep-vars-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    #[test]
    fn test_cascade_deeper_wins() {
        let root = temp_dir("cascade");
        write(&root.join(".beep/vars.json"), r#"{"host": "root", "only_root": "r"}"#);
        write(&root.join("a/.beep/vars.json"), r#"{"host": "mid"}"#);
        write(&root.join("a/b/.beep/vars.json"), r#"{"host": "deep"}"#);

        let deep = load_folder_vars(&root.join("a/b"), Some(&root)).unwrap();
        assert_eq!(deep.get("host"), Some("deep"));
        assert_eq!(deep.get("only_root"), Some("r"));

        let mid = load_folder_vars(&root.join("a"), Some(&root)).unwrap();
        assert_eq!(mid.get("host"), Some("mid"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_same_level_order() {
        let root = temp_dir("order");
        write(&root.join(".beep/b_vars.json"), r#"{"k": "b", "only_b": "1"}"#);
        write(&root.join(".beep/a_vars.json"), r#"{"k": "a", "x": "a"}"#);
        write(&root.join(".beep/vars.json"), r#"{"x": "main"}"#);
        write(&root.join(".beep/other.json"), r#"{"k": "ignored"}"#);

        let vars = load_folder_vars(&root, Some(&root)).unwrap();
        assert_eq!(vars.get("k"), Some("b"));
        assert_eq!(vars.get("x"), Some("main"));
        assert_eq!(vars.get("only_b"), Some("1"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_value_types() {
        let root = temp_dir("types");
        write(
            &root.join(".beep/vars.json"),
            r#"{"s": "x", "n": 30, "f": 1.5, "b": true, "z": null, "o": {"a": 1}, "arr": [1]}"#,
        );
        let vars = load_folder_vars(&root, None).unwrap();
        assert_eq!(vars.get("s"), Some("x"));
        assert_eq!(vars.get("n"), Some("30"));
        assert_eq!(vars.get("f"), Some("1.5"));
        assert_eq!(vars.get("b"), Some("true"));
        assert_eq!(vars.get("z"), None);
        assert_eq!(vars.get("o"), None);
        assert_eq!(vars.get("arr"), None);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_no_project_reads_only_file_dir() {
        let root = temp_dir("noproject");
        write(&root.join(".beep/vars.json"), r#"{"host": "root"}"#);
        write(&root.join("sub/.beep/vars.json"), r#"{"sub": "1"}"#);

        let vars = load_folder_vars(&root.join("sub"), None).unwrap();
        assert_eq!(vars.get("sub"), Some("1"));
        assert_eq!(vars.get("host"), None);

        // File outside the project root: same as no project.
        let other = temp_dir("noproject-other");
        let vars = load_folder_vars(&root.join("sub"), Some(&other)).unwrap();
        assert_eq!(vars.get("host"), None);

        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&other);
    }

    #[test]
    fn test_invalid_json_errors_with_file_name() {
        let root = temp_dir("invalid");
        write(&root.join(".beep/vars.json"), "{ not json");
        let err = load_folder_vars(&root, None).unwrap_err();
        assert!(err.contains("vars.json"));

        write(&root.join(".beep/vars.json"), "[1, 2]");
        let err = load_folder_vars(&root, None).unwrap_err();
        assert!(err.contains("JSON object"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_missing_beep_dir_is_empty() {
        let root = temp_dir("missing");
        let vars = load_folder_vars(&root, Some(&root)).unwrap();
        assert!(vars.is_empty());
        let _ = fs::remove_dir_all(&root);
    }
}
