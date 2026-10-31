pub mod exec_store;
pub mod json_store;
#[cfg(feature = "sqlite")]
pub mod sqlite_store;

use crate::{
    config::{Backend, StorageConfig},
    domain::Item,
};
use std::{
    io,
    path::{Path, PathBuf},
};

pub use exec_store::ExecStore;
pub use json_store::JsonStore;
#[cfg(feature = "sqlite")]
pub use sqlite_store::SqliteStore;

pub trait Storage {
    fn load(&self) -> io::Result<Vec<Item>>;
    fn save(&self, items: &[Item]) -> io::Result<()>;
    fn data_dir(&self) -> &Path;
}

pub fn default_dir() -> PathBuf {
    let mut dir = match dirs::data_local_dir() {
        Some(dir) => dir,
        None => PathBuf::from("."),
    };
    dir.push("hopes");
    dir
}

pub fn open(cfg: &StorageConfig, fallback_dir: Option<&PathBuf>) -> io::Result<Box<dyn Storage>> {
    let dir = if let Some(dir) = &cfg.dir {
        dir.clone()
    } else if let Some(dir) = fallback_dir {
        dir.clone()
    } else {
        default_dir()
    };
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
