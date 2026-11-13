//! A toss of a run that did not finish, and of a record that names no commit (ADR 0017, "Found while building
//! F46"; `docs/development/toss.md`, "A run that did not finish"): the kept copy goes as `copy clean` removes
//! one, the branch goes only while it is still where the run started, and any record is written `tossed`.

use std::fs;

use serde_json::{Value, json};

use super::{BEFORE, DAY, DELETED, Made, WORK, World, ended, printed, ran, tossed, wait_for, world};

pub const UNFINISHED: &str = "run {run}, which did not finish, was tossed";
pub const REMOVED: &str = "its kept copy was removed, ";
pub const NO_COPY: &str = "no copy of it was kept by now, ";
pub const CUT_OFF: &str = "a toss of run {run} that was cut off was finished";
const MOVED: &str = "its branch is left, as it is not at the commit the record names for it";
const UNNAMED: &str = "its branch is left, as the record names no commit for it";

/// A run that failed, or one a stop cancelled, with its copy kept and its branch where the run started.
pub fn unfinished(cancelled: bool) -> Made {
    let w = world();
    w.play(if cancelled { "forever" } else { "exit-3" }, 0);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let run = if cancelled {
        let going = w.start_going(&wish, &repo);
        wait_for(&w.at("rec").join("started"));
        printed(&w.run(&["run", "stop", "--wish", &wish, "--date", DAY]));
        printed(&ended(going))["run"].as_str().unwrap().to_string()
    } else {
        ran(&w.start(&wish, &repo)).0
    };
    let made = Made { w, repo, wish, run };
    assert_eq!((made.w.copies(), made.w.branches(&made.repo).len()), (vec![made.run.clone()], 1));
    assert_eq!(made.record()["outcome"], if cancelled { "cancelled" } else { "failed" });
    made
}

// A run whose command was killed has no end in its record. A toss gives it one first, as a stop and a page do.
#[test]
fn a_run_whose_command_was_killed_is_given_its_end_and_then_tossed() {
    let w = world();
    w.play("forever", 0);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let mut going = w.start_going(&wish, &repo);
    wait_for(&w.at("rec").join("started"));
    going.kill().unwrap();
    going.wait().unwrap();
    if !cfg!(unix) {
        w.let_end("stop"); // there is no signal there: a toss stops nothing, and its folder cannot be removed
    }
    let run = w.kept(&wish)["run"].as_str().unwrap().to_string();
    let made = Made { w, repo, wish, run };
    assert_eq!(made.record()["outcome"], Value::Null);
    let start = json!(made.w.commit(&made.repo, "main"));
    tossed(&made, &made.toss(), true, start.clone(), &format!("{UNFINISHED}: {REMOVED}{DELETED}"));
    assert_eq!(
        (made.w.copies(), made.w.branches(&made.repo), &made.record()["outcome"]),
        (vec![], vec![], &json!("failed"))
    );
    made.is_tossed(&start, 0);
}

impl World {
    /// How many working trees a repo lists, its own checkout among them.
    pub fn listed(&self, repo: &std::path::Path) -> usize {
        self.git(repo, &["worktree", "list", "--porcelain"]).matches("worktree ").count()
    }
}

#[test]
fn a_run_that_did_not_finish_is_tossed_with_its_kept_copy_and_its_branch_at_the_start() {
    for cancelled in [false, true] {
        let made = unfinished(cancelled);
        let (start, person, others) = (made.w.commit(&made.repo, "main"), made.w.persons(&made.repo), made.others());
        let (start, line) = (json!(start), made.w.kept(&made.wish)["why"].clone());
        assert_eq!((&made.record()["start"], made.record().get("end")), (&start, None));
        // Its line said the copy is kept, and a stop says the same until the copy is gone.
        assert!(line.as_str().unwrap().ends_with(": its copy is kept and no star was added"), "{line}");
        assert_eq!(made.stop()["why"], line);
        tossed(&made, &made.toss(), true, start.clone(), &format!("{UNFINISHED}: {REMOVED}{DELETED}"));
        // The copy's folder, the repo's listing of it and its branch are gone, and nothing of the person's is.
        assert_eq!((made.w.copies(), made.w.listed(&made.repo), made.w.branches(&made.repo)), (vec![], 1, vec![]));
        assert!(!made.w.jm("runs").join(&made.run).join("copy.json").exists());
        assert!(person == made.w.persons(&made.repo) && others == made.others());
        made.is_tossed(&start, 0);
        assert_eq!(made.w.on_page(&made.wish)["state"], "let_go");
        // A stop says how the run ended and that it was tossed, and adds no star.
        let stopped = made.stop();
        let outcome = if cancelled { "cancelled" } else { "failed" };
        assert_eq!((&stopped["outcome"], &stopped["star"]), (&json!(outcome), &Value::Null));
        assert_eq!(stopped["why"], made.w.kept(&made.wish)["why"]);
        let before = made.held();
        tossed(&made, &made.toss(), false, start, BEFORE);
        assert!(before == made.held());
    }
}

#[test]
fn a_branch_that_is_not_where_the_run_started_is_left_and_the_copy_goes() {
    let made = unfinished(false);
    let start = json!(made.w.commit(&made.repo, "main"));
    made.w.git(&made.w.jm("copies").join(&made.run), &["commit", "-q", "--allow-empty", "-m", "the person's own"]);
    let mine = made.w.commit(&made.repo, &made.branch());
    tossed(
        &made,
        &made.toss(),
        true,
        start.clone(),
        &format!("{UNFINISHED}: {REMOVED}{MOVED}, and the wish is let go"),
    );
    assert_eq!((made.w.copies(), made.w.listed(&made.repo)), (vec![], 1));
    assert_eq!(made.w.commit(&made.repo, &made.branch()), mine, "the branch keeps the commit made on it");
    made.is_tossed(&start, 0);
    let before = made.held();
    tossed(&made, &made.toss(), false, start, &format!("{BEFORE}: {MOVED}"));
    assert!(before == made.held());
}

#[test]
fn a_copy_that_is_gone_already_is_said_so_and_a_repo_that_is_not_found_leaves_the_branch() {
    // The folder was removed by hand: the repo's listing of it and its record go, and the branch with them.
    let made = unfinished(false);
    let start = json!(made.w.commit(&made.repo, "main"));
    fs::remove_dir_all(made.w.jm("copies").join(&made.run)).unwrap();
    tossed(&made, &made.toss(), true, start, &format!("{UNFINISHED}: {NO_COPY}{DELETED}"));
    assert_eq!((made.w.listed(&made.repo), made.w.branches(&made.repo)), (1, vec![]));
    // Everything of the copy taken out of the home by hand, the lock every clean holds too: there is no copy, and
    // nothing says whose repo lists it. The repo lists it still, as a checkout on the run's branch: it is left.
    // Half a keep in the record of a run that did not finish is not core's, and is refused first.
    let made = unfinished(false);
    let (start, runs, kept) = (json!(made.w.commit(&made.repo, "main")), made.w.jm("runs"), made.record());
    fs::remove_dir_all(made.w.jm("copies").join(&made.run)).unwrap();
    fs::remove_file(runs.join(&made.run).join("copy.json")).unwrap();
    fs::remove_file(runs.join("clean.lock")).unwrap();
    let mut record = kept.clone();
    (record["kept"], record["onto"]) = (start.clone(), json!("refs/heads/main"));
    made.write(&record);
    let before = made.held();
    made.refuses(&made.toss(), &before, "the record of run {run} is not one Jellymoon wrote for this wish");
    made.write(&kept);
    let stood = "its branch is left, as a checkout stands on it, and the wish is let go";
    tossed(&made, &made.toss(), true, start, &format!("{UNFINISHED}: {NO_COPY}{stood}"));
    assert_eq!((made.w.listed(&made.repo), made.w.branches(&made.repo).len()), (2, 1));
    // Asked of a folder that is not the run's repo: the copy goes by its own record, and no branch is asked for
    // anywhere. The next toss, asked of the repo, finishes it.
    let made = unfinished(false);
    let (start, other) = (json!(made.w.commit(&made.repo, "main")), made.w.repo("elsewhere/alpha"));
    made.w.git(&other, &["branch", &format!("jellymoon/run-{}", made.run)]);
    let unasked = "its branch is left, as the repo could not be found or asked, and the wish is let go";
    tossed(&made, &made.toss_in(&other, &[]), true, start.clone(), &format!("{UNFINISHED}: {REMOVED}{unasked}"));
    assert_eq!((made.w.copies(), made.w.branches(&made.repo).len(), made.w.branches(&other).len()), (vec![], 1, 1));
    made.is_tossed(&start, 0);
    tossed(&made, &made.toss(), true, start, &format!("{CUT_OFF}: {NO_COPY}{DELETED}"));
    assert_eq!((made.w.branches(&made.repo).len(), made.w.branches(&other).len()), (0, 1));
}

#[test]
fn a_kept_copy_whose_place_is_not_a_folder_jellymoon_made_is_refused_with_nothing_tossed() {
    let nothing = "the kept copy of run {run} could not be removed: its place is not a folder Jellymoon made, or it \
                   could not be asked about, and nothing was tossed";
    // A folder at the copy's name with no record of a copy.
    let made = unfinished(false);
    let record = made.w.jm("runs").join(&made.run).join("copy.json");
    let kept = fs::read(&record).unwrap();
    fs::remove_file(&record).unwrap();
    let before = made.held();
    made.refuses(&made.toss(), &before, nothing);
    assert_eq!((made.w.copies().len(), &made.w.kept(&made.wish)["state"]), (1, &json!("needs_you")));
    fs::write(&record, kept).unwrap();
    // A link at the copy's name: nothing is removed through it, and what it leads to is as it was.
    let (place, real) = (made.w.jm("copies").join(&made.run), made.w.at("work").join("elsewhere"));
    fs::rename(&place, &real).unwrap();
    #[cfg(unix)]
    let linked = std::os::unix::fs::symlink(&real, &place).is_ok();
    #[cfg(windows)]
    let linked = std::os::windows::fs::symlink_dir(&real, &place).is_ok();
    if !linked {
        return crate::support::skipped("no link can be made here");
    }
    made.refuses(&made.toss(), &before, nothing);
    assert!(real.join("a.txt").is_file() && made.record().get("work").is_none());
    fs::remove_file(&place).or_else(|_| fs::remove_dir(&place)).unwrap();
    fs::rename(&real, &place).unwrap();
    let start = json!(made.w.commit(&made.repo, "main"));
    tossed(&made, &made.toss(), true, start, &format!("{UNFINISHED}: {REMOVED}{DELETED}"));
}

#[test]
fn a_record_that_names_no_commit_is_tossed_and_its_branch_is_left() {
    // A run that finished under a build from before F48.
    let made = super::finished();
    made.write(&made.as_before());
    let (end, others) = (made.w.commit(&made.repo, &made.branch()), made.git(&super::REFS));
    tossed(&made, &made.toss(), true, Value::Null, &format!("{WORK}: {UNNAMED}, and the wish is let go"));
    assert_eq!((made.git(&super::REFS), made.w.commit(&made.repo, &made.branch())), (others, end));
    made.is_tossed(&Value::Null, 1);
    let before = made.held();
    tossed(&made, &made.toss(), false, Value::Null, &format!("{BEFORE}: {UNNAMED}"));
    assert!(before == made.held());
    // A run that did not finish under one: its kept copy goes, and its branch is left.
    let made = unfinished(false);
    made.write(&made.as_before());
    let why = format!("{UNFINISHED}: {REMOVED}{UNNAMED}, and the wish is let go");
    tossed(&made, &made.toss(), true, Value::Null, &why);
    assert_eq!((made.w.copies(), made.w.listed(&made.repo), made.w.branches(&made.repo).len()), (vec![], 1, 1));
    made.is_tossed(&Value::Null, 0);
    tossed(&made, &made.toss(), false, Value::Null, &format!("{BEFORE}: {UNNAMED}"));
}

#[test]
fn a_stop_does_not_say_a_copy_is_kept_that_is_gone() {
    // The folder is removed by hand here, as a failed copy past the newest five is by a later run. The copy's
    // record is still there: it is the folder that the words are about.
    let made = unfinished(false);
    let line = made.w.kept(&made.wish)["why"].as_str().unwrap().to_string();
    assert_eq!(made.stop()["why"], json!(line), "the copy stands");
    fs::remove_dir_all(made.w.jm("copies").join(&made.run)).unwrap();
    let gone = line.replace(": its copy is kept and no star", ": its copy is no longer kept, and no star");
    assert!(gone != line && gone.ends_with(": its copy is no longer kept, and no star was added"));
    assert_eq!(made.stop()["why"], json!(gone));
    assert_eq!(made.w.kept(&made.wish)["why"], json!(line), "a stop of a run that ended writes nothing");
}
