//! `.env` file for `{{process.env.KEY}}` (SPEC 8.7.1). Read-only: Beep never writes it.

use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

/// `.env` of the project root, else of the `.http` file folder. Missing file = empty.
pub fn load_dotenv(file_dir: Option<&Path>, project_root: Option<&Path>) -> Result<HashMap<String, String>, String> {
    let Some(dir) = project_root.or(file_dir) else {
        return Ok(HashMap::new());
    };
    let path = dir.join(".env");
    match fs::read_to_string(&path) {
        Ok(text) => Ok(parse_dotenv(&text)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(HashMap::new()),
        Err(e) => Err(format!("Cannot read '{}': {e}", path.display())),
    }
}

/// `KEY=value` lines; `#` comments, optional `export `, `"..."` (escapes) or `'...'` (literal).
/// Lines without `=` or with an empty key are skipped. Later keys win.
pub fn parse_dotenv(text: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        out.insert(key.to_string(), parse_value(value.trim()));
    }
    out
}

fn parse_value(v: &str) -> String {
    if let Some(inner) = v.strip_prefix('"').and_then(|r| r.find('"').map(|_| r)) {
        return unescape_double(inner);
    }
    if let Some(rest) = v.strip_prefix('\'')
        && let Some(end) = rest.find('\'')
    {
        return rest[..end].to_string();
    }
    // unquoted: ends at ` #`
    match v.find(" #") {
        Some(i) => v[..i].trim_end().to_string(),
        None => v.to_string(),
    }
}

/// Body of a `"..."` value up to the closing unescaped quote; `\n`, `\r`, `\t`, `\"`, `\\`.
fn unescape_double(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => break,
            '\\' => match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some(o) => out.push(o),
                None => out.push('\\'),
            },
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dotenv() {
        let env = parse_dotenv(
            "# comment\n\
             A=1\n\
             B = spaced value  \n\
             export C=exp\n\
             D=x #trailing\n\
             E=\"q # kept\\nline \\\"in\\\"\" # c\n\
             F='raw \\n # kept'\n\
             noequals\n\
             =novalue\n\
             G=\n\
             A=2\r\n",
        );
        assert_eq!(env.get("A").map(String::as_str), Some("2"));
        assert_eq!(env.get("B").map(String::as_str), Some("spaced value"));
        assert_eq!(env.get("C").map(String::as_str), Some("exp"));
        assert_eq!(env.get("D").map(String::as_str), Some("x"));
        assert_eq!(env.get("E").map(String::as_str), Some("q # kept\nline \"in\""));
        assert_eq!(env.get("F").map(String::as_str), Some("raw \\n # kept"));
        assert_eq!(env.get("G").map(String::as_str), Some(""));
        assert_eq!(env.len(), 7);
    }

    #[test]
    fn test_load_dotenv_root_first() {
        let root = std::env::temp_dir().join(format!("beep-dotenv-{}", std::process::id()));
        let sub = root.join("sub");
        fs::create_dir_all(&sub).unwrap();
        fs::write(root.join(".env"), "K=root").unwrap();
        fs::write(sub.join(".env"), "K=sub").unwrap();
        let get = |m: HashMap<String, String>| m.get("K").cloned();
        assert_eq!(get(load_dotenv(Some(&sub), Some(&root)).unwrap()).as_deref(), Some("root"));
        assert_eq!(get(load_dotenv(Some(&sub), None).unwrap()).as_deref(), Some("sub"));
        assert!(load_dotenv(None, None).unwrap().is_empty());
        assert!(load_dotenv(Some(&root.join("missing")), None).unwrap().is_empty());
        let _ = fs::remove_dir_all(&root);
    }
}
