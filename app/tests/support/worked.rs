//! A made wish whose run has finished, for the F47 tests, in the world of `wished.rs`. The wish is made and run
//! by the `jellymoon` command, as a person would type them, with the made-up agent of F39's tests in place of
//! Claude Code; the taps are carried by a made-up window in a process of its own (`tapper.rs`), so the git core
//! starts has the world's variables and none of this process's.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{env, fs};

use serde_json::Value;
use sticker_kit::Scene;

use super::Piece;
use super::day::{self, DAY};
use super::wished::{SAID, Window, World, wait_until, world};

/// Every file under a folder, by its path, with what it holds.
pub type Files = BTreeMap<PathBuf, Vec<u8>>;

pub fn tree(folder: &Path) -> Files {
    let mut files = Files::new();
    let mut left = vec![folder.to_path_buf()];
    while let Some(folder) = left.pop() {
        for entry in fs::read_dir(&folder).into_iter().flatten() {
            let path = entry.unwrap().path();
            if path.is_dir() && !path.is_symlink() {
                left.push(path);
            } else {
                files.insert(path.clone(), fs::read(&path).unwrap_or_default());
            }
        }
    }
    files
}

/// A made wish for `alpha`, `m1`, and its run ended, with the agent playing `script`: the world, and the repo.
pub fn finished(script: &str) -> (World, PathBuf) {
    let w = world();
    fs::create_dir(w.at("harness")).unwrap();
    w.play(script);
    let repo = w.at("work").join("alpha");
    let made = w.jellymoon(&["wish", "--date", DAY, "--say", SAID, "--project", "alpha"]);
    assert_eq!(made["wish"]["id"], "m1", "{made}");
    let (logs, named) = (w.at("logs"), repo.to_str().unwrap());
    let ran = w.jellymoon(&["run", "--wish", "m1", "--date", DAY, "--repo", named, "--logs", logs.to_str().unwrap()]);
    assert_eq!(ran["outcome"], "completed", "{ran}");
    (w, repo)
}

impl World {
    /// The world's variables as a terminal has them: Jellymoon's home, and the made-up agent beside git.
    fn typed(&self, harness: &str) -> Vec<(&'static str, OsString)> {
        let git = self.vars.iter().find(|(name, _)| *name == "PATH").unwrap().1.clone();
        let path = env::join_paths([self.at(harness), PathBuf::from(git)]).unwrap();
        let mut vars: Vec<_> = self.vars.iter().filter(|(name, _)| *name != "PATH").cloned().collect();
        vars.extend([
            ("PATH", path),
            ("JELLYMOON_HOME", self.at("jm").into()),
            ("CLAUDE_CONFIG_DIR", self.at("harness").into()),
            ("GIT_CEILING_DIRECTORIES", self.day.dir.path().into()),
        ]);
        vars
    }

    /// The `jellymoon` command, typed in the world's own folder: the one line it printed.
    pub fn jellymoon(&self, args: &[&str]) -> Value {
        let mut typed = Command::new(env!("CARGO_BIN_EXE_f42-jellymoon"));
        typed.args(args).current_dir(self.day.dir.path());
        typed.env_clear().envs(self.typed("agent")).stdin(Stdio::null());
        let ran = typed.output().unwrap();
        let said = String::from_utf8_lossy(&ran.stdout);
        assert!(ran.status.success(), "jellymoon {args:?}: {said} {}", String::from_utf8_lossy(&ran.stderr));
        serde_json::from_str(said.trim_end()).expect("one line of JSON")
    }

    /// What git says in the repo, with the world's variables.
    pub fn told(&self, repo: &Path, args: &[&str]) -> String {
        let mut git = Command::new("git");
        git.args(args).current_dir(repo).env_clear().envs(self.typed("agent")).stdin(Stdio::null());
        let ran = git.output().unwrap();
        assert!(ran.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&ran.stderr));
        String::from_utf8(ran.stdout).unwrap()
    }

    /// Everything a tap might change: the person's checkout with its index and `HEAD`, every ref of the repo
    /// with what it names, and every file of Jellymoon's home. Nothing here makes git write.
    pub fn everything(&self, repo: &Path) -> (Files, String, Files) {
        (tree(repo), self.told(repo, &["for-each-ref", "--format=%(refname) %(objectname)"]), tree(&self.at("jm")))
    }

    /// What `jellymoon-app layout` places for the day, with git where core finds it.
    pub fn placed(&self) -> Vec<Piece> {
        let path = self.typed("read").into_iter().find(|(name, _)| *name == "PATH").unwrap().1;
        let config = self.at("gitconfig");
        let more = [
            ("PATH", path.to_str().unwrap()),
            ("GIT_CONFIG_GLOBAL", config.to_str().unwrap()),
            ("GIT_CONFIG_NOSYSTEM", "1"),
            ("GIT_CEILING_DIRECTORIES", self.day.dir.path().to_str().unwrap()),
        ];
        let out = day::run(&self.day, &[], &more);
        let said = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success() && said.is_empty(), "layout: {said}");
        let lines = String::from_utf8(out.stdout).unwrap();
        lines.lines().map(|line| Piece(serde_json::from_str(line).expect(line))).collect()
    }

    /// Starts the made-up window of `tapper.rs` on the logs in the folder `logs`, with its acts.
    pub fn taps(&self, logs: &str, acts: &[&str]) -> Window {
        let said = self.at("tmp").join(format!("window-{}", fs::read_dir(self.at("tmp")).unwrap().count()));
        let file = |name: &str| fs::File::create(said.with_extension(name)).unwrap();
        let mut window = Command::new(env!("CARGO_BIN_EXE_f47-tapper"));
        window.arg(self.at("jm")).arg(self.at(logs)).arg(self.at("agent").join("claude")).arg(DAY).args(acts);
        window.current_dir(self.day.dir.path()).env_clear().envs(self.typed("git"));
        window.stdin(Stdio::null()).stdout(file("out")).stderr(file("err"));
        Window { child: window.spawn().unwrap(), said }
    }
}

impl Window {
    /// The lines the window has printed so far.
    fn lines(&self) -> Vec<String> {
        let printed = fs::read_to_string(self.said.with_extension("out")).unwrap();
        printed.lines().filter(|_| printed.ends_with('\n')).map(str::to_string).collect()
    }

    /// Waits until the window has printed `count` lines, while it is still going.
    pub fn printed(&mut self, count: usize) {
        wait_until("the window's lines", || {
            let ended = self.child.try_wait().unwrap();
            assert!(
                ended.is_none(),
                "the window ended early: {}",
                fs::read_to_string(self.said.with_extension("err")).unwrap()
            );
            self.lines().len() >= count
        });
    }

    /// What the window showed, once it has ended: a scene for each act, or none where it showed nothing.
    pub fn showed(mut self) -> Vec<Option<Scene>> {
        self.ended().iter().map(|line| serde_json::from_str(line).expect(line)).collect()
    }

    /// The lines the window printed, once it has ended well.
    pub fn ended(&mut self) -> Vec<String> {
        wait_until("the window's end", || self.child.try_wait().unwrap().is_some());
        let ended = self.child.wait().unwrap();
        assert!(ended.success(), "{}", fs::read_to_string(self.said.with_extension("err")).unwrap());
        self.lines()
    }
}
