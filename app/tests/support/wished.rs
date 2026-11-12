//! The world of a wish's run, for the F42 and F43 tests. A made-up window (`wisher.rs`) enters the wish in a
//! process of its own, started with the test's variables only; core starts the `jellymoon` command the test put
//! where `install.sh` puts it, and that starts the made-up agent of F39's tests in place of Claude Code. The repo
//! is found by its name in a session log typed here, so the world is in the system's own temporary folder.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use std::{env, fs, thread};

use jellymoon_app::taps::OnShow;
use jellymoon_app::today::Today;
use serde_json::Value;
use sticker_kit::Scene;

use super::day::{self, DAY, answer, called, reading, result, said, shown, write_log};

/// The words every made-up window enters.
pub const SAID: &str = "make the uploader retry twice";

/// A day read and kept in two projects: `alpha`, a repo with one commit, and `beta`, whose folder is not there.
pub struct World {
    pub day: day::World,
    /// What the made-up window is started with, and through it the `jellymoon` command and git.
    vars: Vec<(&'static str, OsString)>,
}

fn copied(built: &str, program: &Path) {
    fs::create_dir_all(program.parent().unwrap()).unwrap();
    fs::copy(built, program).unwrap();
    #[cfg(unix)]
    fs::set_permissions(program, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
}

pub fn world() -> World {
    let dir = tempfile::tempdir().unwrap();
    let at = |name: &str| dir.path().join(name);
    for folder in ["user", "rec", "tmp", "git", "work", "logs"] {
        fs::create_dir(at(folder)).unwrap();
    }
    let program = |name: &str| format!("{name}{}", env::consts::EXE_SUFFIX);
    copied(env!("CARGO_BIN_EXE_f34-claude"), &at("read").join(program("claude")));
    copied(env!("CARGO_BIN_EXE_f42-agent"), &at("agent").join(program("claude")));
    fs::write(at("gitconfig"), "").unwrap();
    // The real git, alone in a folder of the test's own where a link can be made: no harness is beside it.
    let path = env::var_os("PATH").unwrap();
    let real =
        env::split_paths(&path).map(|dir| dir.join(program("git"))).find(|git| git.is_absolute() && git.is_file());
    let real = real.expect("git is on PATH");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&real, at("git").join("git")).unwrap();
    let git = if cfg!(unix) { at("git") } else { real.parent().unwrap().to_path_buf() };
    let home = [("TZ", "America/Los_Angeles".into()), ("HOME", at("user").into()), ("USERPROFILE", at("user").into())];
    let mut vars = home.to_vec();
    vars.extend([
        ("PATH", git.into_os_string()),
        ("JELLYMOON_SYSTEM_BIN", OsString::new()),
        ("GIT_CONFIG_GLOBAL", at("gitconfig").into()),
        ("GIT_CONFIG_NOSYSTEM", "1".into()),
        ("TMPDIR", at("tmp").into()),
        ("LC_ALL", "C".into()),
    ]);
    if let Some(root) = env::var_os("SystemRoot").filter(|_| cfg!(windows)) {
        vars.push(("SystemRoot", root)); // a Windows program needs it to start
    }
    let mut reading_vars = home.to_vec();
    reading_vars.extend([
        ("JELLYMOON_HOME", at("jm").into()),
        ("PATH", at("read").into()),
        ("JELLYMOON_SYSTEM_BIN", OsString::new()),
    ]);
    let world = World { day: day::World { home: at("jm"), rec: at("read"), vars: reading_vars, dir }, vars };
    let repo = world.repo("alpha");
    let typed = |folder: &Path| [said("2026-03-10T16:00:00.000Z", folder.to_str().unwrap(), "look at the uploader")];
    write_log(&world.day, "-work-alpha/s1.jsonl", &typed(&repo));
    write_log(&world.day, "-work-beta/s2.jsonl", &typed(&world.at("work").join("beta")));
    answer(&world.day, &result(&reading()), 0);
    shown(&world.day, &[]);
    assert!(called(&world.day), "the day was to be read once, before any wish");
    world.play("edits");
    world
}

impl World {
    pub fn at(&self, name: &str) -> PathBuf {
        self.day.dir.path().join(name)
    }

    fn git(&self, folder: &Path, args: &[&str]) {
        let mut git = Command::new("git");
        git.args(args).current_dir(folder).env_clear().envs(self.vars.iter().map(|(name, value)| (name, value)));
        git.env("GIT_CEILING_DIRECTORIES", self.day.dir.path());
        for who in ["AUTHOR", "COMMITTER"] {
            git.env(format!("GIT_{who}_NAME"), "Test Setup").env(format!("GIT_{who}_EMAIL"), "setup@example.test");
        }
        let ran = git.output().unwrap();
        assert!(ran.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&ran.stderr));
    }

    /// A repo of two files and one commit, at `work/<name>`.
    fn repo(&self, name: &str) -> PathBuf {
        let repo = self.at("work").join(name);
        fs::create_dir(&repo).unwrap();
        self.git(&repo, &["init", "-q", "-b", "main"]);
        for file in ["a.txt", "b.txt"] {
            fs::write(repo.join(file), format!("{file} as it was\n")).unwrap();
        }
        self.git(&repo, &["add", "."]);
        self.git(&repo, &["commit", "-q", "-m", "the first commit"]);
        repo
    }

    /// Tells the made-up agent which script to play.
    pub fn play(&self, script: &str) {
        fs::write(self.at("agent").join("play"), format!("{script}\n{}\n0\n", self.at("rec").display())).unwrap();
    }

    /// How many times the made-up agent was started.
    pub fn starts(&self) -> usize {
        fs::read_to_string(self.at("rec").join("starts")).map_or(0, |starts| starts.lines().count())
    }

    fn bin(&self) -> PathBuf {
        self.at("user").join(".local").join("bin")
    }

    /// Puts the `jellymoon` command where `install.sh` puts it, in folders for their owner alone to write.
    pub fn install(&self) -> PathBuf {
        let command = self.bin().join(format!("jellymoon{}", env::consts::EXE_SUFFIX));
        copied(env!("CARGO_BIN_EXE_f42-jellymoon"), &command);
        #[cfg(unix)]
        for folder in self.bin().ancestors().take(4) {
            fs::set_permissions(folder, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
        }
        command
    }

    /// Starts the made-up window: the words entered for the project of chip `option`, and what it does then. What
    /// it prints goes to two files, never to a pipe: on Windows the run it starts inherits the handles it holds,
    /// and a pipe read here would stay open until that run ended.
    pub fn enters(&self, option: usize, then: &str) -> Window {
        self.enters_with(option, then, "")
    }

    /// [`World::enters`] in a window started with `--skip <skip>`, when that is not empty.
    pub fn enters_with(&self, option: usize, then: &str, skip: &str) -> Window {
        let said = self.at("tmp").join(format!("window-{}", fs::read_dir(self.at("tmp")).unwrap().count()));
        let file = |name: &str| fs::File::create(said.with_extension(name)).unwrap();
        let mut window = Command::new(env!("CARGO_BIN_EXE_f42-wisher"));
        window.arg(self.at("jm")).arg(self.at("logs")).arg(self.bin()).arg(self.at("agent").join("claude"));
        window.args([DAY, &option.to_string(), SAID, then, skip]).current_dir(self.day.dir.path());
        window.env_clear().envs(self.vars.iter().map(|(name, value)| (name, value)));
        window.stdin(Stdio::null()).stdout(file("out")).stderr(file("err"));
        Window { child: window.spawn().unwrap(), said }
    }

    /// The day as a window opened now holds it, in this process. Nothing here may ask for a run: the command
    /// core starts would inherit this process's variables.
    pub fn on_show(&self) -> OnShow {
        let today = Today {
            date: DAY.to_string(),
            logs: self.at("logs"),
            skip: Vec::new(),
            model: "sonnet".to_string(),
            home: self.at("jm"),
            harness: Some(self.at("agent").join("claude")),
            fresh: false,
            settings: None,
            bin: Some(self.bin()),
        };
        let (scene, page) = today.first();
        OnShow { today, page, scene }
    }

    /// Waits for the `jellymoon run` of the first made wish to be gone: a test ends it before its folder goes.
    pub fn ended(&self) {
        let run = self.kept()["run"].as_str().unwrap().to_string();
        wait_until("the end of the command", || !self.at("jm").join("runs").join(&run).join("run.lock").exists());
    }

    /// The first made wish as it is kept.
    pub fn kept(&self) -> Value {
        let kept = fs::read_to_string(self.at("jm").join("pages").join(format!("{DAY}.wishes.json"))).unwrap();
        serde_json::from_str::<Value>(&kept).unwrap_or_default()["wishes"][0].clone()
    }
}

/// The made-up window, going, and the files what it prints goes to (`.out` and `.err`).
pub struct Window {
    child: Child,
    said: PathBuf,
}

/// The scenes the made-up window showed, once it has ended: a minute is waited at most.
pub fn showed(mut window: Window) -> Vec<Scene> {
    let deadline = Instant::now() + Duration::from_secs(60);
    let ended = loop {
        if let Some(ended) = window.child.try_wait().unwrap() {
            break ended;
        }
        if Instant::now() > deadline {
            window.child.kill().unwrap();
            window.child.wait().unwrap();
            panic!("the made-up window was still open after a minute");
        }
        thread::sleep(Duration::from_millis(10));
    };
    let said = |name: &str| fs::read_to_string(window.said.with_extension(name)).unwrap();
    assert!(ended.success(), "{}", said("err"));
    said("out").lines().map(|line| serde_json::from_str(line).unwrap()).collect()
}

pub fn wait_until(what: &str, mut there: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !there() {
        assert!(Instant::now() < deadline, "{what} never came");
        thread::sleep(Duration::from_millis(10));
    }
}
