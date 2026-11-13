//! What the tests of a keep (F45) and of a toss (F46) share: one run the made-up agent finished, the real git in
//! its repo, a keep of it, and everything a keep or a toss could change, held to compare.
#![allow(dead_code)]

use std::ffi::OsString;
use std::fs;
use std::path::Path;

use serde_json::Value;

use super::{DAY, Made, ran, refused, world};
use crate::support::Ran;

/// What [`Made::held`] holds.
pub type Held = ((Vec<(std::path::PathBuf, Vec<u8>)>, Vec<u8>, Vec<u8>, Vec<String>), String, Value);

/// One run that finished: the made-up agent edited `a.txt`, removed `b.txt` and made `n.txt`.
pub fn finished() -> Made {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (run, said) = ran(&w.start(&wish, &repo));
    assert_eq!(said["outcome"], "completed", "{said}");
    Made { w, repo, wish, run }
}

impl Made {
    pub fn git(&self, args: &[&str]) -> String {
        self.w.git(&self.repo, args)
    }

    /// `jellymoon run keep` for the wish, in `repo`, with `more` arguments and `vars`.
    pub fn keep_in(&self, repo: &Path, more: &[&str], vars: &[(&'static str, OsString)]) -> Ran {
        let asked = ["run", "keep", "--wish", &self.wish, "--date", DAY, "--repo", repo.to_str().unwrap()];
        self.w.run_with(vars, &[&asked[..], more].concat())
    }

    pub fn keep(&self, more: &[&str]) -> Ran {
        self.keep_in(&self.repo, more, &[])
    }

    /// The commit the run ended as, by its record.
    pub fn end(&self) -> String {
        self.record()["end"].as_str().unwrap().to_string()
    }

    /// The person says who they are in this repo, and commits a change to `c.txt` and a file of their own.
    pub fn moved_on(&self) -> String {
        self.who();
        fs::write(self.repo.join("c.txt"), "c.txt as the person left it\n").unwrap();
        fs::write(self.repo.join("mine.txt"), "the person's own\n").unwrap();
        self.git(&["add", "c.txt", "mine.txt"]);
        self.git(&["commit", "-q", "-m", "the person's own commit"]);
        self.w.commit(&self.repo, "main")
    }

    pub fn who(&self) {
        self.git(&["config", "user.name", "Ada Example"]);
        self.git(&["config", "user.email", "ada@example.test"]);
    }

    /// Everything a keep could change: the person's files, index and `HEAD`, every ref with what it names, the
    /// run's own too, and the run's record.
    pub fn held(&self) -> Held {
        let refs = self.git(&["for-each-ref", "--format=%(refname) %(objectname) %(symref)"]);
        (self.w.persons(&self.repo), refs, self.record())
    }

    /// Fails unless the command was refused in these words, with nothing changed since `before`.
    pub fn refuses(&self, ran: &Ran, before: &Held, why: &str) {
        assert_eq!((ran.code, ran.stdout.as_str()), (1, ""), "{}", ran.stderr);
        assert_eq!(refused(ran), said(&why.replace("{run}", &self.run)));
        assert!(*before == self.held(), "a refused command changed something: {}", ran.stderr);
        self.w.never_says_the_agent(ran);
    }

    /// `jellymoon run toss` for the wish, in `repo`, with `vars`.
    pub fn toss_in(&self, repo: &Path, vars: &[(&'static str, OsString)]) -> Ran {
        self.w.run_with(vars, &["run", "toss", "--wish", &self.wish, "--date", DAY, "--repo", repo.to_str().unwrap()])
    }

    pub fn toss(&self) -> Ran {
        self.toss_in(&self.repo, &[])
    }

    /// What `git status` says of the checkout, the files git ignores too.
    pub fn status(&self) -> String {
        self.git(&["status", "--porcelain", "--ignored"])
    }
}

/// A refusal as it is printed.
pub fn said(why: &str) -> String {
    format!("jellymoon: {why}\n")
}
