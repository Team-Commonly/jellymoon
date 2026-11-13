//! Something done in the repo between two git steps of a keep (`docs/development/keep.md`, "A checkout moved
//! just before the step"): the made-up `fake-git` stands first on the keep's `PATH` and does one thing by hand,
//! with the real git, just before a named step (`support/fake_git.rs`, its `first`, `on` and `returns` files).
//! A checkout moved to another branch, or off every branch, just before the one step that lands the work gets
//! the work where it stood by then, and the keep says so. The run's branch, changed just before it is deleted,
//! is deleted only as itself and only while it names the run's end.

use std::fs;
use std::path::Path;

use serde_json::{Value, json};

use super::settings::finished_over;
use super::support::{Ran, skipped};
use super::{BEFORE, DAY, FORWARD, Made, finished, kept, printed, worked};

const ELSEWHERE: &str = "run {run} was kept, on another branch than the keep began on: the checkout was moved to the \
                         branch that `onto` names while the keep was at work, and git landed the work there";
const LOOSE: &str = "run {run} was kept, on no branch that is named here: while the keep was at work the checkout's \
                     HEAD was detached, or moved to a branch whose name Jellymoon does not write down or the repo \
                     does not list under that spelling, and git landed the work there; the run's branch is left and \
                     still holds the run's own commit, {which}: `kept` is its id";
/// What [`LOOSE`] says of the commit that landed: the run's own, or one the keep made.
const OWN: &str = "which is the commit git landed";
const NEW: &str = "and the new commit the keep made to join the work with the person's commits is named by nothing \
                   of Jellymoon's";
const ON_A_RUNS: &str = "while the keep was at work the checkout was moved onto a branch named as a run's is, where \
                         Jellymoon keeps no work: git may have moved that branch forward and written the work's \
                         files, and nothing is recorded as kept; the branch of run {run} is left, and the next `run \
                         keep` begins again";
const FOLDS: &str = "a branch under another spelling of its name: this disk keeps the two spellings apart";

/// A keep in `checkout` with the made-up git told what to do at the git step that has `step` among its words.
/// Each of `told` is a file of its folder and that file's lines: `first` and `after` are done in the checkout by
/// the real git, just before the step and just after it, and `fails` is the number the step ends with, untaken.
pub fn told(made: &Made, checkout: &Path, step: &str, told: &[(&str, &[&str])]) -> Ran {
    let (vars, fake) = (made.w.fake_git(), made.w.at("fake"));
    fs::write(fake.join("on"), step).unwrap();
    fs::write(fake.join("returns"), "").unwrap();
    for (file, lines) in told {
        let there = if *file == "fails" { vec![] } else { vec!["-C", checkout.to_str().unwrap()] };
        fs::write(fake.join(file), [&there[..], lines].concat().join("\n")).unwrap();
    }
    let ran = made.keep_in(checkout, &["--with-settings"], &vars);
    // It takes its lock on `held` at that step and at no other: a keep that never came to the step raced nothing.
    assert!(fake.join("held").exists(), "the made-up git was never at the step {step}: {}", ran.stderr);
    for file in ["on", "first", "after", "fails", "held"] {
        let _ = fs::remove_file(fake.join(file));
    }
    ran
}

/// A keep with `first` done in the repo by the real git just before the git step that has `step` among its words.
fn raced(made: &Made, step: &str, first: &[&str]) -> Ran {
    told(made, &made.repo, step, &[("first", first)])
}

/// What a keep printed when it worked, with `onto` as it is: a name, or null.
fn said(made: &Made, ran: &Ran, commit: &str, onto: Value, why: &str) {
    let why = why.replace("{run}", &made.run);
    let want = json!({"run": made.run, "wish": made.wish, "kept": commit, "onto": onto, "why": why});
    assert_eq!(printed(ran), want, "{}", ran.stderr);
    made.w.never_says_the_agent(ran);
}

/// The record's three words of a keep.
fn words(made: &Made) -> (Value, Value, Value) {
    let record = made.record();
    (record["work"].clone(), record["kept"].clone(), record["onto"].clone())
}

/// What a look says of the run's work.
fn shown(made: &Made) -> Value {
    let show = ["run", "show", "--wish", &made.wish, "--date", DAY, "--repo", made.repo.to_str().unwrap()];
    printed(&made.w.run(&show))["work"].clone()
}

// The gate's second finding. The checks passed for `main`, and just before `git merge` the person made a new
// branch and stood on it: git moved that branch. The work is kept there, and nothing says it is on `main`.
#[test]
fn a_checkout_moved_to_a_new_branch_just_before_the_step_keeps_the_work_there_and_says_so() {
    for joined in [false, true] {
        let made = finished();
        let start = made.w.commit(&made.repo, "main");
        let theirs = if joined { made.moved_on() } else { start.clone() };
        let ran = raced(&made, "merge", &["checkout", "-q", "-b", "feature"]);
        let commit = made.w.commit(&made.repo, "feature");
        assert_ne!(commit, theirs, "git landed the work on the branch the checkout stood on by then");
        said(&made, &ran, &commit, json!("feature"), ELSEWHERE);
        // `main` took nothing, the checkout stands on the new branch with the work's files, and the run's branch
        // is gone: a branch holds its commit.
        assert_eq!(made.w.commit(&made.repo, "main"), theirs);
        assert_eq!(made.git(&["symbolic-ref", "HEAD"]), "refs/heads/feature");
        assert_eq!(made.status(), "");
        assert_eq!(made.git(&["for-each-ref", "--format=%(refname)"]), "refs/heads/feature\nrefs/heads/main");
        assert_eq!(words(&made), (json!("kept"), json!(commit), json!("refs/heads/feature")));
        // A second keep and a look say what is true, and change nothing.
        let before = made.held();
        kept(&made, &made.keep(&[]), &commit, "feature", BEFORE);
        assert_eq!(shown(&made), json!("kept"));
        assert!(before == made.held());
    }
}

#[test]
fn a_checkout_moved_to_an_older_branch_just_before_the_step_keeps_the_work_there_and_says_so() {
    let made = finished_over(&[("more.txt", "more\n")]);
    let (start, end) = (made.w.commit(&made.repo, "main"), made.end());
    made.git(&["branch", "old", "main~1"]);
    let ran = raced(&made, "merge", &["checkout", "-q", "old"]);
    said(&made, &ran, &end, json!("old"), ELSEWHERE);
    assert_eq!((made.w.commit(&made.repo, "old"), made.w.commit(&made.repo, "main")), (end.clone(), start));
    assert_eq!((made.git(&["symbolic-ref", "HEAD"]).as_str(), made.status().as_str()), ("refs/heads/old", ""));
    assert!(made.w.branches(&made.repo).is_empty());
    assert_eq!(words(&made), (json!("kept"), json!(end), json!("refs/heads/old")));
    let before = made.held();
    kept(&made, &made.keep(&[]), &end, "old", BEFORE);
    assert_eq!(shown(&made), json!("kept"));
    assert!(before == made.held());
}

#[test]
fn a_head_detached_just_before_the_step_keeps_the_work_on_no_branch_and_leaves_the_runs_branch() {
    for joined in [false, true] {
        let made = finished();
        let (start, end) = (made.w.commit(&made.repo, "main"), made.end());
        let theirs = if joined { made.moved_on() } else { start };
        let ran = raced(&made, "merge", &["checkout", "-q", "--detach"]);
        // git moved the detached `HEAD` and no branch. The run's branch stays, at the run's own commit: it is the
        // one name that commit has, and the one new commit of a joined keep has no name at all.
        let commit = made.git(&["rev-parse", "HEAD"]);
        assert_eq!(commit == end, !joined);
        said(&made, &ran, &commit, json!(null), &LOOSE.replace("{which}", if joined { NEW } else { OWN }));
        assert!(!made.w.git_ran(&made.repo, &["symbolic-ref", "-q", "HEAD"]).status.success(), "HEAD is detached");
        assert_eq!(made.w.commit(&made.repo, "main"), theirs);
        assert_eq!(made.w.commit(&made.repo, &made.branch()), end);
        assert_eq!(made.git(&["for-each-ref", "--format=%(refname)", "--points-at", &commit]).is_empty(), joined);
        assert_eq!(made.files() == worked(), !joined);
        assert_eq!(made.status(), "");
        assert_eq!(words(&made), (json!("kept"), json!(commit), json!("HEAD")));
        // A second keep and a look say it was kept, name no branch, and leave the run's branch again.
        let before = made.held();
        said(&made, &made.keep(&[]), &commit, json!(null), BEFORE);
        let asked = printed(&made.keep(&["--ask"]));
        assert_eq!((&asked["kept"], &asked["onto"], &asked["fits"]), (&json!(commit), &json!(null), &json!(false)));
        assert_eq!(shown(&made), json!("kept"));
        assert!(before == made.held());
    }
}

// The gate's second finding of its second pass. Moved onto the run's own branch just before the step, the
// checkout is where the work already is, git has nothing to do (or, with a commit to land, will not), and no
// branch of the person's took anything: nothing was kept, the run's branch stays, and the next keep begins again.
// The gate's first finding of its third pass: so too under another spelling of the branch's name, which a disk
// that folds case takes for the branch and the repo does not list. Said as kept there, the same keep deleted the
// run's branch from under the checkout, and nothing reached the run's commit.
#[test]
fn a_checkout_moved_onto_the_runs_own_branch_just_before_the_step_keeps_nothing_and_the_next_keep_lands_it() {
    let spelled = ["Jellymoon/run-", "JELLYMOON/RUN-"];
    for (spelled, joined) in [
        ("jellymoon/run-", false),
        ("jellymoon/run-", true),
        (spelled[0], false),
        (spelled[1], false),
        (spelled[0], true),
    ] {
        let made = finished();
        if spelled != "jellymoon/run-" && !made.repo.join(".GIT").exists() {
            return skipped(FOLDS);
        }
        let (start, end) = (made.w.commit(&made.repo, "main"), made.end());
        let theirs = if joined { made.moved_on() } else { start };
        let own = format!("{spelled}{}", made.run);
        let refs = made.git(&["for-each-ref", "--format=%(refname) %(objectname)"]);
        let ran = raced(&made, "merge", &["checkout", "-q", &own]);
        assert_eq!((ran.code, ran.stdout.as_str()), (1, ""), "{own}, joined {joined}: {}", ran.stderr);
        assert_eq!(super::refused(&ran), super::said(&ON_A_RUNS.replace("{run}", &made.run)), "{own}, {joined}");
        // Every branch is where it was, the run's too, and the record names what was meant and not that it landed.
        assert_eq!(made.git(&["for-each-ref", "--format=%(refname) %(objectname)"]), refs);
        assert_eq!(words(&made).0, json!(null));
        assert_eq!(words(&made).2, json!("refs/heads/main"));
        assert_eq!(shown(&made), json!("waits"));
        // Asked on the run's branch, a keep says that a checkout stands there, and changes nothing.
        let before = made.held();
        assert_eq!(printed(&made.keep(&["--ask"]))["fits"], json!(false));
        assert!(before == made.held());
        // Back on their own branch, the person keeps the work: the commit is on `main`, and only then is the
        // run's branch deleted.
        made.git(&["checkout", "-q", "main"]);
        let ran = made.keep(&[]);
        let commit = made.w.commit(&made.repo, "main");
        kept(&made, &ran, &commit, "main", if joined { super::JOINED } else { FORWARD });
        assert_eq!(made.git(&["rev-parse", &format!("{commit}^")]), theirs);
        assert_eq!(commit == end, !joined);
        assert!(made.w.branches(&made.repo).is_empty());
        assert_eq!(made.status(), "");
    }
}

// Detached at a commit ahead of the work just before the step, git has nothing to do and says so with a 0. The
// work is in that commit's history and on no branch's: nothing was kept, and it is not said to be.
#[test]
fn a_head_detached_ahead_of_the_work_just_before_the_step_is_the_late_refusal_and_not_kept_on_no_branch() {
    let made = finished();
    let (start, end) = (made.w.commit(&made.repo, "main"), made.end());
    made.who();
    let ahead = made.git(&["commit-tree", "-p", &end, "-m", "one more", &format!("{end}^{{tree}}")]);
    let ran = raced(&made, "merge", &["checkout", "-q", "--detach", &ahead]);
    assert_eq!((ran.code, ran.stdout.as_str()), (1, ""), "{}", ran.stderr);
    assert_eq!(super::refused(&ran), super::said(&super::STEPPED.replace("{run}", &made.run)));
    assert_eq!((made.git(&["rev-parse", "HEAD"]), made.w.commit(&made.repo, "main")), (ahead, start));
    assert_eq!(made.w.commit(&made.repo, &made.branch()), end);
    assert_eq!(words(&made), (json!(null), json!(end), json!("refs/heads/main")));
    made.git(&["checkout", "-q", "main"]);
    kept(&made, &made.keep(&[]), &end, "main", FORWARD);
}

// Moved to a new branch just before the step and back just after it: git landed the work on the new branch, and
// by the time the keep asks, the checkout stands on a branch that does not hold it. That branch is not said to
// have held the work already. The keep says git stopped the step, and the next one lands the work on `main` too.
#[test]
fn a_checkout_moved_away_for_the_step_and_back_after_it_is_the_late_refusal_and_the_next_keep_lands_on_main() {
    let made = finished();
    let (start, end) = (made.w.commit(&made.repo, "main"), made.end());
    let moved: [(&str, &[&str]); 2] =
        [("first", &["checkout", "-q", "-b", "feature"]), ("after", &["checkout", "-q", "main"])];
    let ran = told(&made, &made.repo, "merge", &moved);
    assert_eq!((ran.code, ran.stdout.as_str()), (1, ""), "{}", ran.stderr);
    assert_eq!(super::refused(&ran), super::said(&super::STEPPED.replace("{run}", &made.run)));
    assert_eq!((made.w.commit(&made.repo, "feature"), made.w.commit(&made.repo, "main")), (end.clone(), start));
    assert_eq!(made.git(&["symbolic-ref", "HEAD"]), "refs/heads/main");
    assert_eq!(words(&made), (json!(null), json!(end), json!("refs/heads/main")));
    assert_eq!(made.w.commit(&made.repo, &made.branch()), end);
    kept(&made, &made.keep(&[]), &end, "main", FORWARD);
    assert_eq!((made.w.commit(&made.repo, "feature"), made.w.commit(&made.repo, "main")), (end.clone(), end));
    assert!(made.w.branches(&made.repo).is_empty());
}

// The checkout a keep asks about after the step is the one it was asked in. In a linked worktree moved to a new
// branch just before the step, the first checkout stands on `main` as ever, and the work is where the linked
// one stands.
#[test]
fn a_linked_checkout_moved_to_a_new_branch_just_before_the_step_keeps_the_work_where_that_checkout_stands() {
    let made = finished();
    let (start, end) = (made.w.commit(&made.repo, "main"), made.end());
    let feat = made.w.at("work").join("linked").join("alpha");
    made.git(&["worktree", "add", "-q", "-b", "feat", feat.to_str().unwrap()]);
    let main = made.w.persons(&made.repo);
    let ran = told(&made, &feat, "merge", &[("first", &["checkout", "-q", "-b", "other"])]);
    said(&made, &ran, &end, json!("other"), ELSEWHERE);
    assert_eq!(made.w.git(&feat, &["symbolic-ref", "HEAD"]), "refs/heads/other");
    let tips = ["other", "feat", "main"].map(|name| made.w.commit(&made.repo, name));
    assert_eq!(tips, [end.clone(), start.clone(), start]);
    assert_eq!(words(&made), (json!("kept"), json!(end), json!("refs/heads/other")));
    let after = made.w.persons(&made.repo);
    assert_eq!((&main.0, &main.1, &main.2), (&after.0, &after.1, &after.2), "the first checkout was touched");
}

// Moved to a branch that holds the run's commit already, git has nothing to do there and says so with a 0: the
// keep names that branch, and says that nothing was landed.
#[test]
fn a_checkout_moved_to_a_branch_that_holds_the_work_just_before_the_step_is_said_as_that() {
    let made = finished();
    let (start, end) = (made.w.commit(&made.repo, "main"), made.end());
    made.who();
    let ahead = made.git(&["commit-tree", "-p", &end, "-m", "one more", &format!("{end}^{{tree}}")]);
    made.git(&["branch", "ahead", &ahead]);
    let ran = raced(&made, "merge", &["checkout", "-q", "ahead"]);
    said(&made, &ran, &end, json!("ahead"), super::record::HELD);
    assert_eq!((made.w.commit(&made.repo, "ahead"), made.w.commit(&made.repo, "main")), (ahead, start));
    assert_eq!((made.git(&["symbolic-ref", "HEAD"]).as_str(), made.status().as_str()), ("refs/heads/ahead", ""));
    assert!(made.w.branches(&made.repo).is_empty());
    assert_eq!(words(&made), (json!("kept"), json!(end), json!("refs/heads/ahead")));
}

// Moved to a branch whose name a record does not hold (here one with a no-break space), the work is kept as on
// no branch that is named: the record never holds such a name, whoever made the branch. So is it moved to the
// person's own `feature` as `Feature`, on a disk that folds case: git moves `feature`, the repo lists no
// `Feature`, and a record that named it would name a branch nothing reads back. The gate's finding of its fourth
// pass: so is it when the repo does list a `Feature`, in its packed list at a commit of the person's own, and
// `HEAD` is aimed at that name. git moves `feature` through its file, and the listed `Feature` took nothing.
#[test]
fn a_checkout_moved_to_a_branch_no_record_names_just_before_the_step_keeps_the_work_as_on_no_branch() {
    let (odd, aimed) = ("odd\u{a0}name", ["symbolic-ref", "HEAD", "refs/heads/Feature"]);
    let spelled = ["checkout", "-q", "Feature"];
    for (listed, first) in [(odd, &["checkout", "-q", "-b", odd][..]), ("feature", &spelled), ("feature", &aimed)] {
        let made = finished();
        if listed != odd && !made.repo.join(".GIT").exists() {
            return skipped(FOLDS);
        }
        let (start, end) = (made.w.commit(&made.repo, "main"), made.end());
        let theirs = (first == aimed).then(|| {
            made.who();
            let theirs = made.git(&["commit-tree", "-p", &start, "-m", "theirs", &format!("{start}^{{tree}}")]);
            made.git(&["branch", "Feature", &theirs]);
            made.git(&["pack-refs", "--all"]);
            format!("refs/heads/Feature {theirs}\n")
        });
        made.git(&["branch", "feature"]);
        let ran = raced(&made, "merge", first);
        // What the repo lists by now is what tells, and a disk that folds case does one of two things. A Mac keeps
        // the file `feature` under its spelling: nothing lists `Feature` at the commit, and the work is on no
        // branch that is named. Windows gives the file the spelling git wrote it under: the repo lists `Feature`
        // at the commit and no `feature`, the work is kept there, and the run's branch goes.
        let refs = made.git(&["for-each-ref", "--format=%(refname) %(objectname)"]) + "\n";
        let respelled = refs.contains(&format!("refs/heads/Feature {end}\n"));
        assert!(respelled || theirs.is_none_or(|theirs| refs.contains(&theirs)), "`Feature` took nothing: {refs}");
        let (onto, held, why) = match respelled {
            true => (json!("Feature"), "refs/heads/Feature", ELSEWHERE.to_string()),
            false => (json!(null), "HEAD", LOOSE.replace("{which}", OWN)),
        };
        said(&made, &ran, &end, onto.clone(), &why);
        assert_eq!(made.w.commit(&made.repo, "main"), start);
        assert!(respelled || refs.contains(&format!("refs/heads/{listed} {end}\n")), "{refs}");
        assert_eq!(refs.contains(&format!("{} {end}\n", made.branch())), !respelled, "{refs}");
        assert_eq!(words(&made), (json!("kept"), json!(end), json!(held)));
        let before = made.held();
        said(&made, &made.keep(&[]), &end, onto, BEFORE);
        assert!(before == made.held());
    }
}

// What stands when git landed the work nowhere the checkout stands: the late refusal, and the next keep begins
// again. Here the person's own file came to be in the way just before the step.
#[test]
fn a_step_git_stops_is_refused_in_the_late_words_and_the_next_keep_begins_again() {
    let made = finished();
    let (start, end) = (made.w.commit(&made.repo, "main"), made.end());
    let mine = made.w.at("tmp").join("mine.txt");
    fs::write(&mine, "the person's own, made just then\n").unwrap();
    let blob = made.git(&["hash-object", "-w", mine.to_str().unwrap()]);
    let before = made.w.persons(&made.repo);
    // The file is staged as `n.txt` just before the step: git's own check stops for it.
    let ran = raced(&made, "merge", &["update-index", "--add", "--cacheinfo", &format!("100644,{blob},n.txt")]);
    assert_eq!(super::refused(&ran), super::said(&super::STEPPED.replace("{run}", &made.run)));
    let after = made.w.persons(&made.repo);
    assert_eq!(
        (&before.0, &before.2, &before.3),
        (&after.0, &after.2, &after.3),
        "files, HEAD and refs are as they were"
    );
    assert_eq!(words(&made), (json!(null), json!(end), json!("refs/heads/main")), "the record names what was meant");
    assert_eq!(made.w.commit(&made.repo, "main"), start);
    made.git(&["rm", "-q", "--cached", "n.txt"]);
    kept(&made, &made.keep(&[]), &end, "main", FORWARD);
}

// An index that came to be locked just before the step is what stopped it, and is said as that.
#[test]
fn an_index_locked_just_before_the_step_is_said_as_a_locked_index() {
    let made = finished();
    let (start, lock) = (made.w.commit(&made.repo, "main"), made.repo.join(".git").join("index.lock"));
    let files = made.files();
    // The real git makes the file: it writes a config file wherever it is told to.
    let ran = raced(&made, "merge", &["config", "--file", lock.to_str().unwrap(), "a.b", "c"]);
    let locked = "the checkout's index is locked (index.lock): another git is at work in it, or one was cut off and \
                  left the file: the work of run {run} was not kept";
    assert_eq!((ran.code, ran.stdout.as_str()), (1, ""), "{}", ran.stderr);
    assert_eq!(super::refused(&ran), super::said(&locked.replace("{run}", &made.run)));
    assert_eq!((made.w.commit(&made.repo, "main"), made.files()), (start, files));
    fs::remove_file(lock).unwrap();
    kept(&made, &made.keep(&[]), &made.end(), "main", FORWARD);
}

// The run's branch is deleted as itself. Made a name for `main` just before the delete, the name goes and
// `main`, which holds the work by then, stays: a delete that followed the name would take the person's branch.
#[test]
fn a_run_branch_made_a_name_for_main_just_before_it_is_deleted_goes_alone_and_main_stays() {
    let made = finished();
    let end = made.end();
    let ran = raced(&made, "update-ref", &["symbolic-ref", &made.branch(), "refs/heads/main"]);
    kept(&made, &ran, &end, "main", FORWARD);
    assert_eq!(
        made.git(&["for-each-ref", "--format=%(refname) %(objectname) [%(symref)]"]),
        format!("refs/heads/main {end} []")
    );
    assert_eq!((made.git(&["symbolic-ref", "HEAD"]).as_str(), made.status().as_str()), ("refs/heads/main", ""));
}

// And only while it names the run's end: moved just before the delete, it is someone's by then, and stays.
#[test]
fn a_run_branch_moved_just_before_it_is_deleted_is_left_where_it_was_put() {
    let made = finished();
    let (start, end) = (made.w.commit(&made.repo, "main"), made.end());
    let ran = raced(&made, "update-ref", &["update-ref", &made.branch(), &start]);
    kept(&made, &ran, &end, "main", FORWARD);
    let refs = made.git(&["for-each-ref", "--format=%(refname) %(objectname)"]);
    assert_eq!(refs, format!("{} {start}\nrefs/heads/main {end}", made.branch()));
    // The work was kept all the same, and the next keep says so and leaves the branch again.
    kept(&made, &made.keep(&[]), &end, "main", BEFORE);
    assert_eq!(made.w.commit(&made.repo, &made.branch()), start);
}
