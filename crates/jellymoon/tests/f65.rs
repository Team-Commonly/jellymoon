//! F65: a new project is a folder Jellymoon makes (ADR 0020, decision 3; `docs/development/project.md`).
//! `jellymoon project new`, `run`, `wish --run` and `hear` are run as black boxes in F39's test folder
//! (`f39/world.rs`), each world in the system's own temporary folder, with the made-up `fake-agent` as the
//! harness and the real git. This file holds what `project new` makes; `f65/refused.rs` what it refuses, with
//! the whole test folder compared; `f65/half.rs` a making that fails or is cut off half way, and two at once;
//! `f65/wished.rs` a wish for the new project, its run and its keep, and a made project that is forgotten.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "f65/half.rs"]
mod half;
#[path = "f55/heard.rs"]
mod heard;
#[path = "f65/refused.rs"]
mod refused;
mod support;
#[path = "f65/wished.rs"]
mod wished;
#[path = "f39/world.rs"]
mod world;

use std::fs;
use std::path::{Path, PathBuf};

pub use heard::*;
use serde_json::{Value, json};
use support::{Ran, plant, tree};
pub use world::*;

/// The one line a wrong `project` command line gets.
pub const IS: &str = "jellymoon: project is `project add --repo <folder>`, `project new <name>`, `project list` or \
                      `project forget <name>`\n";
/// The tree every commit with no file has.
const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

impl World {
    /// A folder by its real path, every link in it followed: how the list holds one.
    pub fn real(&self, folder: &Path) -> String {
        fs::canonicalize(folder).unwrap().to_str().unwrap().trim_start_matches(r"\\?\").to_string()
    }

    pub fn list_file(&self) -> PathBuf {
        self.jm("projects.json")
    }

    /// The one folder Jellymoon makes projects in, and a project's place in it.
    pub fn made(&self, name: &str) -> PathBuf {
        self.jm("projects").join(name)
    }

    /// `jellymoon project new` for a name, used on the day.
    pub fn new_project(&self, name: &str) -> Ran {
        self.run(&["project", "new", "--date", DAY, "--", name])
    }

    pub fn listed(&self) -> Value {
        printed(&self.run(&["project", "list"]))["projects"].clone()
    }

    /// The list as it is on disk: one line of JSON.
    pub fn on_disk(&self) -> Value {
        let text = fs::read_to_string(self.list_file()).unwrap();
        assert!(text.ends_with("}\n") && text.matches('\n').count() == 1, "{text:?}");
        serde_json::from_str(&text).unwrap()
    }

    /// Writes the list by hand, for the person alone.
    pub fn write_list(&self, bytes: &[u8]) {
        fs::create_dir_all(self.at("jm")).unwrap();
        let _ = fs::remove_file(self.list_file());
        fs::write(self.list_file(), bytes).unwrap();
        #[cfg(unix)]
        fs::set_permissions(self.list_file(), std::os::unix::fs::PermissionsExt::from_mode(0o600)).unwrap();
    }

    /// The whole test folder but the made-up agent's own, every folder and every file with its bytes.
    pub fn all(&self) -> Vec<(PathBuf, Vec<u8>)> {
        tree(self.dir.path()).into_iter().filter(|(path, _)| !path.starts_with(self.at("rec"))).collect()
    }

    /// The test folder but Jellymoon's home and the made-up agent's own.
    pub fn outside(&self) -> Vec<(PathBuf, Vec<u8>)> {
        self.all().into_iter().filter(|(path, _)| !path.starts_with(self.at("jm"))).collect()
    }

    /// Makes a wish for `project` on the day and asks for its run with no folder named: what `run` printed.
    pub fn run_found(&self, project: &str) -> Ran {
        let wish = self.wish(project);
        self.run(&["run", "--wish", &wish, "--date", DAY])
    }

    /// What `jellymoon hear` shows for a hold of the usual words whose four seconds have passed: its last line.
    pub fn heard_for(&self) -> Value {
        let mut events = hold(0, SAID);
        events.push(tick(4999));
        self.hear(&[], &events).pop().unwrap()
    }

    /// The names directly in a folder of Jellymoon's home, in order.
    pub fn names_in(&self, folder: &str) -> Vec<String> {
        let names = fs::read_dir(self.jm(folder)).into_iter().flatten();
        let mut names: Vec<String> = names.map(|entry| entry.unwrap().file_name().into_string().unwrap()).collect();
        names.sort();
        names
    }

    /// What a made project's one commit is: who made it, who committed it, its subject, its parents, its tree.
    pub fn first_commit(&self, name: &str) -> String {
        let made = self.made(name);
        assert_eq!(self.git(&made, &["rev-list", "--count", "--all"]), "1");
        assert_eq!(self.git(&made, &["symbolic-ref", "HEAD"]), "refs/heads/main");
        self.git(&made, &["log", "-1", "--format=%an <%ae>|%cn <%ce>|%s|%P|%T", "main"])
    }
}

/// No line a command printed names a folder: not the test's own, and so none below it; and not Jellymoon's home.
pub fn names_no_path(w: &World, ran: &Ran) {
    let said = format!("{}{}", ran.stdout, ran.stderr);
    for folder in [w.dir.path().to_str().unwrap().to_string(), w.real(w.dir.path())] {
        assert!(!said.contains(&folder), "{said}");
    }
    for below in ["jm", "work", "projects"] {
        assert!(!said.contains(&format!("{}{below}", std::path::MAIN_SEPARATOR)), "{said}");
    }
}

#[test]
fn new_makes_a_folder_in_the_one_folder_of_the_home_a_repo_with_one_empty_first_commit_and_lists_it() {
    let w = heard_world();
    let outside = w.outside();
    assert!(!w.at("jm").exists());
    let ran = w.new_project("alpha");
    assert_eq!(printed(&ran), json!({"name": "alpha", "used": DAY}));
    names_no_path(&w, &ran);
    // The folder is `projects/alpha` in Jellymoon's home, for the person alone, as is the folder it is made in.
    let made = w.made("alpha");
    assert!(made.join(".git").is_dir());
    assert_eq!(w.names_in("projects"), ["alpha"]);
    #[cfg(unix)]
    for folder in [w.at("jm"), w.jm("projects"), made.clone()] {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(fs::metadata(&folder).unwrap().permissions().mode() & 0o777, 0o700, "{}", folder.display());
    }
    // It is listed by its real path, as `project add` lists one, and `project list` names it with no folder.
    assert_eq!(w.on_disk(), json!({"projects": [{"name": "alpha", "folder": w.real(&made), "used": DAY}]}));
    assert_eq!(w.listed(), json!([{"name": "alpha", "used": DAY}]));
    assert!(!w.jm("projects.lock").exists(), "the lock was left");
    // One commit, with no file and no parent, on `main`, by Jellymoon where the person's git config names nobody.
    let jellymoon = "Jellymoon <jellymoon@localhost>";
    assert_eq!(w.first_commit("alpha"), format!("{jellymoon}|{jellymoon}|Jellymoon project alpha||{EMPTY_TREE}"));
    // Nothing is in the folder but the repo, the repo holds nothing a template would have put there, and git
    // finds nothing to say of the checkout.
    assert_eq!(fs::read_dir(&made).unwrap().count(), 1);
    assert!(!made.join(".git").join("hooks").exists() && !made.join(".git").join("description").exists());
    assert_eq!(w.git(&made, &["status", "--porcelain"]), "");
    assert_eq!(w.starts(), 0, "a harness was started");

    // A second project is made beside it; with no `--date` it is used today.
    let beta = printed(&w.run(&["project", "new", "beta"]));
    let used = beta["used"].as_str().unwrap();
    assert!(beta["name"] == "beta" && used.len() == 10 && used.as_bytes()[4] == b'-', "{beta}");
    assert_eq!(w.names_in("projects"), ["alpha", "beta"]);
    assert_eq!(w.listed().as_array().unwrap().len(), 2);
    // Nothing was written outside Jellymoon's home.
    assert!(w.outside() == outside, "something was written outside Jellymoon's folder");
}

#[test]
fn the_first_commit_is_by_the_person_when_their_git_config_names_them_and_obeys_nothing_else_of_it() {
    let w = heard_world();
    // The person's config names them, a template with a file in it, another first branch, and a folder of hooks.
    let (template, hooks) = (w.at("template"), w.at("hooks"));
    fs::create_dir_all(template.join("hooks")).unwrap();
    fs::create_dir_all(&hooks).unwrap();
    fs::write(template.join("planted"), "from the template\n").unwrap();
    let hook = hooks.join(format!("reference-transaction{}", std::env::consts::EXE_SUFFIX));
    plant(env!("CARGO_BIN_EXE_fake-hook"), &hook);
    let slashed = |path: &Path| path.to_str().unwrap().replace('\\', "/");
    let config = format!(
        "[user]\n\tname = Ada Example\n\temail = ada@example.test\n[init]\n\ttemplateDir = {}\n\tdefaultBranch = \
         trunk\n[core]\n\thooksPath = {}\n[commit]\n\tgpgSign = true\n",
        slashed(&template),
        slashed(&hooks)
    );
    fs::write(w.at("gitconfig"), config).unwrap();
    // Plain git obeys all of it: the hook is started, the template's file is copied, the branch is `trunk`.
    let plain = w.folder("plain");
    w.git(&plain, &["init", "-q"]);
    w.git(&plain, &["-c", "commit.gpgSign=false", "commit", "-q", "--allow-empty", "-m", "by plain git"]);
    let mut mark = hook.clone().into_os_string();
    mark.push(".ran");
    let mark = PathBuf::from(mark);
    if cfg!(unix) {
        assert!(mark.exists() && plain.join(".git").join("planted").exists(), "plain git did not obey the config");
        assert_eq!(w.git(&plain, &["symbolic-ref", "HEAD"]), "refs/heads/trunk");
    }
    let _ = fs::remove_file(&mark);

    printed(&w.new_project("alpha"));
    let ada = "Ada Example <ada@example.test>";
    assert_eq!(w.first_commit("alpha"), format!("{ada}|{ada}|Jellymoon project alpha||{EMPTY_TREE}"));
    let git = w.made("alpha").join(".git");
    assert!(!mark.exists(), "a hook of the person's config was started");
    assert!(!git.join("planted").exists() && !git.join("hooks").exists(), "the person's template was copied");
    // The commit is not signed, and no signing program was asked for.
    let raw = w.git(&w.made("alpha"), &["cat-file", "commit", "main"]);
    assert!(!raw.contains("gpgsig"), "{raw}");
}

#[test]
fn project_new_takes_one_name_and_a_wrong_command_line_gets_the_fixed_line_and_makes_nothing() {
    let w = heard_world();
    let folder = w.at("work").join("strayfolder");
    let (folder, key) = (folder.to_str().unwrap(), support::hub_key());
    let cases: Vec<Vec<&str>> = vec![
        vec!["project", "new"],
        vec!["project", "new", "--date", DAY],
        vec!["project", "new", "alpha", "beta"],
        vec!["project", "new", "alpha", "--repo", folder],
        vec!["project", "new", "alpha", "--date"],
        vec!["project", "new", "-x"],
        vec!["project", "new", "--strayoption", "alpha"],
        vec!["project", "new", "alpha", folder],
        vec!["project", "new", "alpha", key.as_str()],
        vec!["project", "list", "new"],
    ];
    for args in cases {
        let ran = w.run(&args);
        assert_eq!((ran.code, ran.stdout.as_str()), (2, ""), "{args:?}: {}", ran.stderr);
        assert!(ran.stderr.starts_with(IS), "{args:?}: {}", ran.stderr);
        assert!(ran.stderr.contains("jellymoon project new <name> [--date <YYYY-MM-DD>]"), "{args:?}");
        let said = |part: &str| ran.stderr.contains(part);
        assert!(!said("stray") && !said(&key[4..]) && !said("beta"), "{args:?}: {}", ran.stderr);
    }
    let day = w.run(&["project", "new", "alpha", "--date", "2026-3-10"]);
    assert_eq!(refused(&day), "jellymoon: --date must be a day as YYYY-MM-DD\n");
    assert!(!w.at("jm").exists(), "a command that is not one made something");
}
