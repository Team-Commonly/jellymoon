//! F44: a finished run's work is shown as what it made, with no git and no folder (ADR 0017, decisions 1, 2, 3 and
//! 6; `docs/development/show.md`). `jellymoon run show` is run as a black box on runs the made-up `fake-agent`
//! finished in F39's test folder (`f39/world.rs`). Work of other shapes is put on the run's branch by the test, with
//! the real `git` and an index file of its own: no file of it is ever written to a disk, so a name no system could
//! hold is tested on every system. `f44/file.rs` holds `--file`, `f44/refused.rs` what is refused, and
//! `f44/names.rs` the names that look like options, the count the list is cut at and ten thousand files, and
//! `f44/obeyed.rs` the spellings a disk takes for a name the person's tools obey.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "f44/file.rs"]
mod file;
#[path = "f44/names.rs"]
mod names;
#[path = "f44/obeyed.rs"]
mod obeyed;
#[path = "f44/refused.rs"]
mod refused;
mod support;
#[path = "f39/world.rs"]
mod world;

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{env, fs};

use serde_json::{Value, json};
use support::{Ran, tree};
pub use world::*;

/// What every made-up file of a run's work holds somewhere: no file under a test's folder may hold it afterwards.
pub const WORK: &str = "plum-otter-made-by-the-run";

/// One run that finished, of the wish `m1` in the repo `work/alpha`: the made-up agent edited `a.txt`, removed
/// `b.txt` and made `n.txt`.
pub struct Made {
    pub w: World,
    pub repo: PathBuf,
    pub wish: String,
    pub run: String,
}

/// One file of made-up work: its mode as git writes it (`0` takes the file away), its path and what it holds. For
/// mode 160000 it holds nothing: the entry names a commit of another repo.
pub type Entry<'a> = (&'a str, &'a str, &'a [u8]);

pub fn finished() -> Made {
    finished_in(world())
}

fn finished_in(w: World) -> Made {
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (run, said) = ran(&w.start(&wish, &repo));
    assert_eq!(said["outcome"], "completed", "{said}");
    Made { w, repo, wish, run }
}

impl Made {
    pub fn branch(&self) -> String {
        format!("refs/heads/jellymoon/run-{}", self.run)
    }

    /// `jellymoon run show` for the wish, in the repo, with `more` arguments.
    pub fn show(&self, more: &[&str]) -> Ran {
        let asked = ["run", "show", "--wish", &self.wish, "--date", DAY, "--repo", self.repo.to_str().unwrap()];
        self.w.run(&[&asked[..], more].concat())
    }

    /// The real git in the repo, as the person, with `input` on its input and, when `index` names one, an index
    /// file of the test's own in place of the person's.
    pub fn git(&self, args: &[&str], input: &[u8], index: Option<&Path>) -> String {
        let mut git = Command::new("git");
        // A name Windows or a Mac could not hold as a file is still a name in a tree: no file of it is written.
        git.args(["-c", "core.protectNTFS=false", "-c", "core.protectHFS=false"]);
        git.args(args).current_dir(&self.repo).env_clear().envs(self.w.vars.iter().map(|(name, value)| (name, value)));
        git.env("GIT_CEILING_DIRECTORIES", self.w.dir.path()).envs(index.map(|index| ("GIT_INDEX_FILE", index)));
        for who in ["AUTHOR", "COMMITTER"] {
            git.env(format!("GIT_{who}_NAME"), "Test Setup").env(format!("GIT_{who}_EMAIL"), "setup@example.test");
        }
        git.envs(env::var_os("SystemRoot").map(|root| ("SystemRoot", root)));
        let mut git = git.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
        git.stdin.take().unwrap().write_all(input).unwrap();
        let ran = git.wait_with_output().unwrap();
        assert!(ran.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&ran.stderr));
        String::from_utf8(ran.stdout).unwrap().trim_end().to_string()
    }

    /// A commit of `entries` over the person's `main`, with the message given, made in the repo's store only.
    pub fn commit_of(&self, entries: &[Entry], parents: &[&str], message: &str) -> String {
        let index = self.w.at("tmp").join("made-up-index");
        let _ = fs::remove_file(&index);
        self.git(&["read-tree", "main"], b"", Some(&index));
        let (mut info, mut stored) = (Vec::new(), HashMap::new());
        for (mode, path, holds) in entries {
            let store = || self.git(&["hash-object", "-w", "--stdin"], holds, None);
            let blob = match *mode {
                "0" => "0".repeat(40),
                "160000" => self.w.commit(&self.repo, "main"),
                _ => stored.entry(*holds).or_insert_with(store).clone(),
            };
            info.extend(format!("{mode} {blob}\t{path}\0").into_bytes());
        }
        self.git(&["update-index", "-z", "--index-info"], &info, Some(&index));
        let tree = self.git(&["write-tree"], b"", Some(&index));
        let parents: Vec<&str> = parents.iter().flat_map(|parent| ["-p", parent]).collect();
        self.git(&[&["commit-tree"][..], &parents, &["-m", message, &tree]].concat(), b"", None)
    }

    /// Puts made-up work where the run's own was: one commit over `main`, with the message core gives a run's.
    pub fn rework(&self, entries: &[Entry]) {
        let made = self.commit_of(entries, &["main"], &format!("Jellymoon run {}", self.run));
        self.git(&["update-ref", &self.branch(), &made], b"", None);
    }

    /// Everything git keeps of the repo that a look could change: the person's files, index and `HEAD`, and every
    /// ref with what it names, the run's own too.
    pub fn held(&self) -> (impl PartialEq + std::fmt::Debug, String) {
        (self.w.persons(&self.repo), self.w.git(&self.repo, &["for-each-ref", "--format=%(refname) %(objectname)"]))
    }

    /// Fails if a file under the test's folder holds the work's words, or has the name of one of its files.
    pub fn nothing_on_disk(&self, names: &[&str]) {
        for (path, bytes) in tree(self.w.dir.path()) {
            // The repo's own store holds the work, packed its way: what is looked at is everything outside it.
            if path.components().any(|part| part.as_os_str() == ".git") {
                continue;
            }
            let named = path.file_name().is_some_and(|last| names.iter().any(|name| last == *name));
            assert!(!named, "{} is a file of the work", path.display());
            assert!(!bytes.windows(WORK.len()).any(|window| window == WORK.as_bytes()), "{}", path.display());
        }
    }
}

/// One file as the list prints it.
pub fn listed(n: usize, name: &str, change: &str, size: usize, kind: &str, obeyed: bool) -> Value {
    json!({"n": n, "name": name, "change": change, "size": size, "kind": kind, "obeyed": obeyed})
}

#[test]
fn a_finished_run_is_shown_as_the_files_it_changed_and_made_and_one_of_them_as_its_text() {
    let made = finished();
    let before = made.held();
    let shown = printed(&made.show(&[]));
    let files = json!([
        listed(1, "a.txt", "changed", 18, "text", false),
        listed(2, "b.txt", "removed", 16, "text", false),
        listed(3, "n.txt", "made", 17, "text", false),
    ]);
    let want = json!({"run": made.run, "wish": "m1", "work": "waits", "files": files, "more": 0, "most": 200});
    assert_eq!(shown, want);
    let file = printed(&made.show(&["--file", "3"]));
    let want = json!({
        "run": made.run, "wish": "m1", "work": "waits", "n": 3, "name": "n.txt", "size": 17,
        "text": "new from the run", "cut": false, "most": 65536
    });
    assert_eq!(file, want);
    assert_eq!(made.held(), before, "a look changes nothing git keeps");
    assert!(!made.repo.join("n.txt").exists() && made.w.copies().is_empty(), "and writes no file of the work");
}

#[test]
fn an_object_is_read_as_it_is_whatever_the_repo_names_to_stand_in_its_place() {
    let made = finished();
    let shown = printed(&made.show(&[]));
    let tip = made.w.commit(&made.repo, &made.branch());
    // A replace ref for the file the run made, and one for the run's commit, each a stand-in of the same kind.
    let file = made.git(&["rev-parse", &format!("{tip}:n.txt")], b"", None);
    let other = made.git(&["hash-object", "-w", "--stdin"], format!("{WORK} stands in\n").as_bytes(), None);
    made.git(&["replace", &file, &other], b"", None);
    let stand_in =
        made.commit_of(&[("100644", "w.txt", WORK.as_bytes())], &["main"], &format!("Jellymoon run {}", made.run));
    made.git(&["replace", &tip, &stand_in], b"", None);
    assert_eq!(made.git(&["cat-file", "blob", &file], b"", None), format!("{WORK} stands in"), "git follows them");
    let ran = made.show(&[]);
    assert_eq!(printed(&ran), shown);
    let text = made.show(&["--file", "3"]);
    assert_eq!(printed(&text)["text"], "new from the run");
    assert!(!ran.stdout.contains("w.txt") && !text.stdout.contains(WORK));
}

#[test]
fn the_repo_is_found_as_a_run_finds_it_by_the_wishs_project_in_the_days_logs() {
    // In the system's own temporary folder, as F39's test of the same lookup is (`f39/refused.rs`).
    let made = finished_in(world_in(tempfile::tempdir().unwrap()));
    let logs = made.w.at("typed-logs");
    let show = || made.w.run(&["run", "show", "--wish", "m1", "--date", DAY, "--logs", logs.to_str().unwrap()]);
    let typed = support::said("2026-03-10T16:00:00.000Z", made.repo.to_str().unwrap(), "look at the uploader");
    support::write_log(&logs, "alpha/s.jsonl", &[typed]);
    assert_eq!(printed(&show()), printed(&made.show(&[])));
    // What git says of a folder read from a log is not repeated, and neither is the folder.
    made.git(&["update-ref", "-d", &made.branch()], b"", None);
    let said = show();
    assert!(refused(&said).contains("is not in the repo") && !said.stderr.contains("alpha"), "{}", said.stderr);
}

#[test]
fn each_file_is_listed_with_what_was_done_to_it_its_size_what_it_is_and_whether_tools_obey_it() {
    let made = finished();
    let text = format!("{WORK}\n").into_bytes();
    let bytes = [WORK.as_bytes(), &[0, 159, 146, 150]].concat();
    made.rework(&[
        ("100644", ".GitHub/Workflows/ci.yml", &text),
        ("100644", ".claude/settings.json", &text),
        ("120000", ".vscode", b"elsewhere/plum-otter-made-by-the-run"),
        ("100644", "a.txt", b"changed\n"),
        ("0", "b.txt", b""),
        ("100755", "bin/tool", &bytes),
        ("100644", "docs/CLAUDE.md", &text),
        ("100644", "docs/CLAUDE.md.txt", &text),
        ("100644", "docs/claude/notes.md", &text),
        ("100644", "latin.txt", &[b'c', b'a', b'f', 0xe9]),
        ("160000", "vendor/lib", b""),
        ("100644", "web/.envrc", &text),
    ]);
    let before = made.held();
    let shown = printed(&made.show(&[]));
    let size = text.len();
    let files = json!([
        listed(1, ".GitHub/Workflows/ci.yml", "made", size, "text", true),
        listed(2, ".claude/settings.json", "made", size, "text", true),
        listed(3, ".vscode", "made", 36, "link", true),
        listed(4, "a.txt", "changed", 8, "text", false),
        listed(5, "b.txt", "removed", 16, "text", false),
        listed(6, "bin/tool", "made", bytes.len(), "other", false),
        listed(7, "docs/CLAUDE.md", "made", size, "text", true),
        listed(8, "docs/CLAUDE.md.txt", "made", size, "text", false),
        listed(9, "docs/claude/notes.md", "made", size, "text", false),
        listed(10, "latin.txt", "made", 4, "other", false),
        listed(11, "vendor/lib", "made", 0, "repo", false),
        listed(12, "web/.envrc", "made", size, "text", true),
    ]);
    assert_eq!(shown["files"], files);
    assert_eq!((&shown["more"], &shown["work"]), (&json!(0), &json!("waits")));
    assert_eq!(made.held(), before);
    made.nothing_on_disk(&["ci.yml", "settings.json", ".vscode", "tool", "CLAUDE.md", "lib", ".envrc", "latin.txt"]);
}

#[test]
fn a_run_that_changed_nothing_has_no_files_and_work_the_record_says_was_kept_or_tossed_is_not_listed() {
    let w = world();
    w.play("nothing", 0);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (run, said) = ran(&w.start(&wish, &repo));
    assert!(said["why"].as_str().unwrap().contains("changed nothing"), "{said}");
    let made = Made { w, repo, wish, run };
    // The branch is where the person's checkout was: its last commit is theirs, and is not shown as the run's.
    let want = json!({"run": made.run, "wish": "m1", "work": "waits", "files": [], "more": 0, "most": 200});
    assert_eq!(printed(&made.show(&[])), want);
    assert!(refused(&made.show(&["--file", "1"])).contains("names no file"));

    let made = finished();
    let file = made.w.jm("runs").join(&made.run).join("run.json");
    for state in ["kept", "tossed"] {
        let mut record = made.w.record(&made.run);
        record["work"] = json!(state);
        fs::write(&file, format!("{record}\n")).unwrap();
        let want = json!({"run": made.run, "wish": "m1", "work": state, "files": [], "more": 0, "most": 200});
        assert_eq!(printed(&made.show(&[])), want);
        assert!(refused(&made.show(&["--file", "1"])).contains("names no file"));
    }
}
