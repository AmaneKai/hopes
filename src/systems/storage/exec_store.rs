use super::Storage;
use crate::domain::Item;
use std::{
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

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

fn failed(verb: &str, status: std::process::ExitStatus, stderr: &[u8]) -> io::Error {
    io::Error::other(format!(
        "storage command `{verb}` failed ({status}): {}",
        String::from_utf8_lossy(stderr).trim()
    ))
}

impl Storage for ExecStore {
    fn data_dir(&self) -> &Path {
        &self.dir
    }

    fn load(&self) -> io::Result<Vec<Item>> {
        let out = self.command("load").stdin(Stdio::null()).output()?;
        if !out.status.success() {
            return Err(failed("load", out.status, &out.stderr));
        }
        if out.stdout.iter().all(u8::is_ascii_whitespace) {
            return Ok(Vec::new());
        }
        serde_json::from_slice(&out.stdout)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    fn save(&self, items: &[Item]) -> io::Result<()> {
        let payload =
            serde_json::to_vec(items).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let mut child = self
            .command("save")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(&payload)?;
        }
        let out = child.wait_with_output()?;
        if out.status.success() {
            Ok(())
        } else {
            Err(failed("save", out.status, &out.stderr))
        }
    }
}
