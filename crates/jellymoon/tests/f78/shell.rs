//! What the tests of F78 share: F39's test folder with no harness on its `PATH`, a made-up login shell
//! (`support/fake_shell.rs`) under a login shell's name for `JELLYMOON_SHELL` to name, and the made-up harnesses
//! put in the folders that login shell names. No real login shell is started and no record of a user is read: the
//! door sets `JELLYMOON_SHELL` in every world, to nothing unless the test names this one.
#![allow(dead_code)]

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde_json::json;

use crate::support::{Ran, install, plant, said, started, write_log};
use crate::world::{DAY, World, world};

/// The one command core gives a login shell, as its fourth argument: typed here, and compared whole.
pub const COMMAND: &str = r#"printf '%s%s%s%s%s\n' __jellymoon _path__ "$PATH" __jellymoon _path__"#;
/// How the line of a search that found no harness begins and ends, with the rest of the places looked in between.
pub const NONE: &str = "jellymoon: finding Claude Code: no `claude` program in PATH, ~/.local/bin, ~/.claude/local";
pub const INSTALL: &str = "; install Claude Code, or name the program with JELLYMOON_CLAUDE\n";

/// The made-up login shell of one world: its program, and the folder it leaves its record in.
pub struct Shell {
    pub program: PathBuf,
    pub rec: PathBuf,
}

/// A world with something typed on the day, and no harness on `PATH` or in a fixed place: `PATH` is one folder
/// that holds nothing but the world's link to the real `git`.
pub fn finder() -> World {
    let mut w = world();
    let line = said("2026-03-10T16:00:00.000Z", "/work/alpha", "add a retry");
    write_log(&w.at("logs"), "-work-alpha/s1.jsonl", &[line]);
    w.vars.push(("PATH", w.at("git").into()));
    w
}

impl World {
    /// Puts the made-up login shell in the folder `shell` under `name`, and names it with `JELLYMOON_SHELL`.
    pub fn shell(&mut self, name: &str) -> Shell {
        let rec = self.at("shell");
        fs::create_dir_all(&rec).unwrap();
        let program = rec.join(name);
        plant(env!("CARGO_BIN_EXE_fake-shell"), &program);
        self.vars.push(("JELLYMOON_SHELL", program.clone().into()));
        Shell { program, rec }
    }

    /// A folder of the world, made.
    pub fn made(&self, name: &str) -> PathBuf {
        let folder = self.at(name);
        fs::create_dir_all(&folder).unwrap();
        folder
    }

    /// Puts a harness for the reading call in a folder: it answers with an empty reading.
    pub fn reader_in(&self, folder: &Path) -> PathBuf {
        let program = install(folder, "claude");
        let reply = json!({"type": "result", "is_error": false, "result": r#"{"wishes":[],"clashes":[]}"#});
        fs::write(folder.join("reply"), reply.to_string()).unwrap();
        fs::write(folder.join("exit"), "0").unwrap();
        program
    }

    /// Puts a harness for a run in a folder: the made-up agent, which edits a file and says it is done.
    pub fn agent_in(&self, folder: &Path) {
        fs::create_dir_all(folder).unwrap();
        plant(env!("CARGO_BIN_EXE_fake-agent"), &folder.join("claude"));
        fs::write(folder.join("play"), format!("edits\n{}\n0\n", self.at("rec").display())).unwrap();
    }

    /// `jellymoon page` over the day, with words on its input that are nobody's to read.
    pub fn page(&self) -> Ran {
        let vars: Vec<_> = self.vars.iter().map(|(name, value)| (*name, value.as_os_str())).collect();
        let logs = self.at("logs");
        let args = ["page", "--date", DAY, "--logs", logs.to_str().unwrap()];
        started(self.dir.path(), &vars, &args, "typed at the command, and not the login shell's\n")
    }

    /// What `jellymoon harness` printed: one line, exit 0, nothing said beside it.
    pub fn harness(&self) -> String {
        let ran = self.run(&["harness"]);
        assert_eq!((ran.code, ran.stderr.as_str()), (0, ""), "{}", ran.stdout);
        assert!(ran.stdout.ends_with("}\n") && ran.stdout.matches('\n').count() == 1, "{:?}", ran.stdout);
        ran.stdout
    }

    /// A page that found no harness: the one line it said. Nothing was printed, and nothing was written.
    pub fn no_page(&self) -> String {
        let ran = self.page();
        assert_eq!((ran.code, ran.stdout.as_str()), (1, ""), "{}", ran.stderr);
        assert!(!self.at("jm").exists(), "a look that found no harness wrote into Jellymoon's folder");
        ran.stderr
    }
}

/// Puts a copy of a planted program in its place, with this mode. `plant` makes a hard link on Linux, so a mode
/// set on a planted file is set on the built program itself, for every test that starts it. A copy with a mode of
/// its own is never started.
#[cfg(unix)]
pub fn moded(program: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    let bytes = fs::read(program).unwrap();
    fs::remove_file(program).unwrap();
    fs::write(program, bytes).unwrap();
    fs::set_permissions(program, fs::Permissions::from_mode(mode)).unwrap();
}

/// The line `jellymoon harness` prints.
pub fn found(found: bool, how: &str, says: &str) -> String {
    format!("{}\n", json!({"found": found, "how": how, "says": says}))
}

impl Shell {
    pub fn play(&self, script: &str) {
        fs::write(self.rec.join("play"), script).unwrap();
    }

    /// The `PATH` it names: these folders, in their order.
    pub fn names(&self, folders: &[&Path]) {
        fs::write(self.rec.join("path"), std::env::join_paths(folders).unwrap().to_str().unwrap()).unwrap();
    }

    /// It starts one more program in its group, and neither ends by itself.
    pub fn stays(&self) {
        fs::write(self.rec.join("stays"), "").unwrap();
    }

    /// What it recorded under a name at its last start.
    pub fn said(&self, name: &str) -> String {
        fs::read_to_string(self.rec.join(name)).unwrap()
    }

    /// How many times it was started.
    pub fn starts(&self) -> usize {
        fs::read_to_string(self.rec.join("starts")).map_or(0, |starts| starts.lines().count())
    }

    /// Whether it and the program it left in its group were both there and are both gone, ten seconds at most:
    /// each held the system's lock on a file of its own for as long as it lived.
    pub fn gone(&self) -> bool {
        let free = |name: &str| fs::File::open(self.rec.join(name)).is_ok_and(|held| held.try_lock().is_ok());
        let from = Instant::now();
        while !(free("held") && free("left")) {
            if from.elapsed() > Duration::from_secs(10) {
                return false;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        self.rec.join("held-up").exists() && self.rec.join("left-up").exists()
    }
}

/// The folders of the `PATH` a made-up harness for the reading call recorded.
pub fn path_of(env: &str) -> Vec<PathBuf> {
    let path = env.lines().find_map(|line| line.strip_prefix("PATH=")).unwrap();
    std::env::split_paths(&OsString::from(path)).collect()
}
