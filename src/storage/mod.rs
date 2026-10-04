pub mod exec_store;
pub mod json_store;
#[cfg(feature = "sqlite")]
pub mod sqlite_store;

use crate::{
    config::{Backend, StorageConfig},
    models::Item,
};
use std::{
    io,
    path::{Path, PathBuf},
};

pub use exec_store::ExecStore;
pub use json_store::JsonStore;
#[cfg(feature = "sqlite")]
pub use sqlite_store::SqliteStore;

/// Where tasks are persisted. Implement this to add a backend.
///
/// `save` always receives the full item list, in display order, and must make the
/// backend match it.
pub trait Storage {
    fn load(&self) -> io::Result<Vec<Item>>;
    fn save(&self, items: &[Item]) -> io::Result<()>;
    /// Local directory this backend works in (used by the git sync engine).
    fn data_dir(&self) -> &Path;
}

pub fn default_dir() -> PathBuf {
    let mut dir = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    dir.push("hopes");
    dir
}

pub fn open(cfg: &StorageConfig, fallback_dir: Option<&PathBuf>) -> io::Result<Box<dyn Storage>> {
    let dir = cfg
        .dir
        .clone()
        .or_else(|| fallback_dir.cloned())
        .unwrap_or_else(default_dir);
    Ok(match cfg.backend {
        Backend::Json => Box::new(JsonStore::at_dir(dir)),
        Backend::Exec => Box::new(ExecStore::new(dir, cfg.command.clone())?),
        #[cfg(feature = "sqlite")]
        Backend::Sqlite => Box::new(SqliteStore::at_dir(dir)?),
        #[cfg(not(feature = "sqlite"))]
        Backend::Sqlite => {
            return Err(io::Error::other(
                "sqlite backend not compiled in; rebuild with `--features sqlite`",
            ));
        }
    })
}
