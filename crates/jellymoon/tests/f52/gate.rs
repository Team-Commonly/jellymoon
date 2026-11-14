//! What F51's gate left, proved at the binary: a last word that names a part twice, a credential that a line
//! break splits in any text the door takes, a toss that reads the record of a run that asks behind the run's
//! lock, and a keep that cannot ask whether a name of the person's points at the run's branch.

use std::fs;

use serde_json::json;

use super::{DAY, Made, asking, ended, failed, printed, refused, said, until, world};
use crate::support::{map_key, shows, tree};

const UNFIT: &str = "its last word was not `done` alone, or `asks` with one question";
const NOT_CORES: &str = "the record of run {run} is not one Jellymoon wrote for this wish";
const UNASKED: &str = "; the run's branch is left, as the repo could not be found or asked";

// `{"state":"asks","state":"done"}` read by a reader that keeps the last of two is `done`, and the run's work
// would be committed. It is no last word of a run's.
#[test]
fn a_last_word_that_names_a_part_twice_fails_the_run_and_nothing_is_committed() {
    for number in [10, 11, 12] {
        let w = world();
        w.play("word", number);
        let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
        let said = printed(&w.start(&wish, &repo));
        let made = Made { run: said["run"].as_str().unwrap().to_string(), w, repo, wish };
        assert_eq!(said, failed(&made, UNFIT), "word {number}");
        assert_eq!(made.w.commit(&made.repo, &made.branch()), made.w.commit(&made.repo, "main"), "word {number}");
        assert!(made.copy_failed() && made.copy().join("n.txt").is_file(), "word {number}");
    }
}

// The door is the one a made wish passes: a wish typed with a credential over two lines keeps the mark.
#[test]
fn a_credential_split_by_a_line_break_in_a_made_wish_is_replaced_and_two_words_on_two_lines_are_kept() {
    let w = world();
    let key = map_key("kLmNo");
    let (head, tail) = key.split_at(20);
    let typed = format!("rotate {head}\n{tail} today");
    let made = printed(&w.run(&["wish", "--date", DAY, "--say", &typed, "--project", "alpha"]));
    assert_eq!(made["wish"]["said"], "rotate [redacted] today");
    let held = tree(&w.at("jm")).into_iter().find(|(_, bytes)| shows(&String::from_utf8_lossy(bytes), &key));
    assert!(held.is_none(), "{:?}", held.map(|held| held.0));
    // The control: words that are no credential when joined stay two words.
    let made = printed(&w.run(&["wish", "--date", DAY, "--say", "fix the upload\nretry", "--project", "alpha"]));
    assert_eq!(made["wish"]["said"], "fix the upload retry");
}

// A test's build waits in a toss between its first read of the record and the run's lock. The record is
// changed there, by hand, to one that is not core's: the toss goes by what it reads behind the lock.
#[test]
fn a_toss_reads_the_record_of_a_run_that_asks_again_once_it_holds_the_run_s_lock() {
    let made = asking();
    let (go, at) = (made.w.at("rec").join("go.step"), made.w.at("rec").join("go.at"));
    let hold = [("JELLYMOON_TEST_TOSS_LOCK_WAIT_MS", "60000".into()), ("JELLYMOON_TEST_GO", go.clone().into())];
    let asked = ["run", "toss", "--wish", &made.wish, "--date", DAY, "--repo", made.repo.to_str().unwrap()];
    let going = made.w.going(&hold, &asked);
    until("the toss never read the record", || at.exists());
    let mut record = made.record();
    record["question"] = json!(" Which?");
    made.write(&record);
    fs::write(&go, "").unwrap();
    assert_eq!(refused(&ended(going)), said(&NOT_CORES.replace("{run}", &made.run)));
    assert_eq!((made.w.copies().len(), made.w.branches(&made.repo).len()), (1, 1), "nothing was tossed");
    assert!(made.record().get("work").is_none() && !made.w.jm("runs").join(&made.run).join("run.lock").exists());
}

// The made-up git fails the one question a keep asks of the names that point at the run's branch. The work is
// kept all the same, the branch is left, and the line says why in a toss's words for it.
#[test]
fn a_keep_that_cannot_ask_what_points_at_the_run_s_branch_leaves_it_and_says_so() {
    let made = super::kept::finished();
    let (vars, fake) = (made.w.fake_git(), made.w.at("fake"));
    fs::write(fake.join("on"), "--format=%(symref)").unwrap();
    fs::write(fake.join("returns"), "").unwrap();
    fs::write(fake.join("fails"), "128").unwrap();
    let ran = made.keep_in(&made.repo, &[], &vars);
    assert!(fake.join("held").exists(), "the made-up git was never asked what points at the branch");
    let kept = printed(&ran);
    let why = kept["why"].as_str().unwrap();
    assert!(why.ends_with(UNASKED) && kept["kept"] == json!(made.end()), "{why}");
    assert_eq!(made.w.branches(&made.repo), [format!("jellymoon/run-{}", made.run)], "the branch is left");
    assert_eq!(made.record()["work"], "kept");
    // The control: the same keep with a git that answers deletes the branch and says nothing of it.
    let made = super::kept::finished();
    let kept = printed(&made.keep(&[]));
    assert!(!kept["why"].as_str().unwrap().contains("is left") && made.w.branches(&made.repo).is_empty());
}
