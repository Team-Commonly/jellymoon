//! The run of a wish entered in the window. A made-up window (`support/wisher.rs`) enters the wish in a process of
//! its own, started with the test's variables only; core starts the `jellymoon` command the test put where
//! `install.sh` puts it, and that starts the made-up agent of F39's tests in place of Claude Code. The repo is found
//! by its name in a session log typed here, so the world is in the system's own temporary folder, as F41's is.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use std::{env, fs, thread};

use jellymoon_app::wishing::UNSTARTED;
use serde_json::Value;
use sticker_kit::{Mark, Note, Scene, State};

use super::{SAID, drawn, first};
use crate::support::day::{self, DAY, answer, called, reading, result, said, shown, write_log};
use crate::support::open_above;

const STAR: &str = "a run reported it finished";

/// A day read and kept in two projects: `alpha`, a repo with one commit, and `beta`, whose folder is not there.
struct World {
    day: day::World,
    /// What the made-up window is started with, and through it the `jellymoon` command and git.
    vars: Vec<(&'static str, OsString)>,
}

fn copied(built: &str, program: &Path) {
    fs::create_dir_all(program.parent().unwrap()).unwrap();
    fs::copy(built, program).unwrap();
    #[cfg(unix)]
    fs::set_permissions(program, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
}

fn world() -> World {
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
    fn at(&self, name: &str) -> PathBuf {
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
    fn play(&self, script: &str) {
        fs::write(self.at("agent").join("play"), format!("{script}\n{}\n0\n", self.at("rec").display())).unwrap();
    }

    /// How many times the made-up agent was started.
    fn starts(&self) -> usize {
        fs::read_to_string(self.at("rec").join("starts")).map_or(0, |starts| starts.lines().count())
    }

    fn bin(&self) -> PathBuf {
        self.at("user").join(".local").join("bin")
    }

    /// Puts the `jellymoon` command where `install.sh` puts it, in folders for their owner alone to write.
    fn install(&self) -> PathBuf {
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
    fn enters(&self, option: usize, then: &str) -> Window {
        let said = self.at("tmp").join(format!("window-{}", fs::read_dir(self.at("tmp")).unwrap().count()));
        let file = |name: &str| fs::File::create(said.with_extension(name)).unwrap();
        let mut window = Command::new(env!("CARGO_BIN_EXE_f42-wisher"));
        window.arg(self.at("jm")).arg(self.at("logs")).arg(self.bin()).arg(self.at("agent").join("claude"));
        window.args([DAY, &option.to_string(), SAID, then]).current_dir(self.day.dir.path());
        window.env_clear().envs(self.vars.iter().map(|(name, value)| (name, value)));
        window.stdin(Stdio::null()).stdout(file("out")).stderr(file("err"));
        Window { child: window.spawn().unwrap(), said }
    }

    /// The first made wish as it is kept.
    fn kept(&self) -> Value {
        let kept = fs::read_to_string(self.at("jm").join("pages").join(format!("{DAY}.wishes.json"))).unwrap();
        serde_json::from_str::<Value>(&kept).unwrap_or_default()["wishes"][0].clone()
    }
}

/// The made-up window, going, and the files what it prints goes to (`.out` and `.err`).
struct Window {
    child: Child,
    said: PathBuf,
}

/// The scenes the made-up window showed, once it has ended: a minute is waited at most.
fn showed(mut window: Window) -> Vec<Scene> {
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

fn wait_until(what: &str, mut there: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !there() {
        assert!(Instant::now() < deadline, "{what} never came");
        thread::sleep(Duration::from_millis(10));
    }
}

/// The rows of the list of what came true.
fn came_true(scene: &Scene) -> Vec<(String, String)> {
    let rows = scene.notes.iter().find_map(|note| match note {
        Note::List { title, rows } if title == "came true today" => Some(rows.clone()),
        _ => None,
    });
    rows.unwrap_or_default()
}

#[test]
fn a_wish_entered_starts_its_run_apart_and_the_page_shows_its_star_with_no_tap() {
    let w = world();
    w.install();
    if open_above(w.day.dir.path()).is_some() {
        return;
    }
    w.play("slow");
    let window = w.enters(0, "watch");
    wait_until("the agent", || w.at("rec").join("started").is_file());
    let going = w.kept();
    assert_eq!((&going["state"], going["run"].is_string()), (&Value::from("working"), true), "{going}");
    fs::write(w.at("rec").join("go"), "").unwrap();

    let scenes = showed(window);
    assert_eq!(scenes.len(), 2, "the wish at once, and the star: the asking had nothing to say");
    // At once: working, in the person's words, before any run was asked for.
    assert_eq!(drawn(&scenes[0], "m1"), Some((State::Busy, 0, Mark::Busy, SAID.to_string())));
    // The same page on show, looked at again: no tap, and never opened again.
    assert_eq!(drawn(&scenes[1], "m1"), Some((State::Idle, 1, Mark::Star, SAID.to_string())));
    let stars: Vec<String> = came_true(&scenes[1]).into_iter().map(|(star, _)| star).collect();
    assert!(stars.contains(&STAR.to_string()) && !came_true(&scenes[0]).iter().any(|(star, _)| star == STAR));
    assert_eq!((w.starts(), called(&w.day)), (1, false), "one run of the agent, and no reading call");
}

// The run is the installed command's, not the window's: the window is gone while the agent still waits, and the
// run ends, with its star, all the same.
#[test]
fn the_run_of_a_wish_goes_on_when_the_window_is_gone() {
    let w = world();
    w.install();
    if open_above(w.day.dir.path()).is_some() {
        return;
    }
    w.play("slow");
    let scenes = showed(w.enters(0, "leave"));
    assert_eq!(scenes.len(), 1, "{:?}", scenes.last().and_then(first));
    wait_until("the agent", || w.at("rec").join("started").is_file());
    assert_eq!(w.kept()["state"], "working", "the run ended with the window");
    fs::write(w.at("rec").join("go"), "").unwrap();
    wait_until("the star", || w.kept()["stars"] == serde_json::json!([STAR]));
    wait_until("the end of the command", || {
        !w.at("jm").join("runs").join(w.kept()["run"].as_str().unwrap()).join("run.lock").exists()
    });
    assert_eq!((w.kept()["state"].as_str(), w.starts()), (Some("done"), 1));
}

/// What the page says of a wish that was entered and has no run: the wish is kept and drawn working, and one note.
fn unstarted(w: &World, option: usize, wish: &str) -> String {
    let scenes = showed(w.enters(option, "leave"));
    assert_eq!(scenes.len(), 2, "the wish at once, then why it has no run");
    assert!(drawn(&scenes[0], wish).is_some() && first(&scenes[0]).is_none_or(|(title, _)| title != UNSTARTED));
    assert_eq!(drawn(&scenes[1], wish).map(|drawn| drawn.0), Some(State::Busy));
    let said = scenes[1].notes.iter().filter(|note| matches!(note, Note::Plain { title, .. } if title == UNSTARTED));
    assert_eq!(said.count(), 1);
    let copies = fs::read_dir(w.at("jm").join("copies")).map_or(0, Iterator::count);
    assert_eq!((w.starts(), copies), (0, 0), "something was started");
    let (title, body) = first(&scenes[1]).unwrap();
    assert_eq!(title, UNSTARTED);
    body.to_string()
}

#[test]
fn a_command_that_is_not_found_and_a_run_that_was_not_started_each_leave_one_note_that_says_why() {
    let w = world();
    assert_eq!(unstarted(&w, 0, "m1"), "the installed jellymoon command is not used: it was not found.");
    let command = w.install();
    #[cfg(unix)]
    {
        // A command others can write to is not the one `install.sh` put there, and is never started.
        fs::set_permissions(&command, std::os::unix::fs::PermissionsExt::from_mode(0o775)).unwrap();
        assert_eq!(unstarted(&w, 0, "m2"), "the installed jellymoon command is not used: others can write to it.");
        fs::set_permissions(&command, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    }
    if open_above(w.day.dir.path()).is_some() {
        return;
    }
    // The command is started, finds no folder for the project, and says so: its line is the note.
    let wish = if cfg!(unix) { "m3" } else { "m2" };
    let no_repo = "the folder of that name typed in on 2026-03-10 is not one a run can copy: it is not there, or cannot \
                   be reached.";
    assert_eq!(unstarted(&w, 1, wish), no_repo);
    assert!(command.is_file());
}
