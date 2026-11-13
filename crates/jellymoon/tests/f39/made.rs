//! What the tests of a run's work and of its record share (F44, F48): one run of the wish `m1` in the repo
//! `work/alpha` with its record, and the made-up git that stands first on a `PATH` (F38's `fake-git`).

use std::env::consts::EXE_SUFFIX;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::{env, fs};

use serde_json::Value;

use super::World;
use crate::support::{plant, skipped};

/// The three things F48 adds to a run's record, in the order they are written.
pub const NAMED: [&str; 3] = ["repo", "start", "end"];

/// One run of the wish `m1` in the repo `work/alpha`.
pub struct Made {
    pub w: World,
    pub repo: PathBuf,
    pub wish: String,
    pub run: String,
}

impl Made {
    pub fn branch(&self) -> String {
        format!("refs/heads/jellymoon/run-{}", self.run)
    }

    pub fn record(&self) -> Value {
        self.w.record(&self.run)
    }

    /// Writes the run's record by hand, as anyone who can write Jellymoon's home can.
    pub fn write(&self, record: &Value) {
        fs::write(self.w.jm("runs").join(&self.run).join("run.json"), format!("{record}\n")).unwrap();
    }

    /// The record as a build before F48 wrote it: without the three. Written so, a look goes by the branch's
    /// last commit and its one parent.
    pub fn as_before(&self) -> Value {
        let mut record = self.record();
        for name in NAMED {
            record.as_object_mut().unwrap().remove(name);
        }
        record
    }
}

impl World {
    /// Makes git warn at every command in `repo`: its `core.fsync` names 400 parts no git knows, and a git from
    /// 2.36 on says so of each. False, with the reason printed, on a git from before that, which says nothing.
    pub fn warns(&self, repo: &Path) -> bool {
        let parts: Vec<String> = (0..400).map(|n| format!("no-such-part-{n}")).collect();
        self.git(repo, &["config", "core.fsync", &parts.join(",")]);
        let version = self.git(repo, &["version"]);
        let mut numbers = version.split([' ', '.']).filter_map(|part| part.parse::<u32>().ok());
        if (numbers.next(), numbers.next()) < (Some(2), Some(36)) {
            skipped("this git is from before 2.36, and does not warn of a part of core.fsync it does not know");
            return false;
        }
        let said = self.git_ran(repo, &["for-each-ref"]);
        let warned = String::from_utf8_lossy(&said.stderr).lines().filter(|line| line.contains("core.fsync")).count();
        assert!(warned >= 400, "git warned {warned} times of the parts of core.fsync it does not know");
        true
    }

    /// The variables that put `fake-git` first on `PATH` and give a git child one second from when the made-up
    /// git has left `stalled`. What it does, and at which step, is in the files a test leaves in `fake`
    /// (`support/fake_git.rs`): `on`, and with it `first` and `returns`. Asked again, it is the same made-up git.
    pub fn fake_git(&self) -> Vec<(&'static str, OsString)> {
        let (folder, name) = (self.at("fake"), format!("git{EXE_SUFFIX}"));
        if !folder.exists() {
            fs::create_dir(&folder).unwrap();
            plant(env!("CARGO_BIN_EXE_fake-git"), &folder.join(&name));
        }
        let path = &self.vars.iter().find(|(name, _)| *name == "PATH").unwrap().1;
        let real = env::split_paths(path).map(|dir| dir.join(&name)).find(|git| git.is_file()).unwrap();
        fs::write(folder.join("real"), real.to_str().unwrap()).unwrap();
        let first = env::join_paths([folder.clone()].into_iter().chain(env::split_paths(path))).unwrap();
        let stalled = folder.join("stalled").into();
        vec![
            ("PATH", first),
            ("JELLYMOON_TEST_GIT_LIMIT_MS", "1000".into()),
            ("JELLYMOON_TEST_GIT_LIMIT_FROM", stalled),
        ]
    }
}
