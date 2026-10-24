use crate::utils::mini_toml::{self, Table, Value};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct SyncConfig {
    pub enabled: bool,
    pub debounce_seconds: u64,
    pub data_dir: Option<PathBuf>,
    pub remote: String,
    pub branch: String,
    pub pull_on_startup: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            debounce_seconds: 5,
            data_dir: None,
            remote: "origin".to_string(),
            branch: "main".to_string(),
            pull_on_startup: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Backend {
    #[default]
    Json,
    Sqlite,
    Exec,
}

#[derive(Clone, Debug, Default)]
pub struct StorageConfig {
    pub backend: Backend,
    pub dir: Option<PathBuf>,
    pub command: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Config {
    pub sync: SyncConfig,
    pub storage: StorageConfig,
}

impl Config {
    pub fn load() -> Self {
        let Some(mut path) = dirs::config_dir() else {
            return Self::default();
        };
        path.push("hopes");
        path.push("config.toml");
        let Ok(contents) = std::fs::read_to_string(&path) else {
            return Self::default();
        };
        Self::from_toml(&contents).unwrap_or_default()
    }

    pub fn from_toml(text: &str) -> Option<Self> {
        let root = mini_toml::parse(text).ok()?;
        let mut config = Self::default();
        if let Some(sync) = section(&root, "sync")? {
            let c = &mut config.sync;
            read(sync, "enabled", as_bool, &mut c.enabled)?;
            read(sync, "debounce_seconds", as_u64, &mut c.debounce_seconds)?;
            read(sync, "remote", as_string, &mut c.remote)?;
            read(sync, "branch", as_string, &mut c.branch)?;
            read(sync, "pull_on_startup", as_bool, &mut c.pull_on_startup)?;
            if let Some(dir) = sync.get("data_dir") {
                c.data_dir = Some(PathBuf::from(as_string(dir)?));
            }
        }
        if let Some(storage) = section(&root, "storage")? {
            let c = &mut config.storage;
            read(storage, "backend", as_backend, &mut c.backend)?;
            read(storage, "command", as_strings, &mut c.command)?;
            if let Some(dir) = storage.get("dir") {
                c.dir = Some(PathBuf::from(as_string(dir)?));
            }
        }
        Some(config)
    }
}

fn section<'a>(root: &'a Table, name: &str) -> Option<Option<&'a Table>> {
    match root.get(name) {
        None => Some(None),
        Some(Value::Table(t)) => Some(Some(t)),
        Some(_) => None,
    }
}

fn read<T>(table: &Table, key: &str, conv: fn(&Value) -> Option<T>, slot: &mut T) -> Option<()> {
    if let Some(value) = table.get(key) {
        *slot = conv(value)?;
    }
    Some(())
}

fn as_bool(v: &Value) -> Option<bool> {
    match v {
        Value::Bool(b) => Some(*b),
        _ => None,
    }
}

fn as_u64(v: &Value) -> Option<u64> {
    match v {
        Value::Int(n) => u64::try_from(*n).ok(),
        _ => None,
    }
}

fn as_string(v: &Value) -> Option<String> {
    match v {
        Value::Str(s) => Some(s.clone()),
        _ => None,
    }
}

fn as_strings(v: &Value) -> Option<Vec<String>> {
    match v {
        Value::Array(items) => {
            let mut strings = Vec::new();
            for item in items {
                strings.push(as_string(item)?);
            }
            Some(strings)
        }
        _ => None,
    }
}

fn as_backend(v: &Value) -> Option<Backend> {
    match as_string(v)?.as_str() {
        "json" => Some(Backend::Json),
        "sqlite" => Some(Backend::Sqlite),
        "exec" => Some(Backend::Exec),
        _ => None,
    }
}
