use std::{
    io::{self, Read, Write},
    process::{Command, ExitStatus, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

const POLL: Duration = Duration::from_millis(10);
const PIPE_GRACE: Duration = Duration::from_secs(1);

pub struct Run {
    pub status: Option<ExitStatus>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl Run {
    pub fn timed_out(&self) -> bool {
        self.status.is_none()
    }

    pub fn success(&self) -> bool {
        self.status.is_some_and(|s| s.success())
    }
}

fn drain<R: Read + Send + 'static>(mut pipe: R, limit: usize) -> mpsc::Receiver<Vec<u8>> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut kept = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            match pipe.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let room = limit.saturating_sub(kept.len());
                    kept.extend_from_slice(&buf[..n.min(room)]);
                }
            }
        }
        let _ = tx.send(kept);
    });
    rx
}

fn collect(rx: mpsc::Receiver<Vec<u8>>) -> Vec<u8> {
    rx.recv_timeout(PIPE_GRACE).unwrap_or_default()
}

pub fn run(
    cmd: &mut Command,
    input: Option<&[u8]>,
    limit: usize,
    timeout: Option<Duration>,
) -> io::Result<Run> {
    let stdin = if input.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    };
    let mut child = cmd
        .stdin(stdin)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    if let (Some(data), Some(mut pipe)) = (input, child.stdin.take()) {
        let data = data.to_vec();
        thread::spawn(move || {
            let _ = pipe.write_all(&data);
        });
    }
    let out = child.stdout.take().map(|p| drain(p, limit));
    let err = child.stderr.take().map(|p| drain(p, limit));

    let deadline = timeout.map(|t| Instant::now() + t);
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break Some(status);
        }
        if deadline.is_some_and(|at| Instant::now() >= at) {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        thread::sleep(POLL);
    };

    Ok(Run {
        status,
        stdout: out.map(collect).unwrap_or_default(),
        stderr: err.map(collect).unwrap_or_default(),
    })
}
