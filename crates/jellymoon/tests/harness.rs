//! Where `jellymoon page` looks for the harness when `PATH` does not have it, as in an app opened from Finder
//! (`docs/development/page.md`, "Where the harness is looked for"). The built binary is run as a black box. `PATH` is an
//! empty folder, the home and the one system folder are made here, and the harness is the made-up program of
//! `support/fake_claude.rs`, so no folder of this machine is looked in and the real Claude Code is never run.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

use serde_json::json;
use support::{A, Ran, ZONE, install, said, scratch};
use tempfile::TempDir;

/// One test's logs, home, empty `PATH` folder and system folder.
struct World {
    dir: TempDir,
    vars: Vec<(&'static str, OsString)>,
}

fn world() -> World {
    let dir = scratch();
    let at = |name: &str| dir.path().join(name);
    support::write_log(&at("logs"), "-work-alpha/s1.jsonl", &[said("2026-03-10T16:00:00.000Z", A, "add a retry")]);
    for folder in ["user", "path", "sys"] {
        fs::create_dir(at(folder)).unwrap();
    }
    let vars = vec![
        ("TZ", ZONE.into()),
        ("HOME", at("user").into()),
        ("USERPROFILE", at("user").into()),
        ("JELLYMOON_HOME", at("jm").into()),
        ("PATH", at("path").into()),
        ("JELLYMOON_SYSTEM_BIN", at("sys").into()),
    ];
    World { dir, vars }
}

impl World {
    /// The folder of a place: the native installer's, the older local install's, the system's, or `PATH`'s.
    fn place(&self, place: &str) -> PathBuf {
        let parts: &[&str] = match place {
            "native" => &["user", ".local", "bin"],
            "local" => &["user", ".claude", "local"],
            other => &[other],
        };
        parts.iter().fold(self.dir.path().to_path_buf(), |at, part| at.join(part))
    }

    /// Puts a harness that answers with an empty reading in a place.
    fn harness_at(&self, place: &str) -> PathBuf {
        let folder = self.place(place);
        let program = install(&folder, "claude");
        let reply = json!({"type": "result", "is_error": false, "result": r#"{"wishes":[],"clashes":[]}"#});
        fs::write(folder.join("reply"), reply.to_string()).unwrap();
        fs::write(folder.join("exit"), "0").unwrap();
        program
    }

    fn run(&self, command: &str) -> Ran {
        let vars: Vec<_> = self.vars.iter().map(|(name, value)| (*name, value.as_os_str())).collect();
        let logs = self.dir.path().join("logs");
        support::jellymoon(&vars, &[command, "--logs", logs.to_str().unwrap(), "--date", "2026-03-10"])
    }

    /// Runs `jellymoon page`, which must print a page, and says which place's harness was the one run.
    fn ran_at(&self) -> Vec<&'static str> {
        let ran = self.run("page");
        assert_eq!((ran.code, ran.stderr.as_str()), (0, ""), "jellymoon page");
        let mut places = vec!["path", "native", "local", "sys"];
        places.retain(|place| self.place(place).join("arg0").exists());
        places
    }
}

/// A file that is there under the harness's name with this mode, and is never run.
#[cfg(unix)]
fn with_mode(folder: &std::path::Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    fs::create_dir_all(folder).unwrap();
    fs::copy(env!("CARGO_BIN_EXE_fake-claude"), folder.join("claude")).unwrap();
    fs::set_permissions(folder.join("claude"), fs::Permissions::from_mode(mode)).unwrap();
}

#[test]
fn a_harness_in_each_well_known_place_is_found_when_path_has_none() {
    for place in ["native", "local", "sys"] {
        let w = world();
        w.harness_at(place);
        assert_eq!(w.ran_at(), [place]);
    }
}

#[test]
fn path_comes_first_then_the_home_then_the_system() {
    let w = world();
    for place in ["sys", "local"] {
        w.harness_at(place);
    }
    assert_eq!(w.ran_at(), ["local"]);
    w.harness_at("native");
    assert_eq!(w.ran_at(), ["native", "local"], "local ran before; native runs now");
    w.harness_at("path");
    assert_eq!(w.ran_at().first(), Some(&"path"));
    assert!(!w.place("sys").join("arg0").exists(), "the last place was never reached");
}

#[test]
fn the_call_is_the_same_wherever_the_harness_was_found() {
    let w = world();
    w.harness_at("native");
    w.ran_at();
    let saw = |name: &str| fs::read_to_string(w.place("native").join(name)).unwrap_or_default();
    let arguments: Vec<String> =
        (0..).map_while(|n| fs::read_to_string(w.place("native").join(format!("arg{n}"))).ok()).collect();
    let fixed = ["-p", "--model", "sonnet", "--tools", "", "--strict-mcp-config", "--disable-slash-commands"];
    let fixed = [
        &fixed[..],
        &["--no-session-persistence", "--setting-sources", "local", "--output-format", "json", "--system-prompt"],
    ]
    .concat();
    assert_eq!(arguments.len(), 14, "{arguments:?}");
    assert_eq!(arguments[..13], fixed);
    assert_eq!(saw("stdin"), w.run("day").stdout, "the harness's input is what `jellymoon day` prints, byte for byte");
    let names: Vec<String> = saw("env").lines().filter_map(|line| Some(line.split_once('=')?.0.to_string())).collect();
    assert!(!names.iter().any(|name| name.starts_with("JELLYMOON") || name == "TZ"), "{names:?}");
    if cfg!(unix) {
        let allowed = ["PATH", "HOME", "TMPDIR", "USER", "LANG", "LC_ALL", "TERM", "CLAUDE_CONFIG_DIR"];
        assert!(names.iter().all(|name| allowed.contains(&name.as_str())), "{names:?}");
    }
}

#[test]
fn a_harness_found_off_path_can_start_a_helper_beside_it_or_in_a_well_known_place() {
    let child_path = |w: &World, place: &str| {
        let env = fs::read_to_string(w.place(place).join("env")).unwrap();
        let path = env.lines().find_map(|line| line.strip_prefix("PATH=").or_else(|| line.strip_prefix("Path=")));
        std::env::split_paths(path.unwrap()).collect::<Vec<_>>()
    };
    for helper_in in ["native", "sys"] {
        let w = world();
        w.harness_at("native");
        install(&w.place(helper_in), "node");
        fs::write(w.place("native").join("needs"), "node").unwrap();
        assert_eq!(w.ran_at(), ["native"]);
        let want = ["path", "native", "local", "sys"].map(|place| w.place(place));
        let path = child_path(&w, "native");
        assert!(path[0] == want[0] && want.iter().all(|folder| path.contains(folder)), "{path:?}");
    }
    // A harness that PATH has is given PATH as it is.
    let w = world();
    w.harness_at("path");
    w.ran_at();
    assert_eq!(child_path(&w, "path"), [w.place("path")]);
}

#[test]
fn nothing_found_is_one_line_that_names_where_it_looked() {
    let w = world();
    let ran = w.run("page");
    let sys = w.place("sys").display().to_string();
    assert_eq!((ran.code, ran.stdout.as_str(), ran.stderr.lines().count()), (1, "", 1), "{}", ran.stderr);
    for said in ["jellymoon: finding Claude Code", "PATH", "~/.local/bin", "~/.claude/local", &sys, "JELLYMOON_CLAUDE"]
    {
        assert!(ran.stderr.contains(said), "{said:?} is not in {}", ran.stderr);
    }
    assert!(!ran.stderr.contains("passed over"), "{}", ran.stderr);
}

#[cfg(unix)]
#[test]
fn a_file_that_cannot_be_run_or_that_others_can_write_to_is_passed_over() {
    let w = world();
    with_mode(&w.place("native"), 0o644);
    with_mode(&w.place("local"), 0o775);
    fs::create_dir(w.place("sys").join("claude")).unwrap(); // a folder under the name is not a program
    let ran = w.run("page");
    assert_eq!((ran.code, ran.stdout.as_str()), (1, ""), "{}", ran.stderr);
    assert!(ran.stderr.contains("passed over ~/.claude/local/claude: others can write to it"), "{}", ran.stderr);
    assert!(!ran.stderr.contains("~/.local/bin/claude"), "one that cannot be run is not named: {}", ran.stderr);
    with_mode(&w.place("local"), 0o757);
    assert!(w.run("page").stderr.contains("passed over ~/.claude/local/claude"));
    // The next place's harness is the one run, and a link to a program counts as the program.
    fs::remove_dir(w.place("sys").join("claude")).unwrap();
    // The made-up harness leaves what it saw beside the link on a Mac and beside the program on Linux.
    let real = w.harness_at("elsewhere");
    fs::remove_file(w.harness_at("sys")).unwrap();
    std::os::unix::fs::symlink(real, w.place("sys").join("claude")).unwrap();
    w.ran_at();
    assert!(["sys", "elsewhere"].iter().any(|place| w.place(place).join("arg0").exists()));
}
