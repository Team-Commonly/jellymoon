//! The other ends of an answered run: a harness that no longer keeps the session, a failure, a stop, a harness
//! too old to go on with a session, and an answer that was killed half way.

use std::fs;

use serde_json::json;

use super::{ANSWER, ASKS, QUESTION, asking, ended, failed, printed, refused, said, until};

const LOST: &str = "Claude Code no longer keeps the session it asked in";
const CUT: &str = "run {run} was cut off before it ended: its copy is kept and no star was added";
const NOT_ASKING: &str = "run {run} does not ask a question, so there is nothing to answer";

#[test]
fn a_harness_that_no_longer_keeps_the_session_fails_the_run_in_fixed_words_with_its_copy_kept() {
    // 3: a stream that did not start as a run's is ended at once, so the exit of 0 it would have had is not asked.
    for number in 0..4 {
        let made = asking();
        fs::write(made.copy().join("half.txt"), "left by the first turn\n").unwrap();
        made.w.play("no-session", number);
        let ran = made.answer(ANSWER);
        assert_eq!(printed(&ran), failed(&made, LOST), "no-session {number}");
        made.w.never_says_the_agent(&ran);
        assert!(made.keeps("No conversation").is_none(), "the harness's words are kept nowhere");
        // The copy is kept as a failed run's is, with what was in it, and nothing was committed.
        assert!(made.copy_failed() && made.copy().join("half.txt").is_file(), "no-session {number}");
        assert_eq!(made.w.commit(&made.repo, &made.branch()), made.w.commit(&made.repo, "main"));
        let (record, wish) = (made.record(), made.w.kept(&made.wish));
        let line = failed(&made, LOST)["why"].clone();
        assert_eq!((&record["outcome"], &record["why"], record.get("question")), (&json!("failed"), &line, None));
        assert_eq!((&wish["state"], &wish["why"], wish.get("question")), (&json!("needs_you"), &line, None));
        // It asks no more: a second answer is refused, and a toss lets the wish go.
        made.unanswered(ANSWER, NOT_ASKING);
        assert_eq!(printed(&made.toss())["tossed"], true);
    }
    // Said with an exit of 0 after a stream that started as a run's, they are not believed: the run failed as any
    // run whose result is an error.
    let made = asking();
    made.w.play("no-session", 4);
    let ran = printed(&made.answer(ANSWER));
    assert!(ran["outcome"] == "failed" && !ran["why"].as_str().unwrap().contains(LOST), "{ran}");
    // Said by a first start, which was told to go on with nothing, they are a failed run's too.
    let w = super::world();
    w.play("no-session", 0);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let ran = printed(&w.start(&wish, &repo));
    assert!(ran["outcome"] == "failed" && !ran["why"].as_str().unwrap().contains(LOST), "{ran}");
    // The control: the same words from a harness that goes on to finish change nothing.
    let made = asking();
    made.w.play("edits", 0);
    assert_eq!(printed(&made.answer("No conversation found with session ID"))["outcome"], "completed");
}

#[test]
fn an_answered_run_that_fails_keeps_its_copy_and_one_that_is_stopped_is_cancelled() {
    let made = asking();
    made.w.play("exit-3", 0);
    assert_eq!(printed(&made.answer(ANSWER)), failed(&made, "Claude Code exited with a failure after its result"));
    assert!(made.copy_failed() && made.copy().join("n.txt").is_file());
    assert_eq!(made.w.commit(&made.repo, &made.branch()), made.w.commit(&made.repo, "main"));
    // A stop of an answered run that is going ends it as cancelled, as it ends any run.
    let made = asking();
    made.w.play("forever", 0);
    let going = made.answering(&[], ANSWER);
    until("the answered run never started its agent", || made.w.at("rec").join("started").is_file());
    let stopped = printed(&made.w.run(&["run", "stop", "--wish", &made.wish, "--date", super::DAY]));
    let line = format!("run {} was cancelled by a stop: its copy is kept and no star was added", made.run);
    assert_eq!((&stopped["outcome"], &stopped["why"]), (&json!("cancelled"), &json!(line)));
    assert_eq!(printed(&ended(going))["outcome"], "cancelled");
    assert!(made.w.freed("held") && made.copy_failed());
}

// A stop asked of the first turn just as it ended with its question was never read. It does not stop the answer.
#[test]
fn a_stop_left_from_before_the_question_does_not_stop_the_answered_run() {
    let made = asking();
    fs::write(made.w.jm("runs").join(&made.run).join("stop"), "").unwrap();
    made.w.play("edits", 0);
    assert_eq!(printed(&made.answer(ANSWER))["outcome"], "completed");
}

#[test]
fn a_harness_too_old_to_go_on_with_a_session_starts_nothing_and_the_run_still_asks() {
    let made = asking();
    let before = made.home();
    made.w.play("old-option", 3);
    let ran = made.answer(ANSWER);
    let why = "nothing was started: this Claude Code is too old to take an answer, as it does not know --resume; run \
               {run} still asks its question";
    assert_eq!(refused(&ran), said(&why.replace("{run}", &made.run)));
    assert!(before == made.home(), "the record, the wish and the copy are as they were before the answer");
    assert_eq!(made.w.on_page(&made.wish)["question"], QUESTION);
    // The answer is taken once the harness can.
    made.w.play("edits", 0);
    assert_eq!(printed(&made.answer(ANSWER))["outcome"], "completed");
}

// A test's build waits once the wish says working and before the record does: the command is killed there. The
// record still asks, nothing holds the run, and the next look gives the wish its question back.
#[test]
fn an_answer_killed_between_its_wish_and_its_record_leaves_a_run_that_still_asks() {
    let made = asking();
    let record = made.record();
    let (go, at) = (made.w.at("rec").join("go.step"), made.w.at("rec").join("go.at"));
    let hold = [("JELLYMOON_TEST_ANSWER_WAIT_MS", "60000".into()), ("JELLYMOON_TEST_GO", go.into())];
    let mut going = made.answering(&hold, ANSWER);
    until("the answer never came to its record", || at.exists());
    assert_eq!((&made.w.kept(&made.wish)["state"], made.record() == record), (&json!("working"), true));
    going.kill().unwrap();
    going.wait().unwrap();
    let on_page = made.w.on_page(&made.wish);
    assert_eq!((&on_page["state"], &on_page["question"]), (&json!("needs_you"), &json!(QUESTION)));
    let wish = made.w.kept(&made.wish);
    let line = json!(ASKS.replace("{run}", &made.run));
    assert_eq!((&wish["why"], &wish["question"], &wish["answer"]), (&line, &json!(QUESTION), &json!(ANSWER)));
    assert!(!made.copy_failed() && made.record() == record && made.w.starts() == 1);
    made.w.play("edits", 0);
    assert_eq!(printed(&made.answer("the old one"))["outcome"], "completed");
}

// Killed once the record says the run goes again, the answered run was cut off as any run is: the next look
// writes it down as failed, and marks its copy.
#[test]
fn an_answer_killed_after_its_record_was_written_is_a_run_that_was_cut_off() {
    let made = asking();
    let (go, at) = (made.w.at("rec").join("go.step"), made.w.at("rec").join("go.at"));
    let hold = [("JELLYMOON_TEST_RECORD_WAIT_MS", "60000".into()), ("JELLYMOON_TEST_GO", go.into())];
    let mut going = made.answering(&hold, ANSWER);
    until("the answer never wrote its record", || at.exists());
    assert!(made.record()["outcome"].is_null() && made.record().get("question").is_none());
    going.kill().unwrap();
    going.wait().unwrap();
    let wish = made.w.on_page(&made.wish);
    assert_eq!((&wish["state"], wish.get("question")), (&json!("needs_you"), None));
    assert_eq!(made.w.kept(&made.wish)["why"], CUT.replace("{run}", &made.run));
    assert!(made.copy_failed() && made.w.starts() == 1);
    made.unanswered(ANSWER, NOT_ASKING);
}
