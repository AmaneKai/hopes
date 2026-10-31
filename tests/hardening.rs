use hopes::{
    config::Config,
    systems::storage::{JsonStore, Storage},
    utils::text::normalize,
};

fn utf16(bom: [u8; 2], units: &[u16], big: bool) -> Vec<u8> {
    let mut out = bom.to_vec();
    for u in units {
        out.extend(if big {
            u.to_be_bytes()
        } else {
            u.to_le_bytes()
        });
    }
    out
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("hopes-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn normalize_handles_every_encoding() {
    assert_eq!(normalize(b"\xEF\xBB\xBF[]"), "[]");
    assert_eq!(normalize(b"[]"), "[]");
    assert_eq!(normalize(&utf16([0xFE, 0xFF], &[0x5B, 0x5D], true)), "[]");
    assert_eq!(
        normalize(&utf16(
            [0xFF, 0xFE],
            &[0x5B, 0x22, 0xD83D, 0xDE00, 0x22, 0x5D],
            false
        )),
        "[\"\u{1F600}\"]"
    );
    assert_eq!(
        normalize(&utf16([0xFF, 0xFE], &[0xD800, 0x78], false)),
        "\u{FFFD}x"
    );
    assert_eq!(normalize(b"\xFF\xFEa\0b"), "a");
    assert_eq!(normalize(b"\xFF\xFE"), "");
    assert_eq!(normalize(b"\xEF\xBB\xBF"), "");
    assert_eq!(normalize(b""), "");
}

#[test]
fn json_store_reads_files_saved_by_other_editors() {
    let dir = scratch("editors");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("items.json");

    std::fs::write(&path, utf16([0xFF, 0xFE], &[0x5B, 0x5D, 0x0D, 0x0A], false)).unwrap();
    assert!(JsonStore::at_dir(dir.clone()).load().unwrap().is_empty());

    std::fs::write(
        &path,
        b"\xEF\xBB\xBF[{\"title\":\"t\",\"priority\":\"Low\",\"status\":\"Todo\"}]",
    )
    .unwrap();
    assert_eq!(JsonStore::at_dir(dir.clone()).load().unwrap().len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn config_tolerates_bom_and_crlf() {
    let text = normalize(b"\xEF\xBB\xBF[sync]\r\nenabled = false\r\n");
    let cfg = Config::from_toml(&text).unwrap();
    assert!(!cfg.sync.enabled);
}

#[test]
fn json_store_creates_nested_directories_and_saves_atomically() {
    let dir = scratch("nested").join("one").join("two").join("three");
    let store = JsonStore::at_dir(dir.clone());
    assert!(dir.is_dir());
    JsonStore::at_dir(dir.clone());
    store.save(&[]).unwrap();
    assert!(dir.join("items.json").is_file());
    assert!(!dir.join("items.tmp").exists());
    let _ = std::fs::remove_dir_all(dir.ancestors().nth(3).unwrap());
}

#[cfg(unix)]
mod processes {
    use hopes::utils::proc::run;
    use std::{
        process::Command,
        time::{Duration, Instant},
    };

    fn sh(script: &str) -> Command {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", script, "hopes"]);
        cmd
    }

    #[test]
    fn arguments_survive_untouched() {
        let args = ["with space", "quote\"inside", "trailing\\", "", "雨音 カイ"];
        let r = run(
            Command::new("printf").arg("[%s]\n").args(args),
            None,
            1 << 16,
            None,
        )
        .unwrap();
        assert_eq!(
            String::from_utf8(r.stdout).unwrap(),
            "[with space]\n[quote\"inside]\n[trailing\\]\n[]\n[雨音 カイ]\n"
        );
    }

    #[test]
    fn large_payload_round_trips_without_deadlock() {
        let data = "0123456789abcdef".repeat(200_000);
        let r = run(
            &mut Command::new("cat"),
            Some(data.as_bytes()),
            1 << 24,
            Some(Duration::from_secs(20)),
        )
        .unwrap();
        assert!(r.success());
        assert_eq!(r.stdout.len(), data.len());
        assert_eq!(r.stdout, data.as_bytes());
    }

    #[test]
    fn chatty_child_cannot_stall_on_a_full_pipe() {
        let r = run(
            &mut sh("head -c 3000000 /dev/zero; head -c 3000000 /dev/zero >&2"),
            None,
            1000,
            Some(Duration::from_secs(20)),
        )
        .unwrap();
        assert!(r.success());
        assert_eq!(r.stdout.len(), 1000);
        assert_eq!(r.stderr.len(), 1000);
    }

    #[test]
    fn output_is_capped() {
        let data = vec![b'x'; 100_000];
        let r = run(&mut Command::new("cat"), Some(&data), 1000, None).unwrap();
        assert!(r.success());
        assert_eq!(r.stdout.len(), 1000);
    }

    #[test]
    fn hung_child_is_killed_at_the_deadline() {
        let started = Instant::now();
        let r = run(
            &mut sh("sleep 30"),
            None,
            1000,
            Some(Duration::from_millis(400)),
        )
        .unwrap();
        assert!(r.timed_out());
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[test]
    fn failure_reports_status_and_stderr() {
        let r = run(
            &mut sh("echo boom from helper >&2; exit 3"),
            None,
            1000,
            None,
        )
        .unwrap();
        assert_eq!(r.status.and_then(|s| s.code()), Some(3));
        assert!(String::from_utf8_lossy(&r.stderr).contains("boom from helper"));
    }

    #[test]
    fn missing_program_is_an_error() {
        assert!(
            run(
                &mut Command::new("hopes-no-such-program-xyz"),
                None,
                1000,
                None
            )
            .is_err()
        );
    }
}
