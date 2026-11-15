//! What F51's gate left, proved at the binary: a last word that names a part twice, a credential that a line
//! break splits in any text the door takes, a toss that reads the record of a run that asks behind the run's
//! lock, and a keep that cannot ask whether a name of the person's points at the run's branch.

use std::fs;

use serde_json::json;

use super::{DAY, Made, asking, ended, failed, printed, refused, said, until, world};
use crate::support::{key_header, map_key, shows, tree};

const UNFIT: &str = "its last word was not `done` alone, or `asks` with one question";
const NOT_CORES: &str = "the record of run {run} is not one Jellymoon wrote for this wish";
const UNASKED: &str = "; the run's branch is left, as the repo could not be found or asked";
const MARKS: &str =
    "nothing is left of these words but the mark of what cleaning replaced, so nothing was made of them";

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
    assert!(made.record().get("work").is_none() && !super::held(&made));
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

/// A private key as it is typed, built here from pieces: its first line, two lines of its body set in, and its
/// last line. And those two lines of its body.
fn key_block() -> (String, [String; 2]) {
    let body = [format!("{}{}", "MIIEow", "IBAAKC"), format!("{}{}", "AQEAx7", "MIIEow")];
    let end = key_header(" ").replace("BEGIN", "END");
    (format!("{}\n  {}\n  {}\n  {end}", key_header(" "), body[0], body[1]), body)
}

// The word before the key and the start of the key's first line are a credential's shape once joined across
// the break. The door that read only that way lost the first line, and kept the key's body.
#[test]
fn a_word_on_the_line_before_a_private_key_keeps_nothing_of_the_key_in_a_wish_or_an_answer() {
    let (word, (block, body)) = ("aB3".repeat(10), key_block());
    let typed = format!("fix {word}\n{block}");
    let w = world();
    let wished = printed(&w.run(&["wish", "--date", DAY, "--say", &typed, "--project", "alpha"]));
    assert_eq!(wished["wish"]["said"], "fix [redacted]");
    let made = asking();
    made.w.play("nothing", 0);
    assert_eq!(printed(&made.answer(&typed))["outcome"], "completed");
    assert_eq!(made.w.rec("stdin").unwrap(), "fix [redacted]\n");
    assert_eq!(made.w.kept(&made.wish)["answer"], "fix [redacted]");
    for (world, part) in [&w, &made.w].into_iter().flat_map(|world| body.iter().map(move |part| (world, part))) {
        let held = tree(world.dir.path()).into_iter().find(|(_, bytes)| shows(&String::from_utf8_lossy(bytes), part));
        assert!(held.is_none(), "{:?}", held.map(|held| held.0));
    }
}

// Words that come out as nothing but marks say nothing: no command makes a wish, an answer or a ruling of them.
#[test]
fn words_of_which_cleaning_leaves_only_marks_are_refused_and_nothing_is_made_of_them() {
    let names = ["aB3".repeat(8), "cD4".repeat(8)];
    // Two long names on two lines are a credential's shape once joined, and take the words on the lines beside
    // them with them.
    let lines = format!("rename\n{}\n{}\nnow", names[0], names[1]);
    let made = asking();
    for typed in [map_key("kLmNo"), lines, key_block().0, format!("{0} {0}", map_key("pQrSt"))] {
        let before = (made.home(), made.w.starts());
        let wished = made.w.run(&["wish", "--date", DAY, "--say", &typed, "--project", "alpha"]);
        assert_eq!(refused(&wished), said(MARKS), "{typed:?}");
        for (stands, over) in [(typed.as_str(), "the other"), ("the other", typed.as_str())] {
            assert_eq!(refused(&made.w.run(&["rule", "--stands", stands, "--over", over])), said(MARKS), "{typed:?}");
        }
        made.unanswered(&typed, MARKS);
        assert!(before == (made.home(), made.w.starts()), "{typed:?}");
    }
    // The control: with a word beside them that is kept, they are a wish and an answer, as the mark.
    let typed = format!("fix these\n{}\n{}\nplease", names[0], names[1]);
    let wished = printed(&made.w.run(&["wish", "--date", DAY, "--say", &typed, "--project", "alpha"]));
    assert_eq!(wished["wish"]["said"], "fix [redacted]");
    made.w.play("nothing", 0);
    assert_eq!(printed(&made.answer(&typed))["outcome"], "completed");
    assert_eq!(made.w.rec("stdin").unwrap(), "fix [redacted]\n");
}
