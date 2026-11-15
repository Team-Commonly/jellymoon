//! F62: Jellymoon keeps a list of the projects it knows, and offers them (ADR 0020, decisions 1 and 2;
//! `docs/development/project.md`). `jellymoon project`, `run`, `wish --run` and `hear` are run as black boxes in
//! F39's test folder (`f39/world.rs`), with the made-up `fake-agent` as the harness, so the real Claude Code is
//! never started. Every world is in the system's own temporary folder, as F41's are. This file holds `project
//! add`, `list` and `forget`; `f62/refused.rs` what `add` refuses and two changes at once; `f62/started.rs` the
//! list a run's start writes; `f62/found.rs` a listed project found on a day nothing was typed, and one that is
//! gone, no repo's top, not the person's own or switched off; `f62/clash.rs` a name both typed in and listed;
//! `f62/hand.rs` a list written by hand.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "f62/clash.rs"]
mod clash;
#[path = "f62/found.rs"]
mod found;
#[path = "f62/hand.rs"]
mod hand;
#[path = "f55/heard.rs"]
mod heard;
#[path = "f62/refused.rs"]
mod refused;
#[path = "f62/started.rs"]
mod started;
mod support;
#[path = "f39/world.rs"]
mod world;

use std::fs;
use std::path::{Path, PathBuf};

pub use heard::*;
use serde_json::{Value, json};
use support::{Ran, tree};
pub use world::*;

pub const LATER: &str = "2026-03-11";
/// Every sentence a run says of a listed folder ends so: the two ways out.
pub const WAY_OUT: &str = "name one with --repo, or let it go with `jellymoon project forget`";

/// What a run says of a listed folder it does not start in.
pub fn fallen(why: &str) -> String {
    format!("the folder listed under the name the wish gives is not one a run can start in: {why}; {WAY_OUT}")
}

/// What `project list`, `add` and `forget` say of a list that is not as Jellymoon writes one.
pub fn unused(why: &str) -> String {
    format!(
        "jellymoon: the list of projects (projects.json in Jellymoon's folder) is not used: {why}; move the file \
         away and add your projects again\n"
    )
}

impl World {
    /// A folder as the list holds it: by its real path, every link in it followed.
    pub fn real(&self, folder: &Path) -> String {
        let real = fs::canonicalize(folder).unwrap();
        real.to_str().unwrap().trim_start_matches(r"\\?\").to_string()
    }

    pub fn list_file(&self) -> PathBuf {
        self.jm("projects.json")
    }

    /// `jellymoon project add` for a folder, used on a day.
    pub fn add_on(&self, repo: &Path, day: &str) -> Ran {
        self.run(&["project", "add", "--repo", repo.to_str().unwrap(), "--date", day])
    }

    pub fn add(&self, repo: &Path) -> Ran {
        self.add_on(repo, DAY)
    }

    /// What `jellymoon project list` prints: its projects.
    pub fn listed(&self) -> Value {
        printed(&self.run(&["project", "list"]))["projects"].clone()
    }

    /// The list as it is on disk: the bytes, which must be one line of JSON.
    pub fn on_disk(&self) -> Value {
        let text = fs::read_to_string(self.list_file()).unwrap();
        assert!(text.ends_with("}\n") && text.matches('\n').count() == 1, "{text:?}");
        serde_json::from_str(&text).unwrap()
    }

    /// Writes the list by hand, for the person alone, as anyone who can write Jellymoon's home can.
    pub fn write_list(&self, bytes: &[u8]) {
        fs::create_dir_all(self.at("jm")).unwrap();
        let _ = fs::remove_file(self.list_file());
        fs::write(self.list_file(), bytes).unwrap();
        #[cfg(unix)]
        fs::set_permissions(self.list_file(), std::os::unix::fs::PermissionsExt::from_mode(0o600)).unwrap();
    }

    /// One line of the list, as Jellymoon writes it.
    pub fn line(&self, name: &str, folder: &str) -> Value {
        json!({"name": name, "folder": folder, "used": DAY})
    }

    /// The whole test folder but the made-up agent's own, every file with its bytes.
    pub fn all(&self) -> Vec<(PathBuf, Vec<u8>)> {
        tree(self.dir.path()).into_iter().filter(|(path, _)| !path.starts_with(self.at("rec"))).collect()
    }

    /// Makes a wish for `project` on the day and asks for its run with no folder named: what `run` printed.
    pub fn run_found(&self, project: &str, more: &[&str]) -> Ran {
        let wish = self.wish(project);
        self.run(&[&["run", "--wish", &wish, "--date", DAY][..], more].concat())
    }

    /// Nothing of a run is in Jellymoon's home, no branch is in `repo`, and the agent was never started.
    pub fn started_nothing(&self, repo: &Path) {
        let runs = fs::read_dir(self.jm("runs")).into_iter().flatten();
        let runs = runs.map(|entry| entry.unwrap().file_name().into_string().unwrap());
        let runs: Vec<String> = runs.filter(|name| name != "no-hooks" && !name.contains("lock")).collect();
        assert_eq!((self.copies(), runs, self.starts()), (vec![], vec![], 0));
        assert!(!repo.join(".git").is_dir() || self.branches(repo).is_empty(), "a branch was left in the repo");
    }

    /// What `jellymoon hear` shows for a hold of the usual words whose four seconds have passed, with `more`
    /// arguments: its last line.
    pub fn heard_for(&self, more: &[&str]) -> Value {
        let mut events = hold(0, SAID);
        events.push(tick(4999));
        self.hear(more, &events).pop().unwrap()
    }
}

/// No line a command printed names a folder: not the test's own, and so none below it.
pub fn names_no_path(w: &World, ran: &Ran) {
    for folder in [w.dir.path().to_str().unwrap().to_string(), w.real(w.dir.path())] {
        assert!(!ran.stdout.contains(&folder) && !ran.stderr.contains(&folder), "{} {}", ran.stdout, ran.stderr);
    }
    let below = format!("{}work", std::path::MAIN_SEPARATOR);
    assert!(!ran.stdout.contains(&below) && !ran.stderr.contains(&below), "{} {}", ran.stdout, ran.stderr);
}

#[test]
fn add_lists_a_repo_s_top_folder_under_its_last_name_and_list_prints_names_and_days_and_never_a_path() {
    let w = heard_world();
    let (alpha, beta) = (w.repo("alpha"), w.repo("deep/beta"));
    assert_eq!(w.listed(), json!([]), "with no list there are no projects");
    assert!(!w.at("jm").exists(), "a look at the list made something");
    let persons = w.persons(&alpha);

    let added = w.add(&alpha);
    assert_eq!(printed(&added), json!({"name": "alpha", "used": DAY}));
    names_no_path(&w, &added);
    // The file's format: one line, the name, the folder by its real path, the day.
    assert_eq!(w.on_disk(), json!({"projects": [w.line("alpha", &w.real(&alpha))]}));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(fs::metadata(w.list_file()).unwrap().permissions().mode() & 0o777, 0o600);
    }
    assert!(!w.jm("projects.lock").exists(), "the lock was left");

    // A folder named with a `..` and a slash at its end is the same folder, and is listed by where it leads.
    let around = alpha.join("..").join("deep").join("beta").join("");
    assert_eq!(printed(&w.add_on(&around, LATER)), json!({"name": "beta", "used": LATER}));
    let list = w.run(&["project", "list"]);
    names_no_path(&w, &list);
    assert_eq!(printed(&list), json!({"projects": [{"name": "alpha", "used": DAY}, {"name": "beta", "used": LATER}]}));
    assert_eq!(w.on_disk()["projects"][1], json!({"name": "beta", "folder": w.real(&beta), "used": LATER}));

    // Added again, a project is the same line: its day moves on, and never back.
    assert_eq!(printed(&w.add_on(&beta, DAY)), json!({"name": "beta", "used": LATER}));
    assert_eq!(printed(&w.add_on(&alpha, LATER)), json!({"name": "alpha", "used": LATER}));
    assert_eq!(w.listed(), json!([{"name": "alpha", "used": LATER}, {"name": "beta", "used": LATER}]));
    // Nothing of the person's was written, and nothing was started.
    assert!(w.persons(&alpha) == persons && w.starts() == 0);
    w.started_nothing(&alpha);
}

#[test]
fn forget_removes_the_line_and_touches_nothing_else() {
    let w = heard_world();
    let (alpha, beta) = (w.repo("alpha"), w.repo("beta"));
    for repo in [&alpha, &beta] {
        printed(&w.add(repo));
    }
    let without_list = |w: &World| w.all().into_iter().filter(|(path, _)| *path != w.list_file()).collect::<Vec<_>>();
    let before = without_list(&w);
    let forgot = w.run(&["project", "forget", "alpha"]);
    assert_eq!(printed(&forgot), json!({"name": "alpha", "used": DAY}));
    names_no_path(&w, &forgot);
    assert_eq!(w.on_disk(), json!({"projects": [w.line("beta", &w.real(&beta))]}));
    assert!(without_list(&w) == before && alpha.join("a.txt").is_file(), "forget touched more than the line");

    // A name that is not on the list changes nothing, and neither does one no project is called by.
    let before = w.all();
    let none = "jellymoon: no project of that name is on the list, so nothing was changed\n";
    assert_eq!(refused(&w.run(&["project", "forget", "alpha"])), none);
    assert_eq!(refused(&w.run(&["project", "forget", "Beta"])), none);
    let key = support::hub_key();
    for odd in ["", "al\u{2065}pha", "a/b", "..", key.as_str()] {
        let ran = w.run(&["project", "forget", odd]);
        let takes = "jellymoon: project forget takes a project's name, as `jellymoon project list` prints it\n";
        assert_eq!(refused(&ran), takes, "{odd:?}");
    }
    assert!(w.all() == before, "a refusal wrote something");
    // The last line gone, the list is an empty one.
    printed(&w.run(&["project", "forget", "beta"]));
    assert_eq!((w.on_disk(), w.listed()), (json!({"projects": []}), json!([])));
}

#[test]
fn project_is_add_new_list_or_forget_and_anything_else_is_no_command() {
    let w = heard_world();
    let cases: [&[&str]; 9] = [
        &["project"],
        &["project", "add"],
        &["project", "add", "--date", DAY],
        &["project", "forget"],
        &["project", "forget", "alpha", "beta"],
        &["project", "list", "alpha"],
        &["project", "list", "--repo", "x"],
        &["project", "new"],
        &["project", "make", "alpha"],
    ];
    for args in cases {
        let ran = w.run(args);
        assert_eq!((ran.code, ran.stdout.as_str()), (2, ""), "{args:?}: {}", ran.stderr);
        assert!(ran.stderr.contains("jellymoon project add --repo <folder>"), "{args:?}");
    }
    assert!(!w.at("jm").exists(), "a command that is not one makes nothing");
    let day = w.run(&["project", "add", "--repo", w.repo("alpha").to_str().unwrap(), "--date", "2026-3-10"]);
    assert_eq!(refused(&day), "jellymoon: --date must be a day as YYYY-MM-DD\n");
    // A change that is refused before there is a list makes no folder for one, and no lock.
    let none = "jellymoon: no project of that name is on the list, so nothing was changed\n";
    assert_eq!(refused(&w.run(&["project", "forget", "alpha"])), none);
    assert!(!w.at("jm").exists());
}

#[test]
fn a_stray_argument_to_project_is_answered_in_a_fixed_line_and_never_said_back() {
    let w = heard_world();
    let folder = w.at("work").join("strayfolder");
    let (folder, key) = (folder.to_str().unwrap(), support::hub_key());
    let line = "jellymoon: project is `project add --repo <folder>`, `project new <name>`, `project list` or \
                `project forget <name>`\n";
    for stray in [folder, key.as_str(), "--strayoption"] {
        for before in [&["project", "add"][..], &["project", "list"], &["project", "forget", "x"]] {
            let ran = w.run(&[before, &[stray]].concat());
            assert_eq!((ran.code, ran.stdout.as_str()), (2, ""), "{before:?}: {}", ran.stderr);
            assert!(ran.stderr.starts_with(line), "{before:?}: {}", ran.stderr);
            let said = |part: &str| ran.stderr.contains(part);
            assert!(!said(stray) && !said("stray") && !said(&key[4..]), "{before:?}: {}", ran.stderr);
        }
    }
    // An option whose value is missing gets the same line.
    for missing in [&["project", "add", "--repo"][..], &["project", "add", "--repo", "x", "--date"]] {
        let ran = w.run(missing);
        assert_eq!((ran.code, ran.stdout.as_str(), ran.stderr.starts_with(line)), (2, "", true), "{}", ran.stderr);
    }
    // And so does an argument that is not text, as a stray one and as the name to forget.
    #[cfg(unix)]
    for before in [
        &["project", "list"][..],
        &["project", "add"],
        &["project", "forget"],
        &["project", "forget", "x"],
        &["project", "new"],
    ] {
        use std::os::unix::ffi::OsStrExt;
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_jellymoon"));
        command.args(before).arg(std::ffi::OsStr::from_bytes(b"stray\xff\xfename"));
        command.current_dir(w.dir.path()).env_clear().envs(w.vars.iter().map(|(name, value)| (name, value)));
        let ran = command.output().unwrap();
        let said = String::from_utf8_lossy(&ran.stderr);
        assert_eq!((ran.status.code(), ran.stdout.len()), (Some(2), 0), "{before:?}: {said}");
        assert!(said.starts_with(line) && !said.contains("stray"), "{before:?}: {said}");
    }
    assert!(!w.at("jm").exists(), "a command that is not one makes nothing");
}
