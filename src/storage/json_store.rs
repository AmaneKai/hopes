use crate::models::Item;
use std::{
    fs::{self, File},
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
};

pub struct JsonStore {
    path: PathBuf,
}

impl JsonStore {
    pub fn default_path() -> Self {
        let mut dir = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
        dir.push("hopes");
        Self::at_dir(dir)
    }

    pub fn at_dir(dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&dir);
        Self {
            path: dir.join("items.json"),
        }
    }

    #[inline(always)]
    pub fn data_dir(&self) -> &Path {
        self.path.parent().unwrap_or(Path::new("."))
    }

    pub fn load(&self) -> io::Result<Vec<Item>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let file = File::open(&self.path)?;
        let items: Vec<Item> = serde_json::from_reader(io::BufReader::new(file))
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(items)
    }

    pub fn save(&self, items: &[Item]) -> io::Result<()> {
        let tmp_path = self.path.with_extension("tmp");
        {
            let file = File::create(&tmp_path)?;
            let mut writer = BufWriter::new(file);
            serde_json::to_writer_pretty(&mut writer, items)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            writer.flush()?;
        }
        fs::rename(tmp_path, &self.path)?;
        Ok(())
    }
}
