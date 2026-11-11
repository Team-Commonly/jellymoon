//! What an agent could have done to its copy, and a repo whose config names programs. An agent can rewrite every file
//! in its copy, the `.git` file too, and can leave a link where the copy was. Cleanup must leave everything outside
//! byte for byte as it was, and git must start no program. The programs are copies of the made-up `fake-hook`,
//! which leaves a `.ran` file beside itself: each test first shows that plain git does start them here.

use std::env::consts::EXE_SUFFIX;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::{World, refused, said, world};
use crate::support::{plant, tree};

const HOOKS: [&str; 5] = ["post-checkout", "pre-commit", "post-commit", "prepare-commit-msg", "reference-transaction"];

/// A link to a folder. Windows lets only some accounts make one: false there when it did not.
#[cfg(unix)]
fn link(to: &Path, at: &Path) -> bool {
    std::os::unix::fs::symlink(to, at).unwrap();
    true
}

#[cfg(windows)]
fn link(to: &Path, at: &Path) -> bool {
    std::os::windows::fs::symlink_dir(to, at).is_ok()
}

/// A program's path as git's config takes one: git hands the value to a shell.
fn shell(program: &Path) -> String {
    format!("\"{}\"", program.to_str().unwrap().replace('\\', "/"))
}

/// The marks left by every made-up program git started.
fn marks(w: &World) -> Vec<PathBuf> {
    let files = tree(w.dir.path()).into_iter().map(|(path, _)| path);
    files.filter(|path| path.extension().is_some_and(|kind| kind == "ran")).collect()
}

/// Checks that plain git started `want` of the planted programs, then removes their marks. A system that starts
/// fewer (it cannot run a program file as a hook, say) is told so, and the test goes on with those it did start.
fn started(w: &World, want: usize) {
    let marks = marks(w);
    if cfg!(unix) {
        assert_eq!(marks.len(), want, "plain git did not start every planted program: {marks:?}");
    } else if marks.len() != want {
        eprintln!("skipped in part: plain git started {} of {want} planted programs here: {marks:?}", marks.len());
    }
    for mark in marks {
        fs::remove_file(mark).unwrap();
    }
}

impl World {
    /// Gives a repo every hook a commit, a checkout and a ref update start, an fsmonitor program and a filter.
    fn plant_programs(&self, repo: &Path) {
        let (hooks, tools, info) = (repo.join(".git").join("hooks"), self.at("tools"), repo.join(".git").join("info"));
        for folder in [&hooks, &tools, &info] {
            fs::create_dir_all(folder).unwrap();
        }
        for hook in HOOKS {
            plant(env!("CARGO_BIN_EXE_fake-hook"), &hooks.join(format!("{hook}{EXE_SUFFIX}")));
        }
        for (name, keys) in
            [("fsmonitor", &["core.fsmonitor"][..]), ("filter", &["filter.mark.clean", "filter.mark.smudge"])]
        {
            let program = tools.join(format!("{name}{EXE_SUFFIX}"));
            plant(env!("CARGO_BIN_EXE_fake-hook"), &program);
            for key in keys {
                self.git(repo, &["config", key, &shell(&program)]);
            }
        }
        fs::write(info.join("attributes"), "*.txt filter=mark\n").unwrap();
    }
}

#[test]
fn a_link_where_a_copy_was_is_never_followed() {
    let w = world();
    let (repo, outside) = (w.repo("alpha"), w.at("outside"));
    fs::create_dir_all(outside.join("deep")).unwrap();
    fs::write(outside.join("keep.txt"), "not the run's\n").unwrap();
    fs::write(outside.join("deep").join("more.txt"), "nor this\n").unwrap();
    let made = w.make(&repo);
    fs::remove_dir_all(&made.folder).unwrap();
    if !link(&outside, &made.folder) {
        eprintln!("skipped: this system did not let the test make a link to a folder");
        return;
    }
    // And a link under a name no copy was made for.
    let stray = "r00000000000000aa";
    assert!(link(&outside, &made.folder.with_file_name(stray)));

    let (base, before) = (w.commit(&repo, "HEAD"), tree(w.dir.path()));
    for (run, outcome) in [(made.run.as_str(), "completed"), (stray, "completed"), (stray, "failed")] {
        refused(&w.clean(run, outcome));
    }
    assert_eq!(tree(w.dir.path()), before, "a refusal wrote something");

    // Failed, the run is kept; six failed runs after it make it one of the old ones, which are removed.
    assert!(said(&w.clean(&made.run, "failed")).kept);
    let outside_before = tree(&outside);
    for _ in 0..6 {
        said(&w.clean(&w.make(&repo).run, "failed"));
    }
    assert_eq!(tree(&outside), outside_before);
    assert!(fs::symlink_metadata(&made.folder).unwrap().file_type().is_symlink(), "the link was not left alone");
    assert_eq!(w.commit(&repo, &made.branch), base, "something outside was committed");
}

#[test]
fn a_rewritten_dot_git_file_sends_nothing_to_another_repo() {
    let w = world();
    let (repo, other) = (w.repo("alpha"), w.repo("other"));
    w.plant_programs(&other);
    let point_away = |folder: &Path| {
        fs::write(folder.join(".git"), format!("gitdir: {}\n", other.join(".git").display())).unwrap();
    };
    let other_before = tree(&other);

    let made = w.make(&repo);
    point_away(&made.folder);
    fs::write(made.folder.join("a.txt"), "edited by the run\n").unwrap();
    // The variables git itself would take a repo from are no way in either.
    let planted: Vec<(&'static str, OsString)> = vec![
        ("GIT_DIR", other.join(".git").into()),
        ("GIT_WORK_TREE", other.clone().into()),
        ("GIT_INDEX_FILE", other.join(".git").join("index").into()),
    ];
    let cleaned = said(&w.run_with(&planted, &["copy", "clean", "--run", &made.run, "--outcome", "completed"]));
    assert!(!cleaned.kept && !made.folder.exists());
    assert_eq!(tree(&other), other_before, "completed");
    // Nobody is named in the person's config, so the commit is Jellymoon's own.
    let log = w.git(&repo, &["log", "-1", "--format=%an <%ae>|%cn <%ce>", "--name-status", &made.branch]);
    let log: Vec<&str> = log.lines().filter(|line| !line.is_empty()).collect();
    assert_eq!(log, ["Jellymoon <jellymoon@localhost>|Jellymoon <jellymoon@localhost>", "M\ta.txt"]);

    let failed = w.make(&repo);
    point_away(&failed.folder);
    assert!(said(&w.clean(&failed.run, "failed")).kept);
    for _ in 0..5 {
        said(&w.clean(&w.make(&repo).run, "failed"));
    }
    assert!(!failed.folder.exists(), "the old failed copy was not removed");
    assert_eq!(tree(&other), other_before, "failed");
    assert_eq!((w.listed(&repo), w.listed(&other)), (6, 1));
}

#[test]
fn no_hook_filter_or_fsmonitor_of_the_repo_is_started() {
    let w = world();
    let repo = w.repo("alpha");
    w.plant_programs(&repo);
    fs::write(repo.join("f.txt"), "x\n").unwrap();
    w.git(&repo, &["add", "f.txt"]); // the filter
    w.git(&repo, &["commit", "-q", "-m", "started by hand"]); // four of the hooks
    w.git(&repo, &["checkout", "-q", "-b", "side"]); // post-checkout
    w.git(&repo, &["checkout", "-q", "main"]);
    w.git(&repo, &["status", "--porcelain"]); // fsmonitor
    started(&w, 7);

    let made = w.make(&repo);
    fs::write(made.folder.join("a.txt"), "edited by the run\n").unwrap();
    fs::write(made.folder.join("n.txt"), "new from the run\n").unwrap();
    fs::remove_file(made.folder.join("b.txt")).unwrap();
    said(&w.clean(&made.run, "completed"));
    assert_eq!(marks(&w), Vec::<PathBuf>::new(), "after a completed run");
    // The filter was off, so the files were committed as they are.
    assert_eq!(w.git(&repo, &["show", &format!("{}:a.txt", made.branch)]), "edited by the run");
    assert_eq!(w.git(&repo, &["show", &format!("{}:n.txt", made.branch)]), "new from the run");

    for _ in 0..6 {
        let failed = w.make(&repo);
        fs::write(failed.folder.join("a.txt"), "left by a failed run\n").unwrap();
        said(&w.clean(&failed.run, "failed"));
    }
    assert_eq!(marks(&w), Vec::<PathBuf>::new(), "after failed runs, one of them removed");
}

// A repo that holds another repo at a path (a submodule) has a folder there in every copy. git, adding files, asks
// for the status of the repo it finds in that folder, with that repo's own config: one an agent could have written.
#[test]
fn a_repo_an_agent_left_where_a_submodule_goes_has_its_config_ignored() {
    let w = world();
    let (repo, inner) = (w.repo("alpha"), w.repo("inner"));
    let at = format!("160000,{},sub", w.commit(&inner, "HEAD"));
    w.git(&repo, &["update-index", "--add", "--cacheinfo", &at]);
    w.git(&repo, &["commit", "-q", "-m", "a repo inside"]);

    let made = w.make(&repo);
    let sub = made.folder.join("sub");
    w.git(w.dir.path(), &["clone", "-q", inner.to_str().unwrap(), sub.to_str().unwrap()]);
    let program = w.at(&format!("planted{EXE_SUFFIX}"));
    plant(env!("CARGO_BIN_EXE_fake-hook"), &program);
    w.git(&sub, &["config", "filter.planted.clean", &shell(&program)]);
    fs::create_dir_all(sub.join(".git").join("info")).unwrap();
    fs::write(sub.join(".git").join("info").join("attributes"), "* filter=planted\n").unwrap();
    // The same size at another time: git has to read the file, through the filter, to tell whether it changed.
    fs::write(sub.join("a.txt"), "a.txt AS IT WAS\n").unwrap();
    let later = SystemTime::now() + Duration::from_secs(10);
    fs::File::options().write(true).open(sub.join("a.txt")).unwrap().set_modified(later).unwrap();
    w.git(&sub, &["status", "--porcelain"]);
    started(&w, 1);

    fs::write(made.folder.join("a.txt"), "edited by the run\n").unwrap();
    said(&w.clean(&made.run, "completed"));
    assert_eq!(marks(&w), Vec::<PathBuf>::new());
    assert_eq!(w.git(&repo, &["diff", "--name-only", "main", &made.branch]), "a.txt");
}
