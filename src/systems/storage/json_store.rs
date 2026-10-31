use super::Storage;
use crate::{domain::Item, utils::text};
use std::{
    fs::{self, File},
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
};

pub struct JsonStore {
    path: PathBuf,
}

impl JsonStore {
    pub fn at_dir(dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&dir);
        Self {
            path: dir.join("items.json"),
        }
    }
}

impl Storage for JsonStore {
    #[inline(always)]
    fn data_dir(&self) -> &Path {
        self.path.parent().unwrap_or(Path::new("."))
    }

    fn load(&self) -> io::Result<Vec<Item>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let bytes = fs::read(&self.path)?;
        serde_json::from_str(&text::normalize(&bytes))
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    fn save(&self, items: &[Item]) -> io::Result<()> {
        let tmp_path = self.path.with_extension("tmp");
        {
            let file = File::create(&tmp_path)?;
            let mut writer = BufWriter::new(file);
            serde_json::to_writer_pretty(&mut writer, items)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            writer.flush()?;
            writer.get_ref().sync_all()?;
        }
        fs::rename(tmp_path, &self.path)?;
        Ok(())
    }
}
