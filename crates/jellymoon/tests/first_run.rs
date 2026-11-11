//! What `jellymoon day` and `jellymoon page` do for a person who has no session logs yet, and with the list of folders
//! they switched off in the app (`<home>/skip.txt`, `docs/development/reader.md`). The built binary is run as a black
//! box. The home is a scratch folder and the harness is the made-up program of `support/fake_claude.rs`, alone on `PATH`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

use serde_json::{Value, json};
use support::{A, B, Ran, ZONE, install, said, scratch, tree, write_log};
use tempfile::TempDir;

const SKIP_LINE: &str = "jellymoon: the list of folders you switched off (skip.txt in Jellymoon's folder) could not be \
                         read, so nothing was read: ";
/// Every single character of core's table of characters that cannot be seen, both ends of each of its ranges, and
/// one control character: the table cannot lose an entry with this green.
const UNSEEN: &str = "\u{7}\u{ad}\u{61c}\u{115f}\u{1160}\u{180e}\u{2028}\u{2029}\u{3164}\u{feff}\u{ffa0}\
                      \u{200b}\u{200f}\u{202a}\u{202e}\u{2060}\u{2065}\u{206f}\u{fe00}\u{fe0f}\
                      \u{e0000}\u{e007f}\u{e0100}\u{e01ef}";
const NO_LOGS: &str = "jellymoon: no Claude Code session logs were found: there is no folder ";

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

    /// Both commands stop on the list as it is now: one line that ends with `why` and quotes no name, and no call.
    fn refused(&self, why: &str) {
        for command in ["day", "page"] {
            let ran = self.over_logs(command, &[]);
            assert_eq!((ran.code, ran.stdout.as_str()), (1, ""), "{command}: {}", ran.stderr);
            assert!(ran.stderr.starts_with(SKIP_LINE) && ran.stderr.lines().count() == 1, "{command}: {}", ran.stderr);
            assert!(ran.stderr.ends_with(&format!("{why}\n")), "{command}, {why}: {}", ran.stderr);
            assert!(!ran.stderr.contains("private"), "{command}: a line is named by its number: {}", ran.stderr);
            assert_eq!(self.sent(), None, "{command}");
        }
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
    let advice = ". Use Claude Code once and they will be there, or name its projects folder with --logs";
    for command in ["day", "page"] {
        let ran = w.run(&[command, "--date", "2026-03-10"]);
        let line = format!("{NO_LOGS}{}{advice}\n", looked.display());
        assert_eq!((ran.code, ran.stdout.as_str(), ran.stderr), (1, "", line), "{command}");
        // A folder the person named gets no advice to name one.
        let nowhere = w.at("nowhere");
        let ran = w.run(&[command, "--logs", nowhere.to_str().unwrap()]);
        assert_eq!((ran.code, ran.stderr), (1, format!("{NO_LOGS}{}\n", nowhere.display())), "{command}");
        // No folder at all is not a folder that is missing, and a name with a line break in it is still one line.
        for logs in ["", "   "] {
            let ran = w.run(&[command, "--logs", logs]);
            assert!(ran.code == 2 && ran.stderr.starts_with("jellymoon: --logs needs a folder\n"), "{}", ran.stderr);
        }
        let ran = w.run(&[command, "--logs", "no\nwhere"]);
        assert_eq!((ran.code, ran.stdout.as_str(), ran.stderr.lines().count()), (1, "", 1), "{}", ran.stderr);
        // A character that cannot be seen, or that turns the line around, is a space in the line.
        let ran = w.run(&[command, "--logs", "no\u{202e}wh\u{200b}ere"]);
        assert_eq!((ran.code, ran.stderr), (1, format!("{NO_LOGS}no wh ere\n")), "{command}");
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

    // A separator at a name's end is dropped, in the file and in the flag. The match is exact, so case matters.
    fs::write(w.skip_txt(), "  private-notes/ \n").unwrap();
    assert_eq!(typed(&w.over_logs("day", &["--skip", r"alpha\"]).stdout), [""; 0]);
    fs::write(w.skip_txt(), "Private-Notes\n").unwrap();
    assert_eq!(typed(&w.over_logs("day", &[]).stdout).len(), 2, "another case is another name");
    // A name of separators alone is kept as written: it switches off every folder written with it, as it always did.
    for name in ["/", "//", " / "] {
        fs::write(w.skip_txt(), format!("{name}\n")).unwrap();
        let none = name.trim() == "/";
        assert_eq!(typed(&w.over_logs("day", &[]).stdout).is_empty(), none, "{name:?} in the list");
        assert_eq!(w.over_logs("page", &[]).code, 0);
        assert_eq!(w.sent().is_none(), none, "{name:?}: a day with no turn makes no call");
        fs::remove_file(w.skip_txt()).unwrap();
        assert_eq!(typed(&w.over_logs("day", &["--skip", name]).stdout).is_empty(), none, "--skip {name:?}");
    }
    assert_eq!(typed(&w.over_logs("day", &["--skip", r"\"]).stdout).len(), 2, "no folder here is written with it");

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
fn a_skip_txt_that_is_not_a_whole_plain_list_stops_both_commands_with_one_line_and_no_call() {
    let w = world();
    fs::write(w.skip_txt(), [b'p', 0xff, 0xfe, b'\n']).unwrap();
    w.refused("it is not UTF-8 text");
    fs::write(w.skip_txt(), "private-notes\n".repeat(4682)).unwrap(); // 65548 bytes
    w.refused("it is over 65536 bytes");
    fs::write(w.skip_txt(), "private-notes\n".repeat(4681) + "xyz").unwrap(); // 65537 bytes: one over
    w.refused("it is over 65536 bytes");
    fs::write(w.skip_txt(), "private-notes\n".repeat(4681) + "xy").unwrap(); // 65536 bytes: the most that is read
    assert_eq!(typed(&w.over_logs("day", &[]).stdout), ["add a retry"]);

    // Characters no folder name holds say the file is not the list it looks like: the line's number, never its text.
    fs::write(w.skip_txt(), "alpha\nprivate\0notes\n").unwrap();
    w.refused("line 2 holds a NUL character");
    let utf16: Vec<u8> = "private-notes\n".encode_utf16().flat_map(u16::to_le_bytes).collect();
    fs::write(w.skip_txt(), utf16).unwrap();
    w.refused("line 1 holds a NUL character");
    fs::write(w.skip_txt(), "alpha\n\u{feff}private-notes\n").unwrap();
    w.refused("line 2 holds a byte order mark");
    fs::write(w.skip_txt(), "\u{feff}\u{feff}private-notes\n").unwrap();
    w.refused("line 1 holds a byte order mark");
    fs::write(w.skip_txt(), "private-notes\ralpha\r").unwrap(); // old Mac line ends: one line to every reader since
    w.refused("line 1 holds a carriage return that does not end it");
    // A character nobody can see would make a name that matches nothing and looks right: in the list it stops the
    // run, in `--skip` it is an argument error that names the name by its place.
    for unseen in UNSEEN.chars() {
        fs::write(w.skip_txt(), format!("alpha\n\nprivate-{unseen}notes\n")).unwrap();
        let what = if unseen == '\u{feff}' { "a byte order mark" } else { "a character that cannot be seen" };
        w.refused(&format!("line 3 holds {what}"));
        fs::remove_file(w.skip_txt()).unwrap();
        let ran = w.over_logs("day", &["--skip", &format!("alpha, private-{unseen}notes")]);
        let line = "jellymoon: --skip: name 2 holds a character that cannot be seen\nusage:";
        assert!(ran.code == 2 && ran.stdout.is_empty() && ran.stderr.starts_with(line), "{unseen:?}: {}", ran.stderr);
        assert!(!ran.stderr.contains("private"), "{unseen:?}: {}", ran.stderr);
    }
    assert_eq!(w.over_logs("page", &["--skip", "al\u{200b}pha"]).code, 2);
    assert_eq!(w.sent(), None);
    // White space around a name is dropped before the look, in both, so what an editor or a shell leaves is honoured.
    fs::write(w.skip_txt(), "private-notes\t\u{c}\n").unwrap();
    assert_eq!(typed(&w.over_logs("day", &[]).stdout), ["add a retry"]);
    assert_eq!(typed(&w.over_logs("day", &["--skip", "\talpha\u{2028}\n"]).stdout), [""; 0]);

    // A folder under the list's name cannot be read as a list on any system.
    fs::remove_file(w.skip_txt()).unwrap();
    fs::create_dir(w.skip_txt()).unwrap();
    w.refused("it is not a file");
    // The list is looked at before the logs, so a damaged one is what is said even when there are no logs.
    let ran = w.run(&["day", "--logs", w.at("nowhere").to_str().unwrap()]);
    assert!(ran.code == 1 && ran.stderr.starts_with(SKIP_LINE), "{}", ran.stderr);
}

// Only nothing at all under the name is no list. A pipe is told by its type and never opened: opening one would wait.
#[cfg(unix)]
#[test]
fn a_link_to_nothing_a_pipe_a_closed_file_and_a_home_that_is_a_file_each_stop_both_commands() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let mut w = world();
    symlink(w.at("gone"), w.skip_txt()).unwrap();
    w.refused("(os error 2)");
    fs::write(w.at("gone"), "private-notes\n").unwrap();
    assert_eq!(typed(&w.over_logs("day", &[]).stdout), ["add a retry"], "a link to a list is the list");

    fs::remove_file(w.skip_txt()).unwrap();
    assert!(std::process::Command::new("mkfifo").arg(w.skip_txt()).status().unwrap().success());
    w.refused("it is not a file");

    fs::remove_file(w.skip_txt()).unwrap();
    fs::write(w.skip_txt(), "private-notes\n").unwrap();
    fs::set_permissions(w.skip_txt(), fs::Permissions::from_mode(0o000)).unwrap();
    if fs::File::open(w.skip_txt()).is_err() {
        w.refused("(os error 13)"); // not for a user that no mode stops
    }

    fs::write(w.at("file"), "not a folder").unwrap();
    w.vars.push(("JELLYMOON_HOME", w.at("file").into()));
    w.refused("(os error 20)");
}

// A relative home would name another list, other rulings and another hook record in every folder a command starts in.
#[test]
fn a_jellymoon_home_that_is_not_a_full_path_is_refused_by_every_command_that_would_use_it() {
    let mut w = world();
    fs::create_dir(w.at("rel")).unwrap();
    fs::write(w.at("rel").join("skip.txt"), "private-notes\n").unwrap();
    fs::write(w.at("settings.json"), "{}\n").unwrap();
    w.vars.push(("JELLYMOON_HOME", "rel".into()));
    w.vars.push(("JELLYMOON_APP_DIR", w.at("apps").into()));
    w.vars.push(("JELLYMOON_BIN_DIR", w.at("installed").into()));
    let (logs, settings) = (w.at("logs"), w.at("settings.json"));
    let (logs, settings) = (logs.to_str().unwrap(), settings.to_str().unwrap());
    let before = tree(w.dir.path());
    let commands: [&[&str]; 6] = [
        &["day", "--date", "2026-03-10", "--logs", logs],
        &["page", "--date", "2026-03-10", "--logs", logs],
        &["rule", "--stands", "fail fast", "--over", "add a retry"],
        &["hook", "install", "--settings", settings],
        &["hook", "uninstall"],
        &["uninstall"],
    ];
    for args in commands {
        let ran = w.run(args);
        let line = "jellymoon: no home directory: set JELLYMOON_HOME to an absolute path\n";
        assert_eq!((ran.code, ran.stdout.as_str(), ran.stderr.as_str()), (1, "", line), "{args:?}");
        assert!(tree(w.dir.path()) == before, "{args:?}: nothing was written and no call was made");
    }
}

// The hook is the one command that says nothing: an error from it would show at every step of a session.
#[test]
fn the_hook_with_a_relative_home_says_nothing_exits_0_and_hands_over_nothing() {
    let w = world();
    let (store, rel) = (w.at("rel"), std::ffi::OsStr::new("rel"));
    let ruled = support::jellymoon(
        &[("JELLYMOON_HOME", store.as_os_str())],
        &["rule", "--stands", "fail fast", "--over", "add a retry"],
    );
    assert_eq!((ruled.code, ruled.stderr.as_str()), (0, ""));
    let step = json!({"session_id": "s-1", "transcript_path": "/made/up.jsonl", "cwd": "/work/alpha",
        "hook_event_name": "PostToolUse", "tool_name": "Read", "tool_input": {}, "tool_response": {}})
    .to_string();
    let before = tree(w.dir.path());
    // Started where `rel` would lead to the store with the waiting ruling.
    let ran = support::started(w.dir.path(), &[("JELLYMOON_HOME", rel)], &["hook"], &step);
    assert_eq!((ran.code, ran.stdout.as_str(), ran.stderr.as_str()), (0, "", ""));
    assert!(tree(w.dir.path()) == before, "no mark was made: the ruling still waits");
    let told = support::started(w.dir.path(), &[("JELLYMOON_HOME", store.as_os_str())], &["hook"], &step);
    assert!(told.code == 0 && told.stdout.contains("fail fast"), "the same step with the full path is told");
}
