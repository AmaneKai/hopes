use super::Storage;
use crate::domain::Item;
use rusqlite::{Connection, params};
use std::{
    io,
    path::{Path, PathBuf},
};

pub struct SqliteStore {
    conn: Connection,
    dir: PathBuf,
}

fn db_err(e: impl std::error::Error + Send + Sync + 'static) -> io::Error {
    io::Error::other(e)
}

impl SqliteStore {
    pub fn at_dir(dir: PathBuf) -> io::Result<Self> {
        std::fs::create_dir_all(&dir)?;
        let conn = Connection::open(dir.join("hopes.db")).map_err(db_err)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS items (
                position INTEGER PRIMARY KEY,
                data     TEXT NOT NULL
            );",
        )
        .map_err(db_err)?;
        Ok(Self { conn, dir })
    }
}

impl Storage for SqliteStore {
    fn data_dir(&self) -> &Path {
        &self.dir
    }

    fn load(&self) -> io::Result<Vec<Item>> {
        let mut stmt = self
            .conn
            .prepare("SELECT data FROM items ORDER BY position")
            .map_err(db_err)?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(db_err)?;
        rows.map(|row| {
            let data = row.map_err(db_err)?;
            serde_json::from_str(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        })
        .collect()
    }

    fn save(&self, items: &[Item]) -> io::Result<()> {
        let tx = self.conn.unchecked_transaction().map_err(db_err)?;
        tx.execute("DELETE FROM items", []).map_err(db_err)?;
        {
            let mut ins = tx
                .prepare("INSERT INTO items (position, data) VALUES (?1, ?2)")
                .map_err(db_err)?;
            for (pos, item) in items.iter().enumerate() {
                let data = serde_json::to_string(item).map_err(db_err)?;
                ins.execute(params![pos as i64, data]).map_err(db_err)?;
            }
        }
        tx.commit().map_err(db_err)
    }
}
