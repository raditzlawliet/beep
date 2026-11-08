//! `.beep` variable files: collection vars and environments.
//!
//! Both walk from the project root down to the `.http` file's directory (deeper wins).
//! - Collection: `.beep/vars.<part>.json` alphabetically, then `.beep/vars.json` last.
//! - Environment: `.beep/env.json` (Default) or `.beep/env.<name>.json` (selected name only).

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::context::{VarScope, VarSource, VarStore};

const VARS_DIR: &str = ".beep";
const VARS_FILE: &str = "vars.json";
const VARS_PREFIX: &str = "vars.";
const ENV_PREFIX: &str = "env.";
const JSON_EXT: &str = ".json";
const VALUE_FIELD: &str = "value";
const ENABLED_FIELD: &str = "enabled";

/// Load the merged folder variables for a `.http` file in `file_dir`.
///
/// With `project_root` set and containing `file_dir`, every folder from the root down to
/// `file_dir` is read. Otherwise only `file_dir` is read.
pub fn load_folder_vars(file_dir: &Path, project_root: Option<&Path>) -> Result<VarStore, String> {
    let mut store = VarStore::new();
    for v in load_folder_var_sources(file_dir, project_root)? {
        store.set(v.key, v.value);
    }
    Ok(store)
}

/// Every folder variable with the file it comes from, in load order (later wins).
pub fn load_folder_var_sources(
    file_dir: &Path,
    project_root: Option<&Path>,
) -> Result<Vec<VarSource>, String> {
    let mut out = Vec::new();
    for dir in cascade_dirs(file_dir, project_root) {
        for file in vars_files(&dir.join(VARS_DIR))? {
            load_vars_file(&file, VarScope::Folder, &mut out)?;
        }
    }
    Ok(out)
}

/// Serializes `write_env_var` read-modify-write so concurrent sets do not overwrite each other.
static ENV_WRITE_LOCK: Mutex<()> = Mutex::new(());

/// Env name usable in a file name: no path separators or leading dot, so it stays in `.beep`.
fn is_valid_env_name(name: &str) -> bool {
    !name.is_empty() && !name.contains(['/', '\\', ':', ' ', '\t']) && !name.starts_with('.')
}

fn check_env_name(env: Option<&str>) -> Result<(), String> {
    match env {
        Some(name) if !is_valid_env_name(name) => Err(format!("Invalid environment name '{name}'")),
        _ => Ok(()),
    }
}

/// Env file name: `env.json` for Default (`None`), else `env.<name>.json`.
fn env_file_name(env: Option<&str>) -> String {
    match env {
        Some(name) => format!("{ENV_PREFIX}{name}{JSON_EXT}"),
        None => format!("env{JSON_EXT}"),
    }
}

/// Existing files of one environment along the cascade, in load order (project root first).
pub fn env_files(file_dir: &Path, project_root: Option<&Path>, env: Option<&str>) -> Vec<PathBuf> {
    if check_env_name(env).is_err() {
        return Vec::new();
    }
    cascade_dirs(file_dir, project_root)
        .into_iter()
        .map(|dir| dir.join(VARS_DIR).join(env_file_name(env)))
        .filter(|f| f.is_file())
        .collect()
}

/// File to open for editing an environment: the first existing one in load order,
/// else `{}` created in the farthest folder (project root).
pub fn env_file_for_edit(
    file_dir: &Path,
    project_root: Option<&Path>,
    env: Option<&str>,
) -> Result<PathBuf, String> {
    check_env_name(env)?;
    if let Some(first) = env_files(file_dir, project_root, env).into_iter().next() {
        return Ok(first);
    }
    let farthest = cascade_dirs(file_dir, project_root)
        .into_iter()
        .next()
        .unwrap_or_else(|| file_dir.to_path_buf());
    let dir = farthest.join(VARS_DIR);
    let path = dir.join(env_file_name(env));
    fs::create_dir_all(&dir).map_err(|e| format!("Cannot create '{}': {e}", dir.display()))?;
    fs::write(&path, "{}\n")
        .map_err(|e| format!("Cannot write env file '{}': {e}", path.display()))?;
    Ok(path)
}

/// Create `<project_root>/.beep/env.<name>.json` with `{}` (kept as-is when it exists).
pub fn create_env(project_root: &Path, name: &str) -> Result<PathBuf, String> {
    let name = name.trim();
    check_env_name(Some(name))?;
    let dir = project_root.join(VARS_DIR);
    let path = dir.join(env_file_name(Some(name)));
    if !path.is_file() {
        fs::create_dir_all(&dir).map_err(|e| format!("Cannot create '{}': {e}", dir.display()))?;
        fs::write(&path, "{}\n")
            .map_err(|e| format!("Cannot write env file '{}': {e}", path.display()))?;
    }
    Ok(path)
}

/// Environment names (`env.<name>.json`) found along the cascade, sorted. Default is not listed.
pub fn list_envs(file_dir: &Path, project_root: Option<&Path>) -> Vec<String> {
    let mut names = BTreeSet::new();
    for dir in cascade_dirs(file_dir, project_root) {
        let Ok(entries) = fs::read_dir(dir.join(VARS_DIR)) else { continue };
        for entry in entries.flatten() {
            let file = entry.file_name().to_string_lossy().to_string();
            if let Some(name) = file.strip_prefix(ENV_PREFIX).and_then(|r| r.strip_suffix(JSON_EXT)) {
                if !name.is_empty() && entry.path().is_file() {
                    names.insert(name.to_string());
                }
            }
        }
    }
    names.into_iter().collect()
}

/// Every value of the selected environment with its file, in load order (later wins).
pub fn load_env_var_sources(
    file_dir: &Path,
    project_root: Option<&Path>,
    env: Option<&str>,
) -> Result<Vec<VarSource>, String> {
    let mut out = Vec::new();
    for file in env_files(file_dir, project_root, env) {
        load_vars_file(&file, VarScope::Env, &mut out)?;
    }
    Ok(out)
}

/// Merged values of the selected environment.
pub fn load_env_vars(
    file_dir: &Path,
    project_root: Option<&Path>,
    env: Option<&str>,
) -> Result<VarStore, String> {
    let mut store = VarStore::new();
    for v in load_env_var_sources(file_dir, project_root, env)? {
        store.set(v.key, v.value);
    }
    Ok(store)
}

/// Save `key = value` into the selected environment: the deepest env file that already has
/// `key`, else the farthest one (project root, or `file_dir` without a project), created if
/// missing. Other keys and their order are kept. Returns the written file.
pub fn write_env_var(
    file_dir: &Path,
    project_root: Option<&Path>,
    env: Option<&str>,
    key: &str,
    value: &str,
) -> Result<PathBuf, String> {
    check_env_name(env)?;
    let _guard = ENV_WRITE_LOCK
        .lock()
        .map_err(|_| "Env write lock poisoned".to_string())?;
    let existing = load_env_var_sources(file_dir, project_root, env)?
        .into_iter()
        .rev()
        .find(|v| v.key == key)
        .and_then(|v| v.origin.map(PathBuf::from));
    let path = match existing {
        Some(p) => p,
        None => {
            let farthest = cascade_dirs(file_dir, project_root)
                .into_iter()
                .next()
                .unwrap_or_else(|| file_dir.to_path_buf());
            farthest.join(VARS_DIR).join(env_file_name(env))
        }
    };

    let mut crlf = false;
    let mut map = if path.is_file() {
        let text = fs::read_to_string(&path)
            .map_err(|e| format!("Cannot read env file '{}': {e}", path.display()))?;
        crlf = text.contains("\r\n");
        match serde_json::from_str(&text) {
            Ok(serde_json::Value::Object(m)) => m,
            _ => return Err(format!("Env file '{}' must contain a JSON object", path.display())),
        }
    } else {
        serde_json::Map::new()
    };
    let new_value = serde_json::Value::String(value.to_string());
    match map.get_mut(key) {
        // long form: only `value` changes, description/enabled/extra fields kept
        Some(serde_json::Value::Object(entry)) => {
            entry.insert(VALUE_FIELD.to_string(), new_value);
        }
        _ => {
            map.insert(key.to_string(), new_value);
        }
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Cannot create '{}': {e}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(&serde_json::Value::Object(map))
        .map_err(|e| e.to_string())?
        + "\n";
    // Keep the file's own line endings (pretty JSON escapes newlines inside strings).
    let text = if crlf { text.replace('\n', "\r\n") } else { text };
    // Temp file + rename, so lock-free readers see the old or the new file, never a partial one.
    // Leading dot keeps it out of the env/vars file patterns.
    let file_name = path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default();
    let tmp = path.with_file_name(format!(".{file_name}.tmp"));
    let write_err = |e: std::io::Error| format!("Cannot write env file '{}': {e}", path.display());
    fs::write(&tmp, text).map_err(write_err)?;
    if let Err(e) = fs::rename(&tmp, &path) {
        let _ = fs::remove_file(&tmp);
        return Err(write_err(e));
    }
    Ok(path)
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
        } else if name.starts_with(VARS_PREFIX) && name.ends_with(JSON_EXT) {
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

/// Text of a scalar JSON value (string as-is, number/bool as text).
fn scalar_text(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Append one JSON vars file to `out`. Entry = scalar, or `{ "value": scalar,
/// "enabled"?: bool, "description"?: string }` (`enabled: false` skipped).
/// null/arrays/objects without a scalar `value` ignored.
fn load_vars_file(path: &Path, scope: VarScope, out: &mut Vec<VarSource>) -> Result<(), String> {
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
        let text = match &value {
            serde_json::Value::Object(entry) => {
                if entry.get(ENABLED_FIELD) == Some(&serde_json::Value::Bool(false)) {
                    continue;
                }
                entry.get(VALUE_FIELD).and_then(scalar_text)
            }
            other => scalar_text(other),
        };
        let Some(text) = text else { continue };
        out.push(VarSource {
            key,
            scope,
            value: text,
            origin: Some(path.display().to_string()),
        });
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
        write(&root.join(".beep/vars.b.json"), r#"{"k": "b", "only_b": "1"}"#);
        write(&root.join(".beep/vars.a.json"), r#"{"k": "a", "x": "a"}"#);
        write(&root.join(".beep/vars.json"), r#"{"x": "main"}"#);
        write(&root.join(".beep/other.json"), r#"{"k": "ignored"}"#);
        write(&root.join(".beep/z_vars.json"), r#"{"k": "old pattern ignored"}"#);
        write(&root.join(".beep/env.json"), r#"{"k": "env is not collection"}"#);

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
    fn test_env_list_and_load() {
        let root = temp_dir("env-load");
        write(&root.join(".beep/env.json"), r#"{"host": "default"}"#);
        write(&root.join(".beep/env.dev.json"), r#"{"host": "dev", "only_dev": "1"}"#);
        write(&root.join(".beep/env.prod_eu.json"), r#"{"host": "prod"}"#);
        write(&root.join("a/.beep/env.dev.json"), r#"{"host": "dev-a"}"#);
        write(&root.join("a/.beep/env.qa.json"), r#"{"host": "qa"}"#);

        assert_eq!(list_envs(&root, Some(&root)), vec!["dev", "prod_eu"]);
        assert_eq!(list_envs(&root.join("a"), Some(&root)), vec!["dev", "prod_eu", "qa"]);

        let d = load_env_vars(&root, Some(&root), None).unwrap();
        assert_eq!(d.get("host"), Some("default"));
        let dev = load_env_vars(&root.join("a"), Some(&root), Some("dev")).unwrap();
        assert_eq!(dev.get("host"), Some("dev-a"));
        assert_eq!(dev.get("only_dev"), Some("1"));
        // A named env does not inherit env.json.
        let qa = load_env_vars(&root, Some(&root), Some("qa")).unwrap();
        assert!(qa.is_empty());
        // Env files are not collection vars.
        assert!(load_folder_vars(&root, Some(&root)).unwrap().is_empty());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_long_form_entries() {
        let root = temp_dir("long-form");
        write(
            &root.join(".beep/vars.json"),
            r#"{"a": {"value": "1", "description": "d"}, "b": {"value": 2, "enabled": false},
               "c": {"value": true, "enabled": true}, "d": {"description": "no value"}, "e": "plain"}"#,
        );
        let vars = load_folder_vars(&root, Some(&root)).unwrap();
        assert_eq!(vars.get("a"), Some("1"));
        assert_eq!(vars.get("b"), None);
        assert_eq!(vars.get("c"), Some("true"));
        assert_eq!(vars.get("d"), None);
        assert_eq!(vars.get("e"), Some("plain"));

        // env.vars.set on a long-form key keeps description + extra fields
        write(&root.join(".beep/env.json"), r#"{"t": {"value": "old", "description": "tok", "x": 1}}"#);
        write_env_var(&root, Some(&root), None, "t", "new").unwrap();
        let json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(root.join(".beep/env.json")).unwrap()).unwrap();
        assert_eq!(json["t"], serde_json::json!({"value": "new", "description": "tok", "x": 1}));
    }

    #[test]
    fn test_env_files_and_edit_target() {
        let root = temp_dir("env-files");
        let sub = root.join("a/b");
        fs::create_dir_all(&sub).unwrap();
        // none yet -> created at project root
        let p = env_file_for_edit(&sub, Some(&root), Some("dev")).unwrap();
        assert_eq!(p, root.join(".beep/env.dev.json"));
        assert_eq!(fs::read_to_string(&p).unwrap(), "{}\n");
        write(&root.join("a/.beep/env.dev.json"), "{}");
        assert_eq!(
            env_files(&sub, Some(&root), Some("dev")),
            vec![root.join(".beep/env.dev.json"), root.join("a/.beep/env.dev.json")]
        );
        assert!(env_files(&sub, Some(&root), None).is_empty());
        // existing -> first in load order, nothing created
        assert_eq!(env_file_for_edit(&sub, Some(&root), Some("dev")).unwrap(), p);
    }

    #[test]
    fn test_create_env() {
        let root = temp_dir("env-create");
        let p = create_env(&root, "stage").unwrap();
        assert_eq!(p, root.join(".beep/env.stage.json"));
        assert_eq!(fs::read_to_string(&p).unwrap(), "{}\n");
        assert_eq!(list_envs(&root, Some(&root)), vec!["stage"]);
        // existing file untouched
        fs::write(&p, r#"{"a": "1"}"#).unwrap();
        create_env(&root, "stage").unwrap();
        assert_eq!(fs::read_to_string(&p).unwrap(), r#"{"a": "1"}"#);
        for bad in ["", "a/b", "a\\b", "a b", ".x"] {
            assert!(create_env(&root, bad).is_err(), "{bad}");
        }
        // Path-escaping names rejected at every entry point.
        let bad = Some("../../evil");
        assert!(env_files(&root, Some(&root), bad).is_empty());
        assert!(env_file_for_edit(&root, Some(&root), bad).is_err());
        assert!(write_env_var(&root, Some(&root), bad, "k", "v").is_err());
        assert!(load_env_vars(&root, Some(&root), bad).unwrap().is_empty());
    }

    #[test]
    fn test_write_env_var_keeps_crlf() {
        let root = temp_dir("env-crlf");
        write(&root.join(".beep/env.json"), "{\r\n  \"a\": \"1\"\r\n}\r\n");
        write_env_var(&root, Some(&root), None, "b", "2").unwrap();
        assert_eq!(
            fs::read_to_string(root.join(".beep/env.json")).unwrap(),
            "{\r\n  \"a\": \"1\",\r\n  \"b\": \"2\"\r\n}\r\n"
        );
        // Temp file renamed away, nothing left behind.
        assert!(!root.join(".beep/.env.json.tmp").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_write_env_var() {
        let root = temp_dir("env-write");
        let sub = root.join("a");
        write(&root.join(".beep/env.dev.json"), "{\n  \"z\": \"1\",\n  \"a\": 2\n}\n");
        write(&sub.join(".beep/env.dev.json"), r#"{"token": "old"}"#);

        // Existing key: deepest file that has it.
        let p = write_env_var(&sub, Some(&root), Some("dev"), "token", "new").unwrap();
        assert_eq!(p, sub.join(".beep/env.dev.json"));
        assert_eq!(load_env_vars(&sub, Some(&root), Some("dev")).unwrap().get("token"), Some("new"));

        // New key: farthest (project root) file, key order kept.
        let p = write_env_var(&sub, Some(&root), Some("dev"), "m", "x").unwrap();
        assert_eq!(p, root.join(".beep/env.dev.json"));
        assert_eq!(
            fs::read_to_string(&p).unwrap(),
            "{\n  \"z\": \"1\",\n  \"a\": 2,\n  \"m\": \"x\"\n}\n"
        );

        // Default env, file missing: created at the root.
        let p = write_env_var(&sub, Some(&root), None, "token", "t").unwrap();
        assert_eq!(p, root.join(".beep/env.json"));
        assert_eq!(load_env_vars(&root, Some(&root), None).unwrap().get("token"), Some("t"));

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
