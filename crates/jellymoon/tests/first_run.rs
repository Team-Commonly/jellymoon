//! What `jellymoon day` and `jellymoon page` do for a person who has no session logs yet, and with the list of folders
//! they switched off in the app (`<home>/skip.txt`, `docs/development/reader.md`). The built binary is run as a black
//! box. The home is a scratch folder and the harness is the made-up program of `support/fake_claude.rs`, alone on `PATH`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

use serde_json::{Value, json};
use support::{A, B, Ran, ZONE, install, said, scratch, write_log};
use tempfile::TempDir;

const SKIP_LINE: &str = "jellymoon: the list of folders you switched off (skip.txt in Jellymoon's folder) could not be \
                         read, so nothing was read: ";

/// One test's logs (a turn in `alpha`, one in `private-notes`), scratch home and harness.
struct World {
    dir: TempDir,
    vars: Vec<(&'static str, OsString)>,
}

fn world() -> World {
    let dir = scratch();
    let at = |name: &str| dir.path().join(name);
    write_log(&at("logs"), "-work-alpha/s1.jsonl", &[said("2026-03-10T16:00:00.000Z", A, "add a retry")]);
    write_log(&at("logs"), "-work-private-notes/s2.jsonl", &[said("2026-03-10T17:00:00.000Z", B, "dear diary")]);
    fs::create_dir_all(at("user").join(".jellymoon")).unwrap();
    install(&at("bin"), "claude");
    let reply = json!({"type": "result", "is_error": false, "result": r#"{"wishes":[],"clashes":[]}"#});
    fs::write(at("bin").join("reply"), reply.to_string()).unwrap();
    fs::write(at("bin").join("exit"), "0").unwrap();
    let vars = vec![
        ("TZ", ZONE.into()),
        ("HOME", at("user").into()),
        ("USERPROFILE", at("user").into()),
        ("PATH", at("bin").into()),
        ("JELLYMOON_SYSTEM_BIN", OsString::new()), // no folder of this machine is looked in for a harness
    ];
    World { dir, vars }
}

impl World {
    fn at(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    /// The list of switched-off folders in the scratch home's own Jellymoon folder.
    fn skip_txt(&self) -> PathBuf {
        self.at("user").join(".jellymoon").join("skip.txt")
    }

    fn run(&self, args: &[&str]) -> Ran {
        let vars: Vec<_> = self.vars.iter().map(|(name, value)| (*name, value.as_os_str())).collect();
        support::jellymoon_in(self.dir.path(), &vars, args)
    }

    /// Runs a command over the day in the world's logs.
    fn over_logs(&self, command: &str, more: &[&str]) -> Ran {
        let logs = self.at("logs");
        self.run(&[&[command, "--date", "2026-03-10", "--logs", logs.to_str().unwrap()], more].concat())
    }

    /// What the harness was sent since the last look: the day's turns as typed. None when it was not called.
    fn sent(&self) -> Option<Vec<String>> {
        let seen = self.at("bin").join("stdin");
        let day = fs::read_to_string(&seen).ok()?;
        fs::remove_file(seen).unwrap();
        Some(typed(&day))
    }
}

/// What the turns of a printed day say.
fn typed(day: &str) -> Vec<String> {
    let day: Value = serde_json::from_str(day).expect(day);
    day["turns"].as_array().unwrap().iter().map(|turn| turn["said"].as_str().unwrap().to_string()).collect()
}

#[test]
fn a_home_with_no_session_logs_gets_one_plain_line_that_names_the_folder_and_no_call() {
    let w = world();
    let looked = w.at("user").join(".claude").join("projects");
    for command in ["day", "page"] {
        let ran = w.run(&[command, "--date", "2026-03-10"]);
        let line = format!(
            "jellymoon: no Claude Code session logs were found: there is no folder {}. Use Claude Code once and they \
             will be there, or name its projects folder with --logs\n",
            looked.display()
        );
        assert_eq!((ran.code, ran.stdout.as_str(), ran.stderr), (1, "", line), "{command}");
        assert_eq!(w.sent(), None, "{command}");
    }
    // A folder that is there and holds no session is a day nobody typed in, as before.
    fs::create_dir_all(&looked).unwrap();
    let ran = w.run(&["day", "--date", "2026-03-10"]);
    assert_eq!((ran.code, typed(&ran.stdout).len(), ran.stderr.as_str()), (0, 0, ""));
}

#[test]
fn a_folder_named_in_skip_txt_is_read_by_neither_command_and_joins_the_ones_named_with_skip() {
    let w = world();
    assert_eq!(typed(&w.over_logs("day", &[]).stdout), ["add a retry", "dear diary"], "with no list, both are read");

    // A byte order mark and Windows line ends, as an editor may leave them.
    fs::write(w.skip_txt(), "\u{feff}private-notes\r\n").unwrap();
    assert_eq!(typed(&w.over_logs("day", &[]).stdout), ["add a retry"]);
    assert_eq!(w.over_logs("page", &[]).code, 0);
    assert_eq!(w.sent(), Some(vec!["add a retry".to_string()]), "the call is sent the day less the folder");
    assert!(typed(&w.over_logs("day", &["--skip", "alpha"]).stdout).is_empty(), "the file's names and the flag's");

    // `JELLYMOON_HOME` moves the list with the rest of Jellymoon's files.
    let mut moved = World { dir: w.dir, vars: w.vars };
    moved.vars.push(("JELLYMOON_HOME", moved.at("jm").into()));
    assert_eq!(typed(&moved.over_logs("day", &[]).stdout).len(), 2, "no list in the new home");
    fs::create_dir(moved.at("jm")).unwrap();
    fs::write(moved.at("jm").join("skip.txt"), "alpha\n").unwrap();
    assert_eq!(typed(&moved.over_logs("day", &[]).stdout), ["dear diary"]);
}

// A list that cannot be read whole is never an empty list: nothing is printed and nothing goes to the call.
#[test]
fn a_skip_txt_that_cannot_be_read_stops_both_commands_with_one_line_and_no_call() {
    let w = world();
    let refused = |why: &str| {
        for command in ["day", "page"] {
            let ran = w.over_logs(command, &[]);
            assert_eq!((ran.code, ran.stdout.as_str()), (1, ""), "{command}: {}", ran.stderr);
            assert!(ran.stderr.starts_with(SKIP_LINE) && ran.stderr.lines().count() == 1, "{command}: {}", ran.stderr);
            assert!(ran.stderr.ends_with(&format!("{why}\n")), "{command}: {}", ran.stderr);
            assert_eq!(w.sent(), None, "{command}");
        }
    };
    fs::write(w.skip_txt(), [b'p', 0xff, 0xfe, b'\n']).unwrap();
    refused("it is not UTF-8 text");
    fs::write(w.skip_txt(), "private-notes\n".repeat(4682)).unwrap(); // 65548 bytes
    refused("it is over 65536 bytes");
    fs::write(w.skip_txt(), "private-notes\n".repeat(4681) + "xy").unwrap(); // 65536 bytes: the most that is read
    assert_eq!(typed(&w.over_logs("day", &[]).stdout), ["add a retry"]);
    // A folder under the list's name cannot be read as a list on any system.
    fs::remove_file(w.skip_txt()).unwrap();
    fs::create_dir(w.skip_txt()).unwrap();
    refused("");
}
