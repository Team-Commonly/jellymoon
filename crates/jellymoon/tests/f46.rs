//! F46: tossing a run's work removes its branch and lets the wish go (ADR 0017, decision 7 and "Found while
//! building F46"; `docs/development/toss.md`). `jellymoon run toss` is run as a black box on runs the made-up
//! `fake-agent` ended in F39's test folder (`f39/world.rs`), in repos made with the real `git`. This file holds
//! what a toss does to work that waits; `f46/refused.rs` what is refused with nothing changed, `f46/unfinished.rs`
//! a run that did not finish and a record that names no commit, `f46/cut.rs` a toss and a keep cut off after
//! each step, `f46/landed.rs` a cut-off keep whose work landed elsewhere than it meant, and `f46/races.rs` a
//! branch changed between the toss's question and its delete.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "f46/cut.rs"]
mod cut;
#[path = "f45/made.rs"]
mod kept;
#[path = "f46/landed.rs"]
mod landed;
#[path = "f46/races.rs"]
mod races;
#[path = "f46/refused.rs"]
mod refused;
mod support;
#[path = "f46/unfinished.rs"]
mod unfinished;
#[path = "f39/world.rs"]
mod world;

pub use kept::{Held, finished, said};
use serde_json::{Value, json};
use support::{Ran, skipped};
pub use world::*;

pub const WORK: &str = "the work of run {run} was tossed";
pub const DELETED: &str = "its branch was deleted, and the wish is let go";
pub const BEFORE: &str = "run {run} was tossed before, and nothing was changed";
pub const LET_GO: &str = "run {run} was tossed, and the wish was let go";
pub const POINTED: &str = "its branch is left, as a name of the person's points at it";
pub const REFS: [&str; 2] = ["for-each-ref", "--format=%(refname) %(objectname) %(symref)"];

/// What a toss that was not refused printed, with its five names and no other.
pub fn tossed(made: &Made, ran: &Ran, changed: bool, commit: Value, why: &str) {
    let why = why.replace("{run}", &made.run);
    let want = json!({"run": made.run, "wish": made.wish, "tossed": changed, "commit": commit, "why": why});
    assert_eq!(printed(ran), want, "{}", ran.stderr);
    made.w.never_says_the_agent(ran);
}

impl Made {
    /// What a toss leaves in the home: the record says tossed and names `commit`, nothing of a keep is in it, and
    /// the wish is let go in the toss's own line, with the run and the stars it had.
    pub fn is_tossed(&self, commit: &Value, stars: usize) {
        let (record, wish) = (self.record(), self.w.kept(&self.wish));
        assert_eq!((&record["work"], &record["tossed"]), (&json!("tossed"), commit));
        assert_eq!((&record["kept"], &record["onto"]), (&Value::Null, &Value::Null));
        let line = json!(LET_GO.replace("{run}", &self.run));
        assert_eq!((&wish["state"], &wish["why"], &wish["run"]), (&json!("let_go"), &line, &json!(self.run)));
        assert_eq!(wish["stars"].as_array().unwrap().len(), stars);
    }

    /// Every ref of the repo but the run's own branch, with what each names.
    pub fn others(&self) -> Vec<String> {
        let mine = format!("{} ", self.branch());
        self.git(&REFS).lines().filter(|line| !line.starts_with(&mine)).map(str::to_string).collect()
    }

    pub fn stop(&self) -> Value {
        printed(&self.w.run(&["run", "stop", "--wish", &self.wish, "--date", DAY]))
    }
}

#[test]
fn work_that_waits_is_tossed_its_branch_alone_is_deleted_and_the_wish_is_let_go() {
    let made = finished();
    made.w.work_in_progress(&made.repo);
    made.git(&["tag", "v1"]);
    made.git(&["branch", "jellymoon/other"]);
    made.git(&["update-ref", "refs/remotes/origin/main", "main"]);
    let (end, person, others, record) = (json!(made.end()), made.w.persons(&made.repo), made.others(), made.record());
    assert_eq!(others.len(), 5, "main, a branch, a tag, a remote's branch and the stash");
    let ran = made.toss();
    tossed(&made, &ran, true, end.clone(), &format!("{WORK}: {DELETED}"));
    // The run's branch is gone, and it alone: no other ref, no file, no index and no `HEAD` is other than it was.
    assert!(made.git(&REFS).lines().all(|line| !line.starts_with(&made.branch())));
    assert_eq!(made.others(), others);
    assert!(person == made.w.persons(&made.repo));
    // The record says tossed and at which commit, after everything a run wrote, and the wish is let go.
    made.is_tossed(&end, 1);
    let mut now = made.record();
    for added in ["work", "tossed"] {
        now.as_object_mut().unwrap().remove(added);
    }
    assert_eq!(now, record);
    assert_eq!(made.w.on_page(&made.wish)["state"], "let_go");
    // A second toss changes nothing, and says so.
    let before = made.held();
    tossed(&made, &made.toss(), false, end, BEFORE);
    assert!(before == made.held());
    // A keep is refused, a look says tossed and lists nothing, and a stop says how the run ended and the toss.
    made.refuses(&made.keep(&[]), &before, "the work of run {run} was tossed, so nothing of it was kept");
    let show = ["run", "show", "--wish", &made.wish, "--date", DAY, "--repo", made.repo.to_str().unwrap()];
    let shown = printed(&made.w.run(&show));
    assert_eq!((&shown["work"], shown["files"].as_array().unwrap().len()), (&json!("tossed"), 0));
    let stopped = made.stop();
    assert_eq!(
        (&stopped["outcome"], &stopped["why"], &stopped["star"]),
        (&json!("completed"), &json!(LET_GO.replace("{run}", &made.run)), &json!(STAR))
    );
    assert!(before == made.held());
}

#[test]
fn a_run_that_changed_nothing_is_tossed_and_its_branch_at_the_start_is_deleted() {
    let w = world();
    w.play("nothing", 0);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (run, _) = ran(&w.start(&wish, &repo));
    let made = Made { w, repo, wish, run };
    let (start, person, others) = (json!(made.w.commit(&made.repo, "main")), made.w.persons(&made.repo), made.others());
    assert_eq!(made.record()["end"], start);
    let why = "run {run}, which changed nothing, was tossed";
    tossed(&made, &made.toss(), true, start.clone(), &format!("{why}: {DELETED}"));
    assert!(made.w.branches(&made.repo).is_empty());
    assert!(person == made.w.persons(&made.repo) && others == made.others());
    made.is_tossed(&start, 1);
    tossed(&made, &made.toss(), false, start, BEFORE);
}

#[test]
fn work_whose_branch_is_not_in_the_repo_is_tossed_in_the_record_and_the_wish() {
    let made = finished();
    made.git(&["update-ref", "-d", &made.branch()]);
    let (end, others) = (json!(made.end()), made.others());
    let why = format!("{WORK}: its branch was not in the repo, and the wish is let go");
    tossed(&made, &made.toss(), true, end.clone(), &why);
    assert_eq!(made.others(), others);
    made.is_tossed(&end, 1);
    tossed(&made, &made.toss(), false, end, BEFORE);
}

// A repo whose config makes git warn at every command (F44's finding) is asked like any other: a name that is
// not there is "not there" whatever git says beside it, so a copy is made in such a repo, and a run started.
#[test]
fn a_copy_is_made_and_a_run_started_and_tossed_in_a_repo_whose_config_makes_git_warn_at_every_command() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    if !w.warns(&repo) {
        return;
    }
    let made = printed(&w.run(&["copy", "make", "--repo", repo.to_str().unwrap()]));
    assert_eq!(w.branches(&repo), vec![made["branch"].as_str().unwrap().to_string()]);
    printed(&w.run(&["copy", "clean", "--run", made["run"].as_str().unwrap(), "--outcome", "completed"]));
    let (run, said) = ran(&w.start(&wish, &repo));
    assert_eq!(said["outcome"], "completed", "{said}");
    let made = Made { w, repo, wish, run };
    tossed(&made, &made.toss(), true, json!(made.end()), &format!("{WORK}: {DELETED}"));
    assert_eq!(made.w.branches(&made.repo).len(), 1, "the copy's own branch is all that is left");
}

// The gate's finding: a name of the person's that is a name for the run's branch (a symbolic ref) would be left
// naming nothing by the delete. The toss is written, the wish is let go, and the branch is left with words that
// say why; on a disk that folds case, a name for another spelling of the branch points at it too.
#[test]
fn a_branch_that_a_name_of_the_persons_points_at_is_left_and_the_toss_is_written() {
    for spelled in ["jellymoon", "Jellymoon"] {
        let made = finished();
        let end = json!(made.end());
        made.git(&["symbolic-ref", "refs/heads/alias", &format!("refs/heads/{spelled}/run-{}", made.run)]);
        if !made.w.git_ran(&made.repo, &["rev-parse", "--verify", "--quiet", "refs/heads/alias"]).status.success() {
            return skipped("a name for another spelling of the run's branch: this disk keeps the spellings apart");
        }
        let (others, person) = (made.others(), made.w.persons(&made.repo));
        tossed(&made, &made.toss(), true, end.clone(), &format!("{WORK}: {POINTED}, and the wish is let go"));
        made.is_tossed(&end, 1);
        assert_eq!((made.others(), made.git(&["rev-parse", "refs/heads/alias"])), (others, made.end()));
        assert!(person == made.w.persons(&made.repo));
        tossed(&made, &made.toss(), false, end.clone(), &format!("{BEFORE}: {POINTED}"));
        // The control: with that name gone, the next toss deletes the branch, and says no more than it knows.
        made.git(&["symbolic-ref", "-d", "refs/heads/alias"]);
        tossed(&made, &made.toss(), true, end, &format!("{}: {DELETED}", unfinished::CUT_OFF));
        assert!(made.w.branches(&made.repo).is_empty());
    }
}
