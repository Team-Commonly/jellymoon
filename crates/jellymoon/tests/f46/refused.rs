//! What a toss of work that waits refuses, with every ref, the person's files, index and `HEAD`, the run's record
//! and its wish as they were, and the reason in fixed words (`docs/development/toss.md`, "What is refused").

use std::fs;

use serde_json::{Value, json};

use super::{DAY, DELETED, Made, WORK, ended, finished, printed, refused, said, tossed, wait_for, world};

const STOOD: &str = "a checkout stands on the branch of run {run}: its work was not tossed, and nothing was changed";
const MOVED: &str = "the branch of run {run} is not as the run left it, so what is on it is not tossed as the run's";
const NOT_CORES: &str = "the record of run {run} is not one Jellymoon wrote for this wish";
const UNREAD: &str = "the work of run {run} could not be read from the repo: the folder is not the top of a checkout \
                      git opens, or git did not answer";

/// Fails unless a toss is refused for `why` with nothing changed, the wish neither.
fn no(made: &Made, why: &str) {
    let (before, wish) = (made.held(), made.w.kept(&made.wish));
    made.refuses(&made.toss(), &before, why);
    assert_eq!(made.w.kept(&made.wish), wish);
}

/// The toss that goes through once what was in its way is gone.
fn then_tossed(made: &Made) {
    tossed(made, &made.toss(), true, json!(made.end()), &format!("{WORK}: {DELETED}"));
    assert!(made.w.branches(&made.repo).is_empty());
}

#[test]
fn a_branch_someone_stands_on_is_left_and_nothing_is_written() {
    let made = finished();
    let short = format!("jellymoon/run-{}", made.run);
    // The person's own checkout stands on it.
    made.git(&["checkout", "-q", &short]);
    no(&made, STOOD);
    made.git(&["checkout", "-q", "main"]);
    // A second checkout of the repo does, and the toss is asked from either.
    let looked = made.w.at("work").join("looked").join("alpha");
    made.git(&["worktree", "add", "-q", looked.to_str().unwrap(), &short]);
    no(&made, STOOD);
    let before = made.held();
    made.refuses(&made.toss_in(&looked, &[]), &before, STOOD);
    made.git(&["worktree", "remove", looked.to_str().unwrap()]);
    then_tossed(&made);
}

#[test]
fn a_branch_that_is_a_name_for_another_or_is_not_as_the_run_left_it_is_left_and_nothing_is_written() {
    let made = finished();
    let (name, end) = (made.branch(), made.end());
    // A symbolic ref to the person's branch: nothing is deleted through it.
    made.git(&["update-ref", "-d", &name]);
    made.git(&["symbolic-ref", &name, "refs/heads/main"]);
    no(&made, "the branch of run {run} is a name for another branch by now, so its work is not tossed");
    made.git(&["symbolic-ref", "-d", &name]);
    // Moved since the work was shown: a commit on top of the run's, and back to where the run started.
    let on_top = made.git(&["commit-tree", &format!("{end}^{{tree}}"), "-p", &end, "-m", "the person's own"]);
    for moved in [on_top.as_str(), "main"] {
        made.git(&["update-ref", &name, moved]);
        no(&made, MOVED);
    }
    // A commit that is not one a run's copy ends as, which the record was made to name.
    made.git(&["update-ref", &name, &on_top]);
    let kept = made.record();
    let mut record = kept.clone();
    record["end"] = json!(on_top);
    made.write(&record);
    no(&made, MOVED);
    // The run's own commit, under a record made to name another start than its one parent.
    made.git(&["update-ref", &name, &end]);
    record["end"] = json!(end);
    record["start"] = json!(on_top);
    made.write(&record);
    no(&made, MOVED);
    made.write(&kept);
    // A repo whose checkouts cannot be listed cannot be asked who stands on the branch.
    let listing = made.repo.join(".git").join("worktrees");
    fs::write(&listing, "not a folder").unwrap();
    no(&made, UNREAD);
    fs::remove_file(&listing).unwrap();
    then_tossed(&made);
}

// The record is written before the branch is deleted, or nothing is: a record that cannot be written ends the
// toss with the branch where it was.
#[cfg(unix)]
#[test]
fn a_record_that_cannot_be_written_ends_the_toss_with_nothing_deleted() {
    use std::os::unix::fs::PermissionsExt;
    let made = finished();
    let folder = made.w.jm("runs").join(&made.run);
    let closed = |mode: u32| fs::set_permissions(&folder, fs::Permissions::from_mode(mode)).unwrap();
    closed(0o500);
    if fs::write(folder.join("probe"), "").is_ok() {
        closed(0o700);
        return crate::support::skipped("this user writes in a folder nobody may write in");
    }
    let (before, wish) = (made.held(), made.w.kept(&made.wish));
    let ran = made.toss();
    closed(0o700);
    made.refuses(&ran, &before, "the record of run {run} could not be written: nothing was tossed");
    assert_eq!(made.w.kept(&made.wish), wish);
    then_tossed(&made);
}

#[test]
fn a_toss_asked_of_a_repo_the_record_does_not_name_is_refused_and_nothing_is_deleted_there() {
    let made = finished();
    let other = made.w.repo("elsewhere/alpha");
    made.w.git(&other, &["branch", &format!("jellymoon/run-{}", made.run)]);
    let (before, wish) = (made.held(), made.w.kept(&made.wish));
    let another = "the repo found for run {run} is not the one its record names, so its work is not tossed";
    made.refuses(&made.toss_in(&other, &[]), &before, another);
    assert_eq!((made.w.branches(&other).len(), made.w.kept(&made.wish)), (1, wish));
    // A folder of the project's name that is no repo, and one of another name.
    let nowhere = made.w.at("tmp").join("alpha");
    fs::create_dir(&nowhere).unwrap();
    made.refuses(&made.toss_in(&nowhere, &[]), &before, UNREAD);
    assert_eq!(made.toss_in(&made.w.at("tmp"), &[]).code, 1);
    assert!(before == made.held());
    then_tossed(&made);
}

#[test]
fn a_wish_with_no_run_a_run_that_is_going_and_work_that_was_kept_are_refused() {
    let w = world();
    w.play("forever", 0);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let toss = |wish: &str| w.run(&["run", "toss", "--wish", wish, "--date", DAY, "--repo", repo.to_str().unwrap()]);
    assert_eq!(refused(&toss(&wish)), said(&format!("wish {wish} has had no run, so there is nothing to toss")));
    assert_eq!(toss("m7").code, 1);
    let going = w.start_going(&wish, &repo);
    wait_for(&w.at("rec").join("started"));
    let run = w.kept(&wish)["run"].as_str().unwrap().to_string();
    assert_eq!(
        refused(&toss(&wish)),
        said(&format!("run {run} is going: it is stopped first, and nothing was tossed"))
    );
    assert_eq!((&w.kept(&wish)["state"], w.branches(&repo).len()), (&json!("working"), 1), "a toss stops nothing");
    printed(&w.run(&["run", "stop", "--wish", &wish, "--date", DAY]));
    assert_eq!(printed(&ended(going))["outcome"], "cancelled");
    // Work the person kept is theirs: a toss says so and changes nothing.
    let made = finished();
    printed(&made.keep(&[]));
    let was_kept = "the work of run {run} was kept, so it is not tossed: nothing was changed";
    no(&made, was_kept);
    assert_eq!(made.w.commit(&made.repo, "main"), made.end());
    // It is the record's `work` that says so: kept on another branch than a keep first meant, or on none (a
    // `HEAD` that names no branch), is kept all the same.
    for onto in [json!("refs/heads/another"), json!("HEAD"), Value::Null] {
        let mut record = made.record();
        record["onto"] = onto;
        if record["onto"].is_null() {
            record.as_object_mut().unwrap().remove("onto");
        }
        made.write(&record);
        no(&made, was_kept);
    }
}

#[test]
fn a_toss_is_refused_while_a_keep_or_another_toss_holds_the_home() {
    let made = finished();
    let before = made.held();
    let lock = fs::File::create(made.w.jm("runs").join("keep.lock")).unwrap();
    lock.lock().unwrap();
    let wait = [("JELLYMOON_TEST_KEEP_WAIT_MS", "200".into())];
    let busy = "another keep or toss is at work in this home: nothing of run {run} was tossed";
    made.refuses(&made.toss_in(&made.repo, &wait), &before, busy);
    drop(lock);
    // A lock that cannot be made is not another command at work, and is said in its own words.
    let file = made.w.jm("runs").join("keep.lock");
    fs::remove_file(&file).unwrap();
    fs::create_dir(&file).unwrap();
    let unmade = "the lock a keep or a toss holds (runs/keep.lock) could not be made or taken in this home: \
                  nothing of run {run} was tossed";
    made.refuses(&made.toss(), &before, unmade);
    fs::remove_dir(&file).unwrap();
    then_tossed(&made);
}

#[test]
fn a_record_that_cannot_be_read_or_is_not_cores_is_refused() {
    let made = finished();
    let (kept, end, start) = (made.record(), made.end(), made.w.commit(&made.repo, "main"));
    // Tossed at a commit while its work waits, tossed at none, at another commit than its end, and half a keep.
    let shapes = [
        vec![("tossed", json!(end))],
        vec![("work", json!("tossed"))],
        vec![("work", json!("tossed")), ("tossed", json!(start))],
        vec![("kept", json!(end))],
    ];
    for shape in shapes {
        let mut record = kept.clone();
        for (key, value) in &shape {
            record[*key] = value.clone();
        }
        made.write(&record);
        no(&made, NOT_CORES);
    }
    made.write(&kept);
    // A second wish made by hand to name this run has no say over it: the run is its own wish's to toss.
    let two = made.w.wish("alpha");
    let file = made.w.jm("pages").join(format!("{DAY}.wishes.json"));
    let mut wishes: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
    let theirs = made.w.kept(&made.wish);
    for wish in wishes["wishes"].as_array_mut().unwrap().iter_mut().filter(|wish| wish["id"] == two.as_str()) {
        (wish["run"], wish["state"], wish["why"]) = (theirs["run"].clone(), json!("done"), theirs["why"].clone());
    }
    fs::write(&file, format!("{wishes}\n")).unwrap();
    let before = made.held();
    let asked = ["run", "toss", "--wish", &two, "--date", DAY, "--repo", made.repo.to_str().unwrap()];
    let another = format!("wish {two} names run {{run}}, which is another wish's run: nothing was tossed");
    made.refuses(&made.w.run(&asked), &before, &another);
    assert_eq!(made.w.kept(&made.wish), theirs);
    fs::write(made.w.jm("runs").join(&made.run).join("run.json"), "{").unwrap();
    let ran = made.toss();
    assert_eq!(refused(&ran), said(&format!("the record of run {} cannot be read, so nothing was tossed", made.run)));
    assert_eq!((made.w.branches(&made.repo).len(), &made.w.kept(&made.wish)["state"]), (1, &json!("done")));
    made.write(&kept);
    then_tossed(&made);
}

#[test]
fn a_command_line_that_is_not_a_toss_s_is_refused_as_usage() {
    let made = finished();
    let before = made.held();
    for more in [&["--ask"][..], &["--with-settings"], &["--file", "1"], &["--force"], &["extra"]] {
        let asked = ["run", "toss", "--wish", &made.wish, "--date", DAY, "--repo", made.repo.to_str().unwrap()];
        let ran = made.w.run(&[&asked[..], more].concat());
        assert_eq!((ran.code, ran.stdout.as_str()), (2, ""), "{more:?}: {}", ran.stderr);
    }
    assert_eq!(made.w.run(&["run", "toss", "--date", DAY]).code, 2, "a toss takes a wish");
    assert!(before == made.held());
}
