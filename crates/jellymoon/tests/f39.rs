//! F39: a wish starts a run with the person's harness, and its star appears when the run reports it finished
//! (ADR 0014, `docs/development/run.md`). `jellymoon run` is run as a black box. The harness it finds is the made-up
//! `fake-agent` (`support/fake_agent.rs`), first on a `PATH` whose only other folder holds git alone, so the real
//! Claude Code is never started. Every repo is made here with the real `git`, under an empty global config, and both
//! homes and the harness's folder are scratch folders. `f39/failed.rs` holds the runs that do not finish and the
//! caps on the stream, `f39/refused.rs` what is refused or not started, `f39/limits.rs` the time limit, a run cut
//! off, and two runs at once.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "f39/failed.rs"]
mod failed;
#[path = "f39/limits.rs"]
mod limits;
#[path = "f39/refused.rs"]
mod refused;
mod support;

use std::env::consts::EXE_SUFFIX;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

use serde_json::{Value, json};
use support::{Ran, ZONE, jellymoon_in, plant, scratch, tree};
use tempfile::TempDir;

pub const DAY: &str = "2026-03-10";
pub const SAID: &str = "make the uploader retry twice";
/// What the made-up agent says in every text of its own. Jellymoon keeps and prints none of it.
pub const MARKER: &str = "zebra-quartz-said-by-the-agent";
pub const STAR: &str = "a run reported it finished";

/// The command line measured on Claude Code 2.1.322, and the one fixed text core adds to it.
const ARGS: [&str; 19] = [
    "-p",
    "--restricted",
    "--strict-mcp-config",
    "--permission-prompts",
    "none",
    "--permission-mode",
    "acceptEdits",
    "--tools",
    "Read",
    "Edit",
    "Write",
    "Glob",
    "Grep",
    "--no-session-persistence",
    "--output-format",
    "stream-json",
    "--verbose",
    "--append-system-prompt",
    "You are working in a copy of a repo that was made for this one wish. You can read and edit files in this \
     folder only. You cannot run commands. Make the change the wish asks for, then stop.",
];

/// One test's folder: `user` is the person's home, `jm` Jellymoon's, `harness` Claude Code's, `work` holds the
/// repos, `bin` the made-up agent under the harness's name, and `rec` what it recorded.
pub struct World {
    pub dir: TempDir,
    pub vars: Vec<(&'static str, OsString)>,
}

/// Every file under a folder with its bytes, as `support::tree` lists them.
type Files = Vec<(PathBuf, Vec<u8>)>;

/// A folder for `PATH` that holds git and no harness: a link to the real git in a folder of the test's own. Where
/// links cannot be made it is the real git's own folder, which must then hold no `claude`.
fn git_alone(folder: &Path) -> PathBuf {
    let name = format!("git{EXE_SUFFIX}");
    let path = env::var_os("PATH").unwrap();
    let mut on_path = env::split_paths(&path).map(|dir| dir.join(&name));
    let real = on_path.find(|git| git.is_absolute() && git.is_file()).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&real, folder.join(&name)).unwrap();
    let folder = if cfg!(unix) { folder } else { real.parent().unwrap() };
    let harness =
        fs::read_dir(folder).unwrap().any(|entry| entry.unwrap().file_name().to_string_lossy().starts_with("claude"));
    assert!(!harness, "{} holds a harness: these tests never start the real one", folder.display());
    folder.to_path_buf()
}

pub fn world() -> World {
    world_in(scratch())
}

pub fn world_in(dir: TempDir) -> World {
    let at = |name: &str| dir.path().join(name);
    for folder in ["user", "harness", "bin", "rec", "tmp", "git", "work", "logs"] {
        fs::create_dir(at(folder)).unwrap();
    }
    fs::write(at("gitconfig"), "").unwrap();
    plant(env!("CARGO_BIN_EXE_fake-agent"), &at("bin").join(format!("claude{EXE_SUFFIX}")));
    let path = env::join_paths([at("bin"), git_alone(&at("git"))]).unwrap();
    let vars = vec![
        ("TZ", ZONE.into()),
        ("HOME", at("user").into()),
        ("USERPROFILE", at("user").into()),
        ("JELLYMOON_HOME", at("jm").into()),
        ("CLAUDE_CONFIG_DIR", at("harness").into()),
        ("JELLYMOON_SYSTEM_BIN", OsString::new()), // no folder of this machine is looked in
        ("GIT_CONFIG_GLOBAL", at("gitconfig").into()),
        ("GIT_CONFIG_NOSYSTEM", "1".into()),
        ("PATH", path),
        ("TMPDIR", at("tmp").into()),
        ("USER", "tester".into()),
        ("LANG", "C".into()),
        ("LC_ALL", "C".into()),
        ("TERM", "dumb".into()),
    ];
    let world = World { dir, vars };
    world.play("edits", 0);
    world
}

/// The one line a command that worked printed, and nothing else.
pub fn printed(ran: &Ran) -> Value {
    assert_eq!((ran.code, ran.stderr.as_str()), (0, ""), "{}", ran.stdout);
    assert!(ran.stdout.ends_with("}\n") && ran.stdout.matches('\n').count() == 1, "{:?}", ran.stdout);
    serde_json::from_str(&ran.stdout).unwrap()
}

/// A refusal: one plain line, exit 1, nothing printed.
pub fn refused(ran: &Ran) -> &str {
    assert_eq!((ran.code, ran.stdout.as_str()), (1, ""), "{}", ran.stderr);
    assert!(ran.stderr.starts_with("jellymoon: ") && ran.stderr.matches('\n').count() == 1, "{:?}", ran.stderr);
    &ran.stderr
}

impl World {
    pub fn at(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    pub fn jm(&self, name: &str) -> PathBuf {
        self.at("jm").join(name)
    }

    /// Tells the made-up agent which script to play, and the number some scripts take.
    pub fn play(&self, script: &str, number: usize) {
        let rec = self.at("rec");
        fs::write(self.at("bin").join("play"), format!("{script}\n{}\n{number}\n", rec.display())).unwrap();
    }

    /// What the made-up agent recorded under a name. None when it was never started.
    pub fn rec(&self, name: &str) -> Option<String> {
        fs::read_to_string(self.at("rec").join(name)).ok()
    }

    /// How many times the made-up agent was started.
    pub fn starts(&self) -> usize {
        self.rec("starts").map_or(0, |starts| starts.lines().count())
    }

    /// Runs the real `git` in `folder`, as the person, and returns what it printed. git looks for a repo no higher
    /// than the test's folder, which is itself inside this repo's checkout.
    pub fn git(&self, folder: &Path, args: &[&str]) -> String {
        let mut git = Command::new("git");
        git.args(args).current_dir(folder).env_clear().envs(self.vars.iter().map(|(name, value)| (name, value)));
        git.env("GIT_CEILING_DIRECTORIES", self.dir.path());
        for who in ["AUTHOR", "COMMITTER"] {
            git.env(format!("GIT_{who}_NAME"), "Test Setup").env(format!("GIT_{who}_EMAIL"), "setup@example.test");
        }
        if let Some(root) = env::var_os("SystemRoot") {
            git.env("SystemRoot", root); // a Windows program needs it to start
        }
        let ran = git.output().unwrap();
        assert!(ran.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&ran.stderr));
        String::from_utf8(ran.stdout).unwrap().trim_end().to_string()
    }

    /// A repo of three files and one commit, on `main`, at `work/<path>`.
    pub fn repo(&self, path: &str) -> PathBuf {
        let repo = self.at("work").join(path);
        fs::create_dir_all(&repo).unwrap();
        self.git(&repo, &["init", "-q", "-b", "main"]);
        for file in ["a.txt", "b.txt", "c.txt"] {
            fs::write(repo.join(file), format!("{file} as it was\n")).unwrap();
        }
        self.git(&repo, &["add", "."]);
        self.git(&repo, &["commit", "-q", "-m", "the first commit"]);
        repo
    }

    /// `jellymoon`, started in the test's own folder, with `more` variables after the world's.
    pub fn run_with(&self, more: &[(&'static str, OsString)], args: &[&str]) -> Ran {
        let vars = self.vars.iter().chain(more).map(|(name, value)| (*name, value.as_os_str()));
        jellymoon_in(self.dir.path(), &vars.collect::<Vec<_>>(), args)
    }

    pub fn run(&self, args: &[&str]) -> Ran {
        self.run_with(&[], args)
    }

    /// Makes a wish for a project on the day, and returns its id.
    pub fn wish(&self, project: &str) -> String {
        let made = printed(&self.run(&["wish", "--date", DAY, "--say", SAID, "--project", project]));
        made["wish"]["id"].as_str().unwrap().to_string()
    }

    /// `jellymoon run` for a wish of the day, in the repo named.
    pub fn start(&self, wish: &str, repo: &Path) -> Ran {
        self.run(&["run", "--wish", wish, "--date", DAY, "--repo", repo.to_str().unwrap()])
    }

    /// A made wish as it is kept.
    pub fn kept(&self, wish: &str) -> Value {
        let kept = fs::read_to_string(self.jm("pages").join(format!("{DAY}.wishes.json"))).unwrap();
        let kept: Value = serde_json::from_str(&kept).unwrap();
        kept["wishes"].as_array().unwrap().iter().find(|kept| kept["id"] == wish).unwrap().clone()
    }

    /// A wish of the day as `jellymoon page` prints it, over logs that hold no session: no reading call is made.
    pub fn on_page(&self, wish: &str) -> Value {
        let logs = self.at("logs");
        let ran = self.run(&["page", "--date", DAY, "--logs", logs.to_str().unwrap()]);
        let page = printed(&ran);
        assert!(!ran.stdout.contains(MARKER));
        page["wishes"].as_array().unwrap().iter().find(|on| on["id"] == wish).unwrap().clone()
    }

    /// A run's record, `runs/<run>/run.json`.
    pub fn record(&self, run: &str) -> Value {
        serde_json::from_str(&fs::read_to_string(self.jm("runs").join(run).join("run.json")).unwrap()).unwrap()
    }

    pub fn commit(&self, repo: &Path, name: &str) -> String {
        self.git(repo, &["rev-parse", "--verify", &format!("{name}^{{commit}}")])
    }

    /// The run branches of a repo.
    pub fn branches(&self, repo: &Path) -> Vec<String> {
        let listed = self.git(repo, &["for-each-ref", "--format=%(refname:short)", "refs/heads/jellymoon"]);
        listed.lines().map(str::to_string).collect()
    }

    /// The copies that are kept, by run id.
    pub fn copies(&self) -> Vec<String> {
        let copies = fs::read_dir(self.jm("copies")).into_iter().flatten();
        copies.map(|entry| entry.unwrap().file_name().into_string().unwrap()).collect()
    }

    /// What is the person's in a repo: every file of the checkout, the index, `HEAD`, and every ref (the stash is
    /// one) that is not a run's branch. Nothing here makes git write: no status is asked for.
    pub fn persons(&self, repo: &Path) -> (Files, Vec<u8>, Vec<u8>, Vec<String>) {
        let files =
            tree(repo).into_iter().filter(|(path, _)| !path.components().any(|part| part.as_os_str() == ".git"));
        let refs = self.git(repo, &["for-each-ref", "--format=%(refname) %(objectname)"]);
        let refs = refs.lines().filter(|line| !line.starts_with("refs/heads/jellymoon/")).map(str::to_string);
        let git = repo.join(".git");
        (files.collect(), fs::read(git.join("index")).unwrap(), fs::read(git.join("HEAD")).unwrap(), refs.collect())
    }

    /// Leaves the person in the middle of something: a stash, a staged file, an edit not staged, a file git has
    /// never seen, and a name of their own for commits.
    pub fn work_in_progress(&self, repo: &Path) {
        fs::write(repo.join("a.txt"), "put away for later\n").unwrap();
        self.git(repo, &["stash", "-q"]);
        fs::write(repo.join("staged.txt"), "staged\n").unwrap();
        self.git(repo, &["add", "staged.txt"]);
        fs::write(repo.join("c.txt"), "half an edit\n").unwrap();
        fs::write(repo.join("note.txt"), "git has never seen this\n").unwrap();
        self.git(repo, &["config", "user.name", "Ada Example"]);
        self.git(repo, &["config", "user.email", "ada@example.test"]);
    }

    /// Fails if the agent's own words are in any file the test's folder holds (the agent's program aside), or in
    /// what a command printed.
    pub fn never_says_the_agent(&self, ran: &Ran) {
        assert!(!ran.stdout.contains(MARKER) && !ran.stderr.contains(MARKER), "{} {}", ran.stdout, ran.stderr);
        let found = |bytes: &[u8]| bytes.windows(MARKER.len()).any(|window| window == MARKER.as_bytes());
        for (path, bytes) in tree(self.dir.path()) {
            assert!(path.starts_with(self.at("bin")) || !found(&bytes), "{} holds the agent's words", path.display());
        }
    }
}

/// What a run printed, as its parts: the run's id, and the rest to compare.
pub fn ran(ran: &Ran) -> (String, Value) {
    let mut said = printed(ran);
    assert_eq!(said.as_object().unwrap().len(), 6, "{said}");
    let run = said["run"].as_str().unwrap().to_string();
    let shaped = run.len() == 17 && run.starts_with('r') && run[1..].bytes().all(|b| b.is_ascii_hexdigit());
    assert!(shaped, "{run}");
    said.as_object_mut().unwrap().remove("run");
    (run, said)
}

#[test]
fn the_agent_is_started_with_the_measured_command_line_in_the_copy_and_the_wish_on_its_input() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (run, _) = ran(&w.start(&wish, &repo));
    let args: Vec<String> = (0..).map_while(|n| w.rec(&format!("arg{n}"))).collect();
    assert_eq!(args, ARGS, "the arguments, in order");
    assert_eq!(w.rec("stdin").unwrap(), format!("{SAID}\n"), "the wish's words go in on the input");
    assert!(args.iter().all(|arg| !arg.contains(SAID) && !arg.contains("uploader")), "and in no argument");
    let copy = fs::canonicalize(w.jm("copies")).unwrap().join(&run);
    assert_eq!(fs::canonicalize(w.rec("cwd").unwrap()).unwrap_or_else(|_| w.rec("cwd").unwrap().into()), copy);
    assert_eq!(w.starts(), 1);
}

#[test]
fn the_agent_sees_only_the_allow_listed_variables() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    // Names shaped like the ones a token is kept under, joined here so no scanner reads one in this file.
    let planted = [format!("GH_{}", "TOKEN"), format!("ANTHROPIC_{}_{}", "API", "KEY"), "GIT_DIR".to_string()];
    let planted: Vec<&'static str> = planted.into_iter().map(|name| &*name.leak()).collect();
    let more: Vec<(&'static str, OsString)> = planted.iter().map(|name| (*name, "planted".into())).collect();
    let args = ["run", "--wish", &wish, "--date", DAY, "--repo", repo.to_str().unwrap()];
    assert_eq!(ran(&w.run_with(&more, &args)).1["outcome"], "completed");
    let seen = w.rec("env").unwrap();
    let seen: Vec<&str> = seen.lines().collect();
    assert!(planted.iter().all(|name| !seen.contains(name)), "{seen:?}");
    if cfg!(windows) {
        let allowed = "PATH PATHEXT SystemRoot SystemDrive ComSpec USERPROFILE HOMEDRIVE HOMEPATH APPDATA \
                       LOCALAPPDATA TEMP TMP USERNAME CLAUDE_CONFIG_DIR";
        let allowed: Vec<String> = allowed.split(' ').map(str::to_lowercase).collect();
        assert!(seen.iter().all(|name| allowed.contains(&name.to_lowercase())), "{seen:?}");
    } else {
        assert_eq!(seen, ["CLAUDE_CONFIG_DIR", "HOME", "LANG", "LC_ALL", "PATH", "TERM", "TMPDIR", "USER"]);
    }
}

#[test]
fn a_finished_run_adds_one_star_and_leaves_its_work_as_one_commit_on_its_branch() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    w.work_in_progress(&repo);
    let (base, before) = (w.commit(&repo, "HEAD"), w.persons(&repo));

    let done = w.start(&wish, &repo);
    let (run, said) = ran(&done);
    let branch = format!("jellymoon/run-{run}");
    let why = format!("run {run} finished: its work is one commit on the branch {branch}");
    assert_eq!(said, json!({"wish": wish, "outcome": "completed", "star": STAR, "denied": 0, "why": why}));

    // The branch holds exactly the agent's edits, as one commit on the commit the checkout was on.
    let log = w.git(&repo, &["log", "-1", "--format=%an <%ae>|%s|%P", "--name-status", &branch]);
    let log: Vec<&str> = log.lines().filter(|line| !line.is_empty()).collect();
    let first = format!("Ada Example <ada@example.test>|Jellymoon run {run}|{base}");
    assert_eq!(log, [first.as_str(), "M\ta.txt", "D\tb.txt", "A\tn.txt"]);
    assert_eq!(w.git(&repo, &["show", &format!("{branch}:a.txt")]), "edited by the run");
    // Nothing of the person's is changed, the copy's folder is gone, and only the run's branch is new.
    assert_eq!(w.persons(&repo), before);
    assert_eq!((w.copies(), w.branches(&repo)), (vec![], vec![branch.clone()]));
    assert_eq!(w.git(&repo, &["worktree", "list", "--porcelain"]).matches("worktree ").count(), 1);

    let kept =
        json!({"id": wish, "said": SAID, "project": "alpha", "state": "done", "stars": [STAR], "run": run, "why": why});
    assert_eq!(w.kept(&wish), kept);
    let on_page = w.on_page(&wish);
    assert_eq!((&on_page["state"], &on_page["stars"]), (&json!("done"), &json!([STAR])), "the page shows the star");
    let record = w.record(&run);
    assert_eq!((&record["wish"], &record["day"], &record["outcome"]), (&json!(wish), &json!(DAY), &json!("completed")));
    assert_eq!((&record["denied"], &record["skipped"]), (&json!(0), &json!(0)));
    w.never_says_the_agent(&done);
}

#[test]
fn a_run_that_changed_nothing_is_still_finished_and_says_so() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    w.play("nothing", 0);
    let (run, said) = ran(&w.start(&wish, &repo));
    let branch = format!("jellymoon/run-{run}");
    let why = format!("run {run} finished and changed nothing: the branch {branch} is where the checkout was");
    assert_eq!(said, json!({"wish": wish, "outcome": "completed", "star": STAR, "denied": 0, "why": why}));
    assert_eq!(w.commit(&repo, &branch), w.commit(&repo, "HEAD"));
    assert_eq!((w.copies(), &w.kept(&wish)["state"]), (vec![], &json!("done")));
}

// The stream is untrusted: what is not a line of it is counted and skipped, and what was denied is only counted.
#[test]
fn lines_that_are_not_the_stream_s_are_skipped_and_denials_are_counted() {
    let w = world();
    let repo = w.repo("alpha");
    w.play("garbage", 0);
    let garbage = w.start(&w.wish("alpha"), &repo);
    let (run, said) = ran(&garbage);
    assert_eq!((&said["outcome"], &said["denied"]), (&json!("completed"), &json!(0)));
    assert_eq!(w.record(&run)["skipped"], 8, "eight lines that are not the stream's");
    w.never_says_the_agent(&garbage);

    w.play("denials", 2);
    let denied = w.start(&w.wish("alpha"), &repo);
    let (run, said) = ran(&denied);
    assert_eq!((&said["outcome"], &said["denied"], &said["star"]), (&json!("completed"), &json!(2), &json!(STAR)));
    assert_eq!(w.record(&run)["denied"], 2);
    w.never_says_the_agent(&denied);
}
