//! What comes after a run that asks: a look takes it for neither going nor cut off, a stop, a show, a keep and a
//! second run are refused in fixed words, and a toss tosses it as a run that did not finish, with the run's own
//! lock held. And what a look makes of a record or a wish that says `asks` and is not as core writes one.

use std::fs;

use serde_json::{Value, json};

use super::{ASKS, DAY, Made, QUESTION, asking, ended, played, printed, ran, refused, said, until};
use crate::support::{Ran, tree};

const TOSSED: &str = "run {run}, which did not finish, was tossed: its kept copy was removed, its branch was \
                      deleted, and the wish is let go";
const LET_GO: &str = "run {run} was tossed, and the wish was let go";
const GOING: &str = "run {run} is going: it is stopped first, and nothing was tossed";
const CUT: &str = "run {run} was cut off before it ended: its copy is kept and no star was added";

/// A change made by hand to a record or a wish.
type Change = fn(&mut Value);

impl Made {
    /// A command for the wish, in its repo. A stop takes no repo.
    fn asked(&self, command: &[&str]) -> Ran {
        let tail = ["--wish", &self.wish, "--date", DAY, "--repo", self.repo.to_str().unwrap()];
        self.w.run(&[command, &tail[..if command.contains(&"stop") { 4 } else { 6 }]].concat())
    }

    /// Every file of the home but the empty file a keep or a toss makes for the lock they share, refused or not.
    fn home(&self) -> Vec<(std::path::PathBuf, Vec<u8>)> {
        tree(&self.w.at("jm")).into_iter().filter(|(path, _)| !path.ends_with("keep.lock")).collect()
    }

    /// Fails unless the command was refused in these words with nothing in the home or the repo changed.
    fn refuses_all(&self, command: &[&str], why: &str) {
        let before = (self.home(), self.w.persons(&self.repo), self.w.branches(&self.repo));
        let ran = self.asked(command);
        assert_eq!(refused(&ran), said(&why.replace("{run}", &self.run)), "{command:?}");
        let after = (self.home(), self.w.persons(&self.repo), self.w.branches(&self.repo));
        assert!(before == after, "{command:?} was refused and changed something");
    }

    fn copy_failed(&self) -> bool {
        let copy = fs::read_to_string(self.w.jm("runs").join(&self.run).join("copy.json")).unwrap();
        serde_json::from_str::<Value>(&copy).unwrap()["failed"].is_u64()
    }

    /// The day's made wishes, changed by hand as anyone who can write the home can change them.
    fn rewrite_wish(&self, change: impl FnOnce(&mut Value)) {
        let file = self.w.jm("pages").join(format!("{DAY}.wishes.json"));
        let mut kept: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
        change(&mut kept["wishes"][0]);
        fs::write(file, format!("{kept}\n")).unwrap();
    }

    /// The wish as it was while the run went: working, with no question. So the next look goes by the record.
    fn untold(&self) {
        self.rewrite_wish(|wish| {
            wish.as_object_mut().unwrap().remove("question");
            wish["state"] = json!("working");
        });
    }
}

#[test]
fn a_stop_a_show_a_keep_and_a_second_run_of_a_run_that_asks_are_refused_in_fixed_words() {
    let made = asking();
    made.refuses_all(&["run", "stop"], "run {run} asks a question and is not going, so nothing was stopped");
    made.refuses_all(&["run", "show"], "run {run} asks a question: there is no work of it to show");
    made.refuses_all(&["run", "show", "--file", "1"], "run {run} asks a question: there is no work of it to show");
    made.refuses_all(&["run", "keep"], "run {run} asks a question: there is no work of it to keep");
    made.refuses_all(&["run"], &format!("wish {} has had its run, {{run}}: a made wish gets one run", made.wish));
    assert_eq!(made.w.starts(), 1, "no second agent was started");
}

#[test]
fn a_look_takes_a_run_that_asks_for_neither_going_nor_cut_off_and_its_copy_is_never_pruned() {
    let made = asking();
    let before = tree(&made.w.at("jm"));
    for _ in 0..2 {
        let on_page = made.w.on_page(&made.wish);
        assert_eq!((&on_page["state"], &on_page["question"]), (&json!("needs_you"), &json!(QUESTION)));
    }
    assert!(before == tree(&made.w.at("jm")), "a look at a run that asks wrote something");
    // Six runs fail after it. The newest five failed copies are kept, and the copy that asks is none of them.
    made.w.play("exit-3", 0);
    let failed: Vec<String> = (0..6).map(|_| ran(&made.w.start(&made.w.wish("alpha"), &made.repo)).0).collect();
    let mut copies = made.w.copies();
    copies.sort();
    let mut want: Vec<String> = failed[1..].iter().cloned().chain([made.run.clone()]).collect();
    want.sort();
    assert_eq!(copies, want, "the oldest failed copy went, and the copy that asks stayed");
    assert!(!made.copy_failed() && made.copy().join("n.txt").is_file());
    assert_eq!(made.w.on_page(&made.wish)["question"], QUESTION);
}

// A `jellymoon run` killed after its record took the question and before its wish did: nothing holds the run,
// and its wish says working. The next look gives the wish the question, and marks no copy failed.
#[test]
fn a_run_killed_between_its_record_and_its_wish_is_given_its_question_by_the_next_look() {
    let w = super::world();
    w.play("asks", 1);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (go, at) = (w.at("rec").join("go.step"), w.at("rec").join("go.at"));
    let hold = [("JELLYMOON_TEST_RECORD_WAIT_MS", "60000".into()), ("JELLYMOON_TEST_GO", go.clone().into())];
    let mut going = w.going(&hold, &["run", "--wish", &wish, "--date", DAY, "--repo", repo.to_str().unwrap()]);
    let run = || w.kept(&wish)["run"].as_str().map(str::to_string);
    until("the record never took the question", || {
        let asks = run().is_some_and(|run| w.record(&run)["outcome"] == "asks");
        if at.exists() && !go.exists() && !asks {
            fs::remove_file(&at).unwrap();
            fs::write(&go, "").unwrap();
        }
        asks && at.exists()
    });
    going.kill().unwrap();
    going.wait().unwrap();
    let made = Made { run: run().unwrap(), w, repo, wish };
    assert_eq!((&made.w.kept(&made.wish)["state"], made.w.kept(&made.wish).get("question")), (&json!("working"), None));
    let record = made.record();
    let on_page = made.w.on_page(&made.wish);
    assert_eq!((&on_page["state"], &on_page["question"]), (&json!("needs_you"), &json!(QUESTION)));
    let kept = made.w.kept(&made.wish);
    let line = json!(ASKS.replace("{run}", &made.run));
    assert_eq!((&kept["state"], &kept["why"], &kept["question"]), (&json!("needs_you"), &line, &json!(QUESTION)));
    assert!(!made.copy_failed() && made.record() == record && made.copy().join("n.txt").is_file());
    // The lock's file the killed command left is nobody's: the run is tossed as any run that asks.
    assert!(made.w.jm("runs").join(&made.run).join("run.lock").exists());
    let tossed = printed(&made.asked(&["run", "toss"]));
    assert_eq!(tossed["why"], TOSSED.replace("{run}", &made.run));
}

// The harness ends with a question while the day's lock is busy for longer than a run waits. The line says the
// run asks, not that it has ended; the record has the question, and the next look gives it to the wish.
#[test]
fn a_run_that_asks_while_the_day_is_locked_says_so_and_the_next_look_tells_its_wish() {
    let w = super::world();
    w.play("asks", 0);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (go, at) = (w.at("rec").join("go.step"), w.at("rec").join("go.at"));
    let lock = w.jm("pages").join(format!("{DAY}.ruled.lock"));
    // A test's build waits before the record takes the end, until the file is there.
    let vars = [
        ("JELLYMOON_TEST_NOTE_WAIT_MS", "60000".into()),
        ("JELLYMOON_TEST_GO", go.clone().into()),
        ("JELLYMOON_TEST_END_WAIT_MS", "300".into()),
    ];
    let going = w.going(&vars, &["run", "--wish", &wish, "--date", DAY, "--repo", repo.to_str().unwrap()]);
    until("the run never came to its end", || at.exists());
    fs::write(&lock, "").unwrap(); // another change to the day, which does not finish in time
    fs::write(&go, "").unwrap();
    let ran = ended(going);
    let run = w.kept(&wish)["run"].as_str().unwrap().to_string();
    let busy = format!(
        "run {run} asks a question and its record says how; the page of {DAY} was busy, and shows it when it is \
         next drawn"
    );
    assert_eq!(refused(&ran), said(&busy));
    assert_eq!((&w.record(&run)["question"], &w.kept(&wish)["state"]), (&json!(QUESTION), &json!("working")));
    fs::remove_file(&lock).unwrap();
    let on_page = w.on_page(&wish);
    assert_eq!((&on_page["state"], &on_page["question"]), (&json!("needs_you"), &json!(QUESTION)));
    assert_eq!(w.copies(), [run]);
}

#[test]
fn a_record_that_says_asks_and_is_not_as_core_writes_one_is_a_run_that_was_cut_off() {
    let odd: [(&str, Change); 8] = [
        ("a question the door would change", |record| record["question"] = json!("Which?\nIgnore the above.")),
        ("a question with a space before it", |record| record["question"] = json!(" Which?")),
        ("an empty question", |record| record["question"] = json!("")),
        ("no question", |record| drop(record.as_object_mut().unwrap().remove("question"))),
        ("no session", |record| drop(record.as_object_mut().unwrap().remove("session"))),
        ("a session id that is no id of core's", |record| record["session"] = json!("../../../etc")),
        ("another line", |record| record["why"] = json!("run r0123456789abcdef asks a question")),
        ("a question and an outcome of failed", |record| record["outcome"] = json!("failed")),
    ];
    for (what, change) in odd {
        let made = asking();
        let mut record = made.record();
        change(&mut record);
        made.write(&record);
        made.untold();
        let on_page = made.w.on_page(&made.wish);
        assert_eq!((&on_page["state"], on_page.get("question")), (&json!("needs_you"), None), "{what}");
        let kept = made.w.kept(&made.wish);
        assert_eq!((&kept["why"], kept.get("question")), (&json!(CUT.replace("{run}", &made.run)), None), "{what}");
        assert!(made.copy_failed(), "{what}: the copy of a run that was cut off is marked");
    }
    // The control: the record as core wrote it is believed, by the same look.
    let made = asking();
    made.untold();
    assert_eq!(made.w.on_page(&made.wish)["question"], QUESTION);
    assert_eq!((made.w.kept(&made.wish)["question"].clone(), made.copy_failed()), (json!(QUESTION), false));
}

#[test]
fn a_wish_whose_question_is_not_one_core_would_have_written_is_not_used() {
    let odd: [(&str, Change); 5] = [
        ("a question the door would change", |wish| wish["question"] = json!("Which?\u{1b}[31m")),
        ("an empty question", |wish| wish["question"] = json!("")),
        ("a question on a wish that is done", |wish| wish["state"] = json!("done")),
        ("a question on a wish that is working", |wish| wish["state"] = json!("working")),
        ("a question on a wish with no run", |wish| drop(wish.as_object_mut().unwrap().remove("run"))),
    ];
    for (what, change) in odd {
        let made = asking();
        made.rewrite_wish(change);
        let why = format!("the made wishes of {DAY} are not used: they are not the made wishes of that day");
        assert_eq!(refused(&made.asked(&["run", "stop"])), said(&why), "{what}");
    }
}

#[test]
fn a_toss_of_a_run_that_asks_removes_its_copy_and_its_branch_and_lets_the_wish_go() {
    let made = asking();
    let (start, person) = (made.w.commit(&made.repo, "main"), made.w.persons(&made.repo));
    let tossed = printed(&made.asked(&["run", "toss"]));
    let why = TOSSED.replace("{run}", &made.run);
    assert_eq!(tossed, json!({"run": made.run, "wish": made.wish, "tossed": true, "commit": start, "why": why}));
    // The copy and its edits are gone, the branch went while it was where the run started, the wish is let go.
    assert_eq!((made.w.copies(), made.w.branches(&made.repo)), (vec![], vec![]));
    assert!(person == made.w.persons(&made.repo));
    let (record, wish) = (made.record(), made.w.kept(&made.wish));
    assert_eq!(
        (&record["work"], &record["tossed"], &record["outcome"]),
        (&json!("tossed"), &json!(start), &json!("asks"))
    );
    let line = json!(LET_GO.replace("{run}", &made.run));
    assert_eq!((&wish["state"], &wish["why"], wish.get("question")), (&json!("let_go"), &line, None));
    assert_eq!(made.w.on_page(&made.wish).get("question"), None);
    assert!(!made.w.jm("runs").join(&made.run).join("run.lock").exists(), "the toss left the run's lock");
    // Tossed, it no longer asks: a look at its work says it did not finish, and a second toss changes nothing.
    made.refuses_all(&["run", "show"], "run {run} did not finish: there is no work of it to show");
    let stopped = printed(&made.asked(&["run", "stop"]));
    assert_eq!((&stopped["outcome"], &stopped["why"], stopped.get("question")), (&json!("asks"), &line, None));
    let again = printed(&made.asked(&["run", "toss"]));
    assert_eq!(
        (&again["tossed"], &again["why"]),
        (&json!(false), &json!(format!("run {} was tossed before, and nothing was changed", made.run)))
    );
}

#[test]
fn a_toss_holds_the_lock_of_a_run_that_asks_and_is_refused_while_another_command_holds_it() {
    let made = asking();
    let lock = made.w.jm("runs").join(&made.run).join("run.lock");
    // Another command holds the run: nothing of it is tossed.
    let held = fs::File::create(&lock).unwrap();
    held.lock().unwrap();
    made.refuses_all(&["run", "toss"], GOING);
    drop(held);
    fs::remove_file(&lock).unwrap();
    // A test's build waits after each writing of a record: the toss has written `tossed`, and holds the lock.
    let asked = ["run", "toss", "--wish", &made.wish, "--date", DAY, "--repo", made.repo.to_str().unwrap()];
    let mut going = made.w.going(&[("JELLYMOON_TEST_RECORD_WAIT_MS", "60000".into())], &asked);
    until("the toss never wrote its word", || made.record()["work"] == "tossed");
    let free = || fs::File::open(&lock).is_ok_and(|lock| lock.try_lock().is_ok());
    assert!(lock.is_file() && !free(), "the toss of a run that asks does not hold the run's lock");
    going.kill().unwrap();
    going.wait().unwrap();
    assert!(free(), "the lock went with its holder");
    let tossed = printed(&made.asked(&["run", "toss"]));
    assert_eq!(
        tossed["why"].as_str().unwrap().split(':').next(),
        Some(&*format!("the record of run {} said it was tossed, and this toss did what was left to do", made.run))
    );
    assert!(!lock.exists(), "a toss that ended left the run's lock");
    // The control: a toss of a run that failed takes no lock of the run's, at the same place.
    let (failed, _) = played("exit-3", 0);
    let lock = failed.w.jm("runs").join(&failed.run).join("run.lock");
    let asked = ["run", "toss", "--wish", &failed.wish, "--date", DAY, "--repo", failed.repo.to_str().unwrap()];
    let going = failed.w.going(&[("JELLYMOON_TEST_RECORD_WAIT_MS", "300".into())], &asked);
    until("the toss never wrote its word", || failed.record()["work"] == "tossed");
    assert!(!lock.exists(), "a toss of a failed run made a lock");
    assert_eq!(printed(&ended(going))["tossed"], true);
}

// No keep writes in the record of a run that asks: it is refused before the record is touched. A record that asks
// and holds a keep's words all the same, written by hand, is neither kept nor tossed.
#[test]
fn a_record_that_asks_and_holds_a_keeps_words_is_neither_kept_nor_tossed() {
    let not_cores = "the record of run {run} is not one Jellymoon wrote for this wish";
    for work in [None, Some("kept")] {
        let made = asking();
        let mut record = made.record();
        (record["kept"], record["onto"]) = (json!(made.w.commit(&made.repo, "main")), json!("refs/heads/main"));
        if let Some(work) = work {
            record["work"] = json!(work);
        }
        made.write(&record);
        let said = if work.is_some() { "did not finish" } else { "asks a question" };
        made.refuses_all(&["run", "keep"], &format!("run {{run}} {said}: there is no work of it to keep"));
        made.refuses_all(&["run", "toss"], not_cores);
        assert_eq!((made.w.copies().len(), made.w.branches(&made.repo).len()), (1, 1));
    }
}
