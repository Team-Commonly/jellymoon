//! What the tests of a run share (F39, F40): one test's folder with the made-up agent in it under the harness's
//! name, a repo made with the real `git`, and `jellymoon` started in it. The made-up agent holds the system's lock
//! on its `held` file for as long as it lives: a test that can take the lock knows the agent is gone, with no clock
//! in the proof.
#![allow(dead_code)]

use std::env::consts::EXE_SUFFIX;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use std::{env, fs, thread};

use serde_json::Value;
use tempfile::TempDir;

use crate::support::{Ran, ZONE, jellymoon_in, plant, scratch, tree};

pub const DAY: &str = "2026-03-10";
pub const SAID: &str = "make the uploader retry twice";
/// What the made-up agent says in every text of its own. Jellymoon keeps and prints none of it.
pub const MARKER: &str = "zebra-quartz-said-by-the-agent";
pub const STAR: &str = "a run reported it finished";
/// The lines a wish keeps of a run that was never started: said by its command, or by the look that found it.
pub const OLD: &str = "nothing was started: this Claude Code is too old for runs, as it does not know --restricted; \
                       update Claude Code and run the wish again";
pub const UNTOLD: &str = "nothing was started: Claude Code is too old for runs or could not be started; run the \
                          wish again when that is mended";

/// The command line measured on Claude Code 2.1.322, and the one fixed text core adds to it.
pub const ARGS: [&str; 19] = [
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
/// A made wish as a run that was never started leaves it: it names no run, says why and needs the person.
pub fn never(mut made: Value, why: &str) -> Value {
    (made["state"], made["why"]) = ("needs_you".into(), why.into());
    made
}

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

impl World {
    /// Whether the made-up agent got as far as its wait, and is gone now.
    pub fn agent_came_and_went(&self) -> bool {
        self.at("rec").join("started").is_file() && self.free("held")
    }

    /// Whether nothing holds the system's lock on a file the made-up agent, or what it left, locks while it lives.
    pub fn free(&self, held: &str) -> bool {
        fs::File::open(self.at("rec").join(held)).unwrap().try_lock().is_ok()
    }

    /// Waits for a lock of the made-up agent's to be free, half a minute at most: a kill is not felt at once. The
    /// first time it is free is the answer, and it is not asked again. To ask is to take the lock and let it go, and
    /// a command that another test starts at that moment has this one's open files, the lock with them, until it is
    /// its own program: asked again just then, the lock is found held with nothing of the agent's left.
    pub fn freed(&self, held: &str) -> bool {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if self.free(held) {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    /// Leaves the file the made-up agent waits for, and waits until the agent is gone, half a minute at most.
    pub fn let_end(&self, file: &str) {
        fs::write(self.at("rec").join(file), "").unwrap();
        let deadline = Instant::now() + Duration::from_secs(30);
        while !self.agent_came_and_went() {
            assert!(Instant::now() < deadline, "the agent left behind never ended");
            thread::sleep(Duration::from_millis(10));
        }
    }

    /// Starts `jellymoon run` and returns at once, with the command still going.
    pub fn start_going(&self, wish: &str, repo: &Path) -> Child {
        self.going(&[], &["run", "--wish", wish, "--date", DAY, "--repo", repo.to_str().unwrap()])
    }

    /// Starts `jellymoon` with `more` variables after the world's and returns at once. What it prints is kept for
    /// [`ended`]: one line, which never fills a pipe.
    pub fn going(&self, more: &[(&'static str, OsString)], args: &[&str]) -> Child {
        let mut command = Command::new(env!("CARGO_BIN_EXE_jellymoon"));
        let vars = self.vars.iter().chain(more).map(|(name, value)| (name, value));
        command.args(args).current_dir(self.dir.path()).env_clear().envs(vars);
        if let Some(root) = std::env::var_os("SystemRoot").filter(|_| cfg!(windows)) {
            command.env("SystemRoot", root); // a Windows program needs it to start
        }
        command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap()
    }
}

/// What a command started with [`World::going`] printed, once it has ended: half a minute is waited at most.
pub fn ended(mut going: Child) -> Ran {
    let deadline = Instant::now() + Duration::from_secs(30);
    while going.try_wait().unwrap().is_none() {
        if Instant::now() > deadline {
            going.kill().unwrap();
            panic!("jellymoon was still going after half a minute");
        }
        thread::sleep(Duration::from_millis(5));
    }
    let done = going.wait_with_output().unwrap();
    let text = |bytes: Vec<u8>| String::from_utf8(bytes).unwrap();
    Ran { stdout: text(done.stdout), stderr: text(done.stderr), code: done.status.code().unwrap_or(-1) }
}

/// Waits for a file the made-up agent leaves, half a minute at most.
pub fn wait_for(file: &Path) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !file.exists() {
        assert!(Instant::now() < deadline, "{} never came", file.display());
        thread::sleep(Duration::from_millis(10));
    }
}

pub fn grows(file: &Path) -> bool {
    let size = || fs::metadata(file).map_or(0, |meta| meta.len());
    let before = size();
    thread::sleep(Duration::from_millis(400));
    size() > before
}
