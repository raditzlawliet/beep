//! Dynamic variables (`{{$guid}}`, `{{$timestamp}}`, ...).
//!
//! Generated on every use. Random values come from std (`RandomState` + time + counter);
//! good enough for test data, not cryptographically secure.

use std::collections::hash_map::RandomState;
use std::collections::{BTreeSet, HashSet};
use std::hash::{BuildHasher, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{SecondsFormat, Utc};

/// Dynamic variable names with usage and description (for editor autocomplete).
pub const DYNAMIC_VARS: &[(&str, &str, &str)] = &[
    ("$uuid", "$uuid", "Random UUID v4"),
    ("$guid", "$guid", "Random UUID v4 (alias of $uuid)"),
    ("$timestamp", "$timestamp", "Unix timestamp in seconds"),
    ("$datetime", "$datetime iso8601", "Current UTC date-time (ISO 8601)"),
    ("$randomInt", "$randomInt 0 100", "Random integer, min inclusive, max exclusive"),
    ("$processEnv", "$processEnv NAME", "OS environment variable (asks permission)"),
];

/// `$processEnv` permission guard. Records every env name asked for, and the denied ones.
///
/// `allow == None` allows every name (CLI, tests). Clones share the same record.
#[derive(Debug, Clone, Default)]
pub struct EnvGuard {
    allow: Option<Arc<HashSet<String>>>,
    record: Arc<Mutex<EnvRecord>>,
}

#[derive(Debug, Default)]
struct EnvRecord {
    used: BTreeSet<String>,
    denied: BTreeSet<String>,
}

impl EnvGuard {
    pub fn allow_all() -> Self {
        Self::default()
    }

    pub fn allow_only(names: HashSet<String>) -> Self {
        Self {
            allow: Some(Arc::new(names)),
            record: Arc::default(),
        }
    }

    /// Record `name` and tell whether it may be read.
    pub fn check(&self, name: &str) -> bool {
        let allowed = self.allow.as_ref().is_none_or(|a| a.contains(name));
        if let Ok(mut r) = self.record.lock() {
            r.used.insert(name.to_string());
            if !allowed {
                r.denied.insert(name.to_string());
            }
        }
        allowed
    }

    /// Env names asked for so far, sorted.
    pub fn used(&self) -> Vec<String> {
        self.record
            .lock()
            .map(|r| r.used.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Env names denied so far, sorted.
    pub fn denied(&self) -> Vec<String> {
        self.record
            .lock()
            .map(|r| r.denied.iter().cloned().collect())
            .unwrap_or_default()
    }
}

/// Env names in `{{$processEnv NAME}}`-like text (scripts, raw templates).
pub fn scan_process_env(text: &str) -> Vec<String> {
    let mut out = BTreeSet::new();
    let mut rest = text;
    while let Some(i) = rest.find("$processEnv") {
        rest = &rest[i + "$processEnv".len()..];
        let trimmed = rest.trim_start_matches([' ', '\t']);
        if trimmed.len() == rest.len() {
            continue;
        }
        let name: String = trimmed
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if !name.is_empty() {
            out.insert(name);
        }
    }
    out.into_iter().collect()
}

/// Resolve a dynamic variable expression (the part after `$`, already trimmed).
///
/// Returns `None` for unknown names or invalid arguments, so the placeholder stays as-is.
/// Every `$processEnv` name is allowed.
pub fn resolve_dynamic(expr: &str) -> Option<String> {
    resolve_dynamic_with(expr, None)
}

/// Like `resolve_dynamic`, with a `$processEnv` guard. A denied name resolves to "".
pub fn resolve_dynamic_with(expr: &str, env: Option<&EnvGuard>) -> Option<String> {
    let mut parts = expr.split_whitespace();
    let name = parts.next()?;
    let args: Vec<&str> = parts.collect();

    match name {
        "guid" | "uuid" if args.is_empty() => Some(uuid_v4()),
        "timestamp" if args.is_empty() => Some(Utc::now().timestamp().to_string()),
        "datetime" => match args.as_slice() {
            [] | ["iso8601"] => Some(Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)),
            _ => None,
        },
        "randomInt" => match args.as_slice() {
            [min, max] => {
                let min: i64 = min.parse().ok()?;
                let max: i64 = max.parse().ok()?;
                random_int(min, max).map(|n| n.to_string())
            }
            _ => None,
        },
        "processEnv" => match args.as_slice() {
            [var] if env.is_none_or(|g| g.check(var)) => {
                Some(std::env::var(var).unwrap_or_default())
            }
            [_] => Some(String::new()),
            _ => None,
        },
        _ => None,
    }
}

/// Random integer in `[min, max)`. `None` when `max <= min`.
fn random_int(min: i64, max: i64) -> Option<i64> {
    if max <= min {
        return None;
    }
    let range = (max as i128 - min as i128) as u128;
    let offset = (random_u64() as u128) % range;
    Some((min as i128 + offset as i128) as i64)
}

/// Random UUID v4 string (lowercase, hyphenated).
fn uuid_v4() -> String {
    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&random_u64().to_le_bytes());
    bytes[8..].copy_from_slice(&random_u64().to_le_bytes());
    bytes[6] = (bytes[6] & 0x0f) | 0x40; // version 4
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // RFC 4122 variant

    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

fn random_u64() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u64(COUNTER.fetch_add(1, Ordering::Relaxed));
    hasher.write_u128(nanos);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_format() {
        let a = resolve_dynamic("guid").unwrap();
        let b = resolve_dynamic("uuid").unwrap();
        assert_eq!(a.len(), 36);
        assert_eq!(&a[14..15], "4");
        assert!(matches!(&a[19..20], "8" | "9" | "a" | "b"));
        assert_ne!(a, b);
    }

    #[test]
    fn test_timestamp_and_datetime() {
        let ts: i64 = resolve_dynamic("timestamp").unwrap().parse().unwrap();
        assert!(ts > 1_700_000_000);
        let dt = resolve_dynamic("datetime iso8601").unwrap();
        assert!(dt.ends_with('Z') && dt.contains('T'));
        assert_eq!(resolve_dynamic("datetime rfc1123"), None);
    }

    #[test]
    fn test_random_int_range() {
        for _ in 0..200 {
            let n: i64 = resolve_dynamic("randomInt 1 3").unwrap().parse().unwrap();
            assert!((1..3).contains(&n));
        }
        assert_eq!(resolve_dynamic("randomInt 5 5"), None);
        assert_eq!(resolve_dynamic("randomInt a b"), None);
    }

    #[test]
    fn test_process_env_and_unknown() {
        let path = resolve_dynamic("processEnv PATH").unwrap();
        assert!(!path.is_empty());
        assert_eq!(
            resolve_dynamic("processEnv BEEP_SURELY_NOT_SET_123").unwrap(),
            ""
        );
        assert_eq!(resolve_dynamic("nope"), None);
    }

    #[test]
    fn test_process_env_guard() {
        let guard = EnvGuard::allow_only(["PATH".to_string()].into_iter().collect());
        assert!(!resolve_dynamic_with("processEnv PATH", Some(&guard)).unwrap().is_empty());
        assert_eq!(
            resolve_dynamic_with("processEnv PATH_SECRET_X", Some(&guard)).unwrap(),
            ""
        );
        assert_eq!(guard.used(), vec!["PATH", "PATH_SECRET_X"]);
        assert_eq!(guard.denied(), vec!["PATH_SECRET_X"]);
    }

    #[test]
    fn test_scan_process_env() {
        let text = r#"x {{$processEnv HOME}} beep.interpolate("{{ $processEnv  API_KEY }}") $processEnvX"#;
        assert_eq!(scan_process_env(text), vec!["API_KEY", "HOME"]);
    }
}
