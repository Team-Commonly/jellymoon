//! A toss of a made wish that has had no run (`docs/development/toss.md`, "A wish that has had no run"): the
//! wish is let go, behind the day's lock, and nothing else in the home or in any repo is touched. A run that
//! is started for it afterwards is refused, and one that named itself first is left to its own toss.

use std::ffi::OsString;
use std::time::Duration;
use std::{fs, thread};

use serde_json::{Value, json};

use super::{DAY, OLD, World, ended, printed, refused, said, wait_for, world};
use crate::support::{Ran, tree};

const LET_GO: &str = "wish {wish} had no run and was let go: nothing of a repo or a copy was touched";
const KEPT: &str = "the wish had no run, and was let go";
const BEFORE: &str = "wish {wish} was let go before, and nothing was changed";
const NO_RUN: &str = "wish {wish} was let go, so no run is started for it";

fn toss(w: &World, wish: &str, more: &[(&'static str, OsString)]) -> Ran {
    w.run_with(more, &["run", "toss", "--wish", wish, "--date", DAY])
}

/// What a toss of a wish with no run printed: a toss's five names, with no run and no commit.
fn let_go(wish: &str, changed: bool, why: &str) -> Value {
    json!({"run": null, "wish": wish, "tossed": changed, "commit": null, "why": why.replace("{wish}", wish)})
}

/// What a look says of the wish, before it is let go and after: it has had no run.
fn looks(w: &World, wish: &str, repo: &str) {
    let ask = |verb: &[&str]| w.run(&[&["run"], verb, &["--wish", wish, "--date", DAY, "--repo", repo]].concat());
    let none = |end: &str| said(&format!("wish {wish} has had no run, so there is no work to {end}"));
    assert_eq!(refused(&ask(&["show"])), none("show"));
    assert_eq!(refused(&ask(&["keep", "--ask"])), none("keep"));
    let stopped = w.run(&["run", "stop", "--wish", wish, "--date", DAY]);
    assert_eq!(refused(&stopped), said(&format!("wish {wish} has no run going, so nothing was stopped")));
    let follow = w.run(&["wish", "--date", DAY, "--say", "and one more, plum-lantern", "--after", wish]);
    assert_eq!(refused(&follow), said(&format!("wish {wish} has had no run, so there is no work to follow")));
}

#[test]
fn a_made_wish_that_has_had_no_run_is_let_go_by_a_toss_and_nothing_else_is_touched() {
    let w = world();
    w.play("edits", 0);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let at = repo.to_str().unwrap();
    looks(&w, &wish, at);
    assert_eq!(w.on_page(&wish)["state"], "working");
    let (person, was) = (w.persons(&repo), w.kept(&wish));
    let home = || -> Vec<_> {
        // But for the day's wishes and the lock of keeps and tosses, in the `runs` folder it is made in.
        let other = |(path, _): &(std::path::PathBuf, Vec<u8>)| {
            !path.ends_with("runs") && !path.ends_with("keep.lock") && !path.ends_with(format!("{DAY}.wishes.json"))
        };
        tree(&w.at("jm")).into_iter().filter(other).collect()
    };
    let before = home();
    assert_eq!(printed(&toss(&w, &wish, &[])), let_go(&wish, true, LET_GO));
    // The `runs` folder the toss made for its lock is the person's alone, as one a run makes is.
    #[cfg(unix)]
    {
        let mode = std::os::unix::fs::PermissionsExt::mode(&fs::metadata(w.jm("runs")).unwrap().permissions());
        assert_eq!(mode & 0o777, 0o700, "others can list the runs of this home");
    }
    // The wish is as it was but for its state and its line, and names no run: a page shows it let go.
    let mut now = was.clone();
    (now["state"], now["why"]) = (json!("let_go"), json!(KEPT));
    assert_eq!(w.kept(&wish), now);
    assert!(person == w.persons(&repo) && before == home(), "a toss of a wish with no run touched something else");
    assert_eq!(w.on_page(&wish)["state"], "let_go");
    assert_eq!((w.branches(&repo).len(), w.copies().len(), w.starts()), (0, 0, 0));
    // Asked again, and with a folder named, it changes nothing and says so.
    for more in [&[][..], &["--repo", at]] {
        let again = w.run(&[&["run", "toss", "--wish", &wish, "--date", DAY][..], more].concat());
        assert_eq!(printed(&again), let_go(&wish, false, BEFORE));
    }
    assert_eq!(w.kept(&wish), now);
    looks(&w, &wish, at);
    // No run is started for it: said before a folder or a harness is looked for, with nothing made.
    let (nowhere, before) = (w.at("nowhere"), home());
    for folder in [repo.as_path(), nowhere.as_path()] {
        assert_eq!(refused(&w.start(&wish, folder)), said(&NO_RUN.replace("{wish}", &wish)));
    }
    assert_eq!((w.kept(&wish), w.branches(&repo).len(), w.copies().len(), w.starts()), (now, 0, 0, 0));
    assert!(before == home(), "a refused run left something in the home");
    // It is one of the day's eight still, as a wish let go with its tossed run is.
    let made: Vec<String> = (0..7).map(|_| w.wish("alpha")).collect();
    assert_eq!(made.last().map(String::as_str), Some("m8"));
    let ninth = w.run(&["wish", "--date", DAY, "--say", "one more", "--project", "alpha"]);
    assert!(refused(&ninth).contains("has its 8 made wishes already"), "{}", ninth.stderr);
}

#[test]
fn a_wish_with_no_run_is_not_let_go_while_a_keep_or_a_toss_holds_the_home() {
    let w = world();
    let (_, wish) = (w.repo("alpha"), w.wish("alpha"));
    let was = w.kept(&wish);
    fs::create_dir_all(w.jm("runs")).unwrap();
    let file = w.jm("runs").join("keep.lock");
    let lock = fs::File::create(&file).unwrap();
    lock.lock().unwrap();
    let busy = "a keep, a toss or the start of a follow-up's run is at work in this home: wish {wish} was not let go";
    let held = toss(&w, &wish, &[("JELLYMOON_TEST_KEEP_WAIT_MS", "200".into())]);
    assert_eq!((refused(&held), w.kept(&wish)), (said(&busy.replace("{wish}", &wish)).as_str(), was.clone()));
    drop(lock);
    // A lock that cannot be made is not another command at work, and is said in its own words.
    fs::remove_file(&file).unwrap();
    fs::create_dir(&file).unwrap();
    let unmade = "the lock a keep or a toss holds (runs/keep.lock) could not be made or taken in this home: wish \
                  {wish} was not let go";
    assert_eq!(refused(&toss(&w, &wish, &[])), said(&unmade.replace("{wish}", &wish)));
    assert_eq!(w.kept(&wish), was);
    fs::remove_dir(&file).unwrap();
    assert_eq!(printed(&toss(&w, &wish, &[])), let_go(&wish, true, LET_GO));
}

// Another change to the day that takes a while is waited for, past the three seconds the day's lock itself
// waits: the let-go is done when that change has finished, and is not refused for it.
#[test]
fn a_let_go_waits_for_another_change_to_the_day_and_is_done_when_that_has_finished() {
    let w = world();
    let (_, wish) = (w.repo("alpha"), w.wish("alpha"));
    let lock = w.jm("pages").join(format!("{DAY}.ruled.lock"));
    fs::write(&lock, "").unwrap();
    let ran = thread::scope(|scope| {
        let going = scope.spawn(|| toss(&w, &wish, &[]));
        thread::sleep(Duration::from_secs(4));
        fs::remove_file(&lock).unwrap();
        going.join().unwrap()
    });
    assert_eq!(printed(&ran), let_go(&wish, true, LET_GO));
    assert_eq!(w.kept(&wish)["state"], "let_go");
}

#[test]
fn a_wish_whose_run_was_never_started_is_let_go_as_one_that_was_never_run() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    w.play("old", 0);
    assert_eq!(printed(&w.start(&wish, &repo))["outcome"], "not_started");
    let was = w.kept(&wish);
    assert_eq!((&was["state"], &was["why"], was.get("run")), (&json!("needs_you"), &json!(OLD), None));
    assert_eq!(printed(&toss(&w, &wish, &[])), let_go(&wish, true, LET_GO));
    let mut now = was;
    (now["state"], now["why"]) = (json!("let_go"), json!(KEPT));
    assert_eq!((w.kept(&wish), w.starts(), w.branches(&repo).len(), w.copies().len()), (now, 1, 0, 0));
}

#[test]
fn a_run_that_is_being_started_and_a_toss_of_its_wish_leave_one_of_the_two_done() {
    // The toss is first: the run, held with its folder found and nothing made, is refused when it comes to
    // name itself, and leaves no copy, no branch and no record.
    let w = world();
    w.play("edits", 0);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let go = w.at("placed");
    let held = [("JELLYMOON_TEST_PLACE_WAIT_MS", OsString::from("60000")), ("JELLYMOON_TEST_GO", go.clone().into())];
    let late = w.going(&held, &["run", "--wish", &wish, "--date", DAY, "--repo", repo.to_str().unwrap()]);
    wait_for(&go.with_extension("at"));
    assert_eq!(printed(&toss(&w, &wish, &[])), let_go(&wish, true, LET_GO));
    let now = w.kept(&wish);
    fs::write(&go, "").unwrap();
    assert_eq!(refused(&ended(late)), said(&NO_RUN.replace("{wish}", &wish)));
    assert_eq!((w.kept(&wish), w.branches(&repo).len(), w.copies().len(), w.starts()), (now, 0, 0, 0));
    // A run's folder holds its record and its locks; the one folder git is started in is kept empty.
    let holds = |run: &fs::DirEntry| fs::read_dir(run.path()).is_ok_and(|inside| inside.count() > 0);
    let runs = fs::read_dir(w.jm("runs")).map_or(0, |runs| runs.flatten().filter(holds).count());
    assert_eq!(runs, 0, "a run that was refused left its folder");
    // The toss is first by less: the run holds its own lock and has its copy, and has not yet named itself.
    // It is refused behind the day's lock, and its copy and its branch are taken back.
    let w = world();
    w.play("edits", 0);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let go = w.at("claimed");
    let held = [("JELLYMOON_TEST_NAME_WAIT_MS", OsString::from("60000")), ("JELLYMOON_TEST_GO", go.clone().into())];
    let late = w.going(&held, &["run", "--wish", &wish, "--date", DAY, "--repo", repo.to_str().unwrap()]);
    wait_for(&go.with_extension("at"));
    assert_eq!(printed(&toss(&w, &wish, &[])), let_go(&wish, true, LET_GO));
    let now = w.kept(&wish);
    fs::write(&go, "").unwrap();
    assert_eq!(refused(&ended(late)), said(&NO_RUN.replace("{wish}", &wish)));
    assert_eq!((w.kept(&wish), w.branches(&repo).len(), w.copies().len(), w.starts()), (now, 0, 0, 0));
    // The run is first: the toss, held after it found the wish with no run, changes nothing and names the run.
    let w = world();
    w.play("edits", 0);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let go = w.at("found");
    let held = [("JELLYMOON_TEST_TOSS_WAIT_MS", OsString::from("60000")), ("JELLYMOON_TEST_GO", go.clone().into())];
    let late = w.going(&held, &["run", "toss", "--wish", &wish, "--date", DAY]);
    wait_for(&go.with_extension("at"));
    let ran = printed(&w.start(&wish, &repo));
    let (run, now) = (ran["run"].as_str().unwrap(), w.kept(&wish));
    assert_eq!((&ran["outcome"], &now["state"]), (&json!("completed"), &json!("done")));
    fs::write(&go, "").unwrap();
    let by_now = format!("wish {wish} has a run by now, {run}: nothing was changed, and `run toss` is asked again");
    assert_eq!(refused(&ended(late)), said(&by_now));
    assert_eq!((w.kept(&wish), w.branches(&repo).len()), (now, 1));
    // Asked again, it is a toss of that run's work.
    let tossed = printed(&w.run(&["run", "toss", "--wish", &wish, "--date", DAY, "--repo", repo.to_str().unwrap()]));
    assert_eq!((&tossed["run"], &tossed["tossed"], w.branches(&repo).len()), (&json!(run), &json!(true), 0));
}
