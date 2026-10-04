use super::Storage;
use crate::models::Item;
use std::{
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

/// Delegates persistence to any external program, in any language.
///
/// `<command> load` must print a JSON array of items to stdout (nothing at all means
/// "no items yet"). `<command> save` receives that same JSON array on stdin. A non-zero
/// exit code is treated as failure.
pub struct ExecStore {
    dir: PathBuf,
    program: String,
    args: Vec<String>,
}

impl ExecStore {
    pub fn new(dir: PathBuf, command: Vec<String>) -> io::Result<Self> {
        let mut parts = command.into_iter();
        let program = parts.next().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "storage.command is required for the exec backend",
            )
        })?;
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::models::Item;

    fn script_store(script: &str) -> ExecStore {
        ExecStore::new(
            std::env::temp_dir(),
            vec!["sh".into(), "-c".into(), script.into(), "hopes".into()],
        )
        .unwrap()
    }

    #[test]
    fn round_trips_through_an_external_program() {
        let file = std::env::temp_dir().join(format!("hopes-exec-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&file);
        let path = file.display();
        let store = script_store(&format!(
            "case $1 in load) cat {path} 2>/dev/null || true;; save) cat > {path};; esac"
        ));

        assert!(store.load().unwrap().is_empty());
        let items = vec![Item::new("a".into(), vec!["x".into()])];
        store.save(&items).unwrap();
        assert_eq!(store.load().unwrap(), items);
        let _ = std::fs::remove_file(&file);
    }

    #[test]
    fn nonzero_exit_is_an_error() {
        let store = script_store("echo boom >&2; exit 3");
        let err = store.load().unwrap_err().to_string();
        assert!(err.contains("boom"), "{err}");
        assert!(store.save(&[]).is_err());
    }

    #[test]
    fn missing_command_is_rejected() {
        assert!(ExecStore::new(std::env::temp_dir(), vec![]).is_err());
    }
}
