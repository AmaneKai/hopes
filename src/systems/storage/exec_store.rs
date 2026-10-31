use super::Storage;
use crate::{
    domain::Item,
    utils::{proc, text},
};
use std::{
    io,
    path::{Path, PathBuf},
    process::Command,
};

const OUTPUT_LIMIT: usize = 64 << 20;
const ERROR_LIMIT: usize = 160;

pub struct ExecStore {
    dir: PathBuf,
    program: String,
    args: Vec<String>,
}

impl ExecStore {
    pub fn new(dir: PathBuf, command: Vec<String>) -> io::Result<Self> {
        let mut parts = command.into_iter();
        let Some(program) = parts.next() else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "storage.command is required for the exec backend",
            ));
        };
        Ok(Self {
            dir,
            program,
            args: parts.collect(),
        })
    }

    fn command(&self, verb: &str) -> Command {
        let mut cmd = Command::new(&self.program);
        cmd.args(&self.args).arg(verb);
        cmd
    }
}

fn failed(verb: &str, run: &proc::Run) -> io::Error {
    let stderr = String::from_utf8_lossy(&run.stderr);
    let detail: String = stderr.trim().chars().take(ERROR_LIMIT).collect();
    match run.status {
        Some(status) => io::Error::other(format!(
            "storage command `{verb}` failed ({status}): {detail}"
        )),
        None => io::Error::new(
            io::ErrorKind::TimedOut,
            format!("storage command `{verb}` timed out"),
        ),
    }
}

impl Storage for ExecStore {
    fn data_dir(&self) -> &Path {
        &self.dir
    }

    fn load(&self) -> io::Result<Vec<Item>> {
        let run = proc::run(&mut self.command("load"), None, OUTPUT_LIMIT, None)?;
        if !run.success() {
            return Err(failed("load", &run));
        }
        let text = text::normalize(&run.stdout);
        if text.trim().is_empty() {
            return Ok(Vec::new());
        }
        serde_json::from_str(&text).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    fn save(&self, items: &[Item]) -> io::Result<()> {
        let payload =
            serde_json::to_vec(items).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let run = proc::run(
            &mut self.command("save"),
            Some(&payload),
            OUTPUT_LIMIT,
            None,
        )?;
        if run.success() {
            Ok(())
        } else {
            Err(failed("save", &run))
        }
    }
}
