//! F38: a run gets its own copy of the repo. `jellymoon copy make` and `jellymoon copy clean` are run as a black box
//! on repos this test makes with the real `git`, in a folder of its own. No test reads this machine's git config or
//! touches a real repo: `GIT_CONFIG_GLOBAL` is an empty file, `GIT_CONFIG_NOSYSTEM` is set, and both homes are scratch
//! folders. `f38/hostile.rs` holds what an agent could have done to its copy, and a repo whose config names programs.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "f38/hostile.rs"]
mod hostile;
#[path = "f38/limits.rs"]
mod limits;
mod support;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

use serde_json::Value;
use support::{Ran, jellymoon_in, scratch, tree};
use tempfile::TempDir;

/// One test's folder: `user` is the person's home, `jm` Jellymoon's, and the repos are beside them.
pub struct World {
    pub dir: TempDir,
    pub vars: Vec<(&'static str, OsString)>,
}

/// Every file under a folder with its bytes, as `support::tree` lists them.
type Files = Vec<(PathBuf, Vec<u8>)>;

/// What `copy make` and `copy clean` print.
#[derive(Debug)]
pub struct Printed {
    pub run: String,
    pub folder: PathBuf,
    pub branch: String,
    pub kept: bool,
}

pub fn world() -> World {
    let dir = scratch();
    let at = |name: &str| dir.path().join(name);
    fs::create_dir(at("user")).unwrap();
    fs::write(at("gitconfig"), "").unwrap();
    let vars = vec![
        ("HOME", at("user").into()),
        ("USERPROFILE", at("user").into()),
        ("JELLYMOON_HOME", at("jm").into()),
        ("GIT_CONFIG_GLOBAL", at("gitconfig").into()),
        ("GIT_CONFIG_NOSYSTEM", "1".into()),
        ("PATH", env::var_os("PATH").unwrap()),
    ];
    World { dir, vars }
}

/// The one line a command that worked printed, and nothing else.
pub fn said(ran: &Ran) -> Printed {
    assert_eq!((ran.code, ran.stderr.as_str()), (0, ""), "{}", ran.stdout);
    assert!(ran.stdout.ends_with("}\n") && ran.stdout.matches('\n').count() == 1, "{:?}", ran.stdout);
    let line: Value = serde_json::from_str(&ran.stdout).unwrap();
    assert_eq!(line.as_object().unwrap().len(), 4, "{line}");
    let text = |key: &str| line[key].as_str().unwrap().to_string();
    Printed { run: text("run"), folder: text("folder").into(), branch: text("branch"), kept: line["kept"] == true }
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

    /// Runs the real `git` in `folder`, as the person, and returns what it printed. The commits the test makes
    /// itself carry a made-up name given here: the person's config names nobody unless a test says so. git looks
    /// for a repo no higher than the test's folder, which is itself inside this repo's checkout.
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

    /// A repo of three files and one commit, on `main`.
    pub fn repo(&self, name: &str) -> PathBuf {
        let repo = self.at(name);
        fs::create_dir(&repo).unwrap();
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

    pub fn make(&self, repo: &Path) -> Printed {
        said(&self.run(&["copy", "make", "--repo", repo.to_str().unwrap()]))
    }

    pub fn clean(&self, run: &str, outcome: &str) -> Ran {
        self.run(&["copy", "clean", "--run", run, "--outcome", outcome])
    }

    /// The commit a name stands for.
    pub fn commit(&self, repo: &Path, name: &str) -> String {
        self.git(repo, &["rev-parse", "--verify", &format!("{name}^{{commit}}")])
    }

    /// How many working trees the repo lists: the person's checkout and every copy.
    pub fn listed(&self, repo: &Path) -> usize {
        self.git(repo, &["worktree", "list", "--porcelain"])
            .lines()
            .filter(|line| line.starts_with("worktree "))
            .count()
    }

    /// What is the person's in a repo: every file of the checkout, the index, `HEAD`, and every ref (the stash is
    /// one) but the run branches named. Nothing here makes git write: no status is asked for.
    fn persons(&self, repo: &Path, runs: &[&Printed]) -> (Files, Vec<u8>, Vec<u8>, Vec<String>) {
        let files =
            tree(repo).into_iter().filter(|(path, _)| !path.components().any(|part| part.as_os_str() == ".git"));
        let refs = self.git(repo, &["for-each-ref", "--format=%(refname) %(objectname)"]);
        let ours = |line: &&str| runs.iter().any(|run| line.starts_with(&format!("refs/heads/{} ", run.branch)));
        let refs = refs.lines().filter(|line| !ours(line)).map(str::to_string).collect();
        let git = repo.join(".git");
        (files.collect(), fs::read(git.join("index")).unwrap(), fs::read(git.join("HEAD")).unwrap(), refs)
    }

    /// Leaves the person in the middle of something: a stash, a staged file, an edit not staged, a file git has
    /// never seen, and a name of their own for commits.
    fn work_in_progress(&self, repo: &Path) {
        fs::write(repo.join("a.txt"), "put away for later\n").unwrap();
        self.git(repo, &["stash", "-q"]);
        fs::write(repo.join("staged.txt"), "staged\n").unwrap();
        self.git(repo, &["add", "staged.txt"]);
        fs::write(repo.join("c.txt"), "half an edit\n").unwrap();
        fs::write(repo.join("note.txt"), "git has never seen this\n").unwrap();
        self.git(repo, &["config", "user.name", "Ada Example"]);
        self.git(repo, &["config", "user.email", "ada@example.test"]);
    }
}

#[test]
fn a_completed_run_leaves_its_work_as_one_commit_on_its_branch_and_the_checkout_as_it_was() {
    let w = world();
    let repo = w.repo("alpha");
    w.work_in_progress(&repo);
    let (base, before) = (w.commit(&repo, "HEAD"), w.persons(&repo, &[]));

    let made = w.make(&repo);
    assert_eq!(made.folder, w.at("jm").join("copies").join(&made.run));
    assert_eq!((made.branch.as_str(), made.kept), (format!("jellymoon/run-{}", made.run).as_str(), true));
    assert_eq!(w.persons(&repo, &[&made]), before, "after make");
    assert_eq!((w.commit(&repo, &made.branch), w.listed(&repo)), (base.clone(), 2));
    // The copy is the commit: nothing the person had not committed is in it.
    assert_eq!(fs::read_to_string(made.folder.join("c.txt")).unwrap(), "c.txt as it was\n");
    assert!(!made.folder.join("note.txt").exists() && !made.folder.join("staged.txt").exists());

    fs::write(made.folder.join("a.txt"), "edited by the run\n").unwrap();
    fs::write(made.folder.join("n.txt"), "new from the run\n").unwrap();
    fs::remove_file(made.folder.join("b.txt")).unwrap();
    let cleaned = said(&w.clean(&made.run, "completed"));
    assert_eq!((&cleaned.run, &cleaned.folder, &cleaned.branch), (&made.run, &made.folder, &made.branch));
    assert!(!cleaned.kept && !made.folder.exists() && w.listed(&repo) == 1);
    assert_eq!(w.persons(&repo, &[&made]), before, "after clean");

    let log = w.git(&repo, &["log", "-1", "--format=%an <%ae>|%s|%P", "--name-status", &made.branch]);
    let log: Vec<&str> = log.lines().filter(|line| !line.is_empty()).collect();
    let first = format!("Ada Example <ada@example.test>|Jellymoon run {}|{base}", made.run);
    assert_eq!(log, [first.as_str(), "M\ta.txt", "D\tb.txt", "A\tn.txt"]);
    assert_eq!(w.git(&repo, &["show", &format!("{}:a.txt", made.branch)]), "edited by the run");

    // Cleaned once: the run has no copy any more, and its id is not given again while its branch holds its work.
    assert!(refused(&w.clean(&made.run, "completed")).contains("no copy"));
    let again = w.run(&["copy", "make", "--repo", repo.to_str().unwrap(), "--run", &made.run]);
    assert!(refused(&again).contains("has its branch already") && w.persons(&repo, &[&made]) == before);
}

// `copy clean` takes the branch where it is, and it must be a plain branch at a commit: nothing is committed
// through a name for another branch, and none onto a branch that is gone.
#[test]
fn a_completed_copy_whose_branch_is_gone_or_is_a_name_for_another_branch_is_refused_and_kept() {
    let w = world();
    let repo = w.repo("alpha");
    let main = w.commit(&repo, "main");
    let line = "jellymoon: the run's branch is not a branch at a commit, so nothing was committed\n";
    for symbolic in [false, true] {
        let made = w.make(&repo);
        fs::write(made.folder.join("n.txt"), "new from the run\n").unwrap();
        let name = format!("refs/heads/{}", made.branch);
        let by_hand: &[&str] =
            if symbolic { &["symbolic-ref", &name, "refs/heads/main"] } else { &["update-ref", "-d", &name] };
        w.git(&repo, by_hand);
        let before = w.git(&repo, &["for-each-ref", "--format=%(refname) %(objectname) %(symref)"]);
        assert_eq!(refused(&w.clean(&made.run, "completed")), line, "symbolic: {symbolic}");
        assert_eq!(w.commit(&repo, "refs/heads/main"), main, "the person's branch took no commit");
        assert_eq!(w.git(&repo, &["for-each-ref", "--format=%(refname) %(objectname) %(symref)"]), before);
        assert_eq!(fs::read_to_string(made.folder.join("n.txt")).unwrap(), "new from the run\n", "the copy is kept");
        assert!(refused(&w.clean(&made.run, "completed")).contains("not a branch at a commit"), "it is still kept");
    }
}

#[test]
fn a_run_that_changed_nothing_is_cleaned_with_no_commit() {
    let w = world();
    let repo = w.repo("alpha");
    let made = w.make(&repo);
    assert!(!said(&w.clean(&made.run, "completed")).kept);
    assert_eq!(w.commit(&repo, &made.branch), w.commit(&repo, "HEAD"));
    assert!(!made.folder.exists() && w.listed(&repo) == 1);
}

#[test]
fn two_copies_of_one_repo_do_not_see_each_other() {
    let w = world();
    let repo = w.repo("alpha");
    let (one, two) = (w.make(&repo), w.make(&repo));
    assert!(one.run != two.run && one.folder != two.folder && one.branch != two.branch);
    fs::write(one.folder.join("a.txt"), "from one\n").unwrap();
    fs::write(two.folder.join("only-two.txt"), "from two\n").unwrap();
    assert_eq!(fs::read_to_string(two.folder.join("a.txt")).unwrap(), "a.txt as it was\n");
    assert!(!one.folder.join("only-two.txt").exists());
    for copy in [&one, &two] {
        said(&w.clean(&copy.run, "completed"));
    }
    let changed = |copy: &Printed| w.git(&repo, &["diff", "--name-only", "main", &copy.branch]);
    assert_eq!((changed(&one).as_str(), changed(&two).as_str()), ("a.txt", "only-two.txt"));
}

#[test]
fn of_seven_failed_runs_the_newest_five_copies_stay() {
    let w = world();
    let repo = w.repo("alpha");
    let made: Vec<Printed> = (0..7).map(|_| w.make(&repo)).collect();
    for copy in &made {
        fs::write(copy.folder.join("left.txt"), "for a look\n").unwrap();
        let cleaned = said(&w.clean(&copy.run, "failed"));
        assert!(cleaned.kept && cleaned.folder == copy.folder);
    }
    let there: Vec<bool> = made.iter().map(|copy| copy.folder.join("left.txt").exists()).collect();
    assert_eq!(there, [false, false, true, true, true, true, true]);
    assert!(!made[0].folder.exists() && !made[1].folder.exists());
    let listed = w.git(&repo, &["worktree", "list", "--porcelain"]);
    assert_eq!(w.listed(&repo), 6, "the checkout and five copies: {listed}");
    assert!(!listed.contains(&made[0].run) && !listed.contains(&made[1].run) && listed.contains(&made[2].run));
    // A failed run ended: it is not cleaned a second time.
    assert!(refused(&w.clean(&made[6].run, "completed")).contains("ended"));
}

#[test]
fn a_run_id_that_is_not_one_of_core_s_own_is_refused_and_nothing_is_written() {
    let w = world();
    let repo = w.repo("alpha");
    let made = w.make(&repo);
    let (elsewhere, longer) = (w.at("elsewhere"), format!("{}0", made.run));
    let (upper, spaced) = (made.run.to_uppercase(), format!("{} ", made.run));
    let ids = [
        "..",
        ".",
        "",
        elsewhere.to_str().unwrap(),
        "a/b",
        "a\\b",
        "../alpha",
        &made.run[1..],
        &longer,
        &upper,
        &spaced,
    ];
    let before = tree(w.dir.path());
    for id in ids {
        for ran in [
            w.run(&["copy", "make", "--repo", repo.to_str().unwrap(), "--run", id]),
            w.clean(id, "completed"),
            w.clean(id, "failed"),
        ] {
            assert!(refused(&ran).contains("run id"), "{id:?}: {}", ran.stderr);
        }
    }
    assert_eq!(tree(w.dir.path()), before);

    // One of the right shape is taken as given, once.
    let given = ["copy", "make", "--repo", repo.to_str().unwrap(), "--run", "r00000000000000a1"];
    assert_eq!(said(&w.run(&given)).run, "r00000000000000a1");
    assert!(refused(&w.run(&given)).contains("already"));
}

#[test]
fn what_cannot_be_copied_is_said_in_one_plain_line() {
    let w = world();
    let (repo, plain, bare, unborn) = (w.repo("alpha"), w.at("plain"), w.at("bare.git"), w.at("unborn"));
    for folder in [&plain, &bare, &unborn] {
        fs::create_dir(folder).unwrap();
    }
    w.git(&bare, &["init", "-q", "--bare"]);
    w.git(&unborn, &["init", "-q", "-b", "main"]);
    let said_bare = w.repo("said-bare");
    w.git(&said_bare, &["config", "core.bare", "true"]);
    fs::create_dir(w.at("empty")).unwrap();
    let no_git = [("PATH", OsString::from(w.at("empty")))];
    let make = |repo: &str, more: &[(&'static str, OsString)]| w.run_with(more, &["copy", "make", "--repo", repo]);
    let cases = [
        // The test's folder is inside this repo's own checkout: a plain folder there is not this repo.
        (make(plain.to_str().unwrap(), &[]), "is not a git repo"),
        (make(bare.to_str().unwrap(), &[]), "bare"),
        (make(unborn.to_str().unwrap(), &[]), "no commit"),
        (make(w.at("nowhere").to_str().unwrap(), &[]), "is not a git repo"),
        (make("alpha", &[]), "full path"),
        (make(repo.to_str().unwrap(), &no_git), "no `git` program"),
        // A checkout whose own config says it is bare is one git answers for as a bare repo.
        (make(said_bare.to_str().unwrap(), &[]), "bare repo or a repo's own git folder: a run needs a checkout"),
    ];
    for (ran, why) in &cases {
        assert!(refused(ran).contains(why), "{why}: {}", ran.stderr);
    }
    let copies = fs::read_dir(w.at("jm").join("copies")).map_or(0, Iterator::count);
    assert_eq!((copies, w.listed(&repo), w.git(&repo, &["branch", "--list"]).as_str()), (0, 1, "* main"));

    let id = "r00000000000000a1";
    for args in [
        &["copy"][..],
        &["copy", "mend"],
        &["copy", "make"],
        &["copy", "clean", "--run", id],
        &["copy", "clean", "--outcome", "failed"],
        &["copy", "clean", "--run", id, "--outcome", "done"],
        &["copy", "make", "--repo", "x", "--outcome", "failed"],
    ] {
        let ran = w.run(args);
        assert!(ran.code == 2 && ran.stdout.is_empty() && ran.stderr.contains("usage:"), "{args:?}: {}", ran.stderr);
    }
}
