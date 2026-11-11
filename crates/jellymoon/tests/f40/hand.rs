//! Files of the home changed by hand. A run's record and a day's wishes are files that whoever can write the home
//! can write, so a stop believes of them only what core itself would have written
//! (`docs/development/run.md`, "The end is written in the record first", "A stop the person asks for").

use serde_json::{Value, json};

use super::{STAR, World, cancelled, grows, ms, printed, ran, refused, said_cancelled, world};
use crate::support::tree;

fn cut(run: &str) -> String {
    format!("run {run} was cut off before it ended: its copy is kept and no star was added")
}

/// The agent a cut-off run left is gone, where there is a signal to end it with.
fn the_agent_is_gone(w: &World, file: &std::path::Path) {
    if cfg!(unix) {
        assert!(w.freed("held") && !grows(file), "the agent a cut-off run left is still editing its copy");
    } else {
        w.let_end("stop"); // there is no signal there, and nothing is stopped
    }
}

// The record of a run that was cut off, changed by hand to say the run was cancelled. In core's own words it is
// believed; in any others it is not, and the run was cut off. Either way nothing holds the run, so the agent it
// left is ended.
#[test]
fn a_cut_off_run_whose_record_was_made_to_say_cancelled_has_its_agent_ended_whatever_the_words() {
    for (words, outcome) in [("", "believed"), ("run r1 was cancelled, says the person", "not believed")] {
        let w = world();
        let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
        let (run, file) = w.cut_off(&wish, &repo);
        let why = if words.is_empty() { cancelled(&run) } else { words.to_string() };
        w.edit_record(&run, |record| (record["outcome"], record["why"]) = (json!("cancelled"), json!(why)));

        let said = printed(&w.stop(&wish));
        let cut_off =
            json!({"run": run, "wish": wish, "outcome": "failed", "star": null, "denied": 0, "why": cut(&run)});
        assert_eq!(said, if words.is_empty() { said_cancelled(&run, &wish, 0) } else { cut_off }, "{outcome}");
        assert_eq!((&w.kept(&wish)["state"], &w.kept(&wish)["why"]), (&json!("needs_you"), &said["why"]), "{outcome}");
        the_agent_is_gone(&w, &file);
        // A record that has an end is never written again, believed or not.
        assert_eq!((&w.record(&run)["outcome"], &w.record(&run)["why"]), (&json!("cancelled"), &json!(why)));
    }
}

// A stop asked for after the run ended prints the run's end. What it prints of the record, the end and the count
// of what was denied, it prints only of a record whose end is core's and is the end the wish has.
#[test]
fn a_stop_prints_nothing_of_a_record_whose_end_it_does_not_believe() {
    let w = world();
    let repo = w.repo("alpha");
    let not_a_success = "did not finish (its result was not a success): its copy is kept and no star was added";
    // A run that failed, with three things denied: the record is given other words, and another count.
    let failed = w.wish("alpha");
    w.play("denied-failed", 3);
    let (run, first) = ran(&w.start(&failed, &repo));
    assert_eq!((&first["outcome"], &first["denied"]), (&json!("failed"), &json!(3)));
    w.edit_record(&run, |record| (record["why"], record["denied"]) = (json!("it failed, says the person"), json!(77)));
    let why = format!("run {run} {not_a_success}");
    let said = json!({"run": run, "wish": failed, "outcome": "failed", "star": null, "denied": 0, "why": why});
    assert_eq!(printed(&w.stop(&failed)), said);

    // A run that finished, with two things denied: the record is made to say it failed, in core's own words for
    // that. The wish is done and has its star, and the record does not say so: it is not what is printed.
    let done = w.wish("alpha");
    w.play("denials", 2);
    let (run, first) = ran(&w.start(&done, &repo));
    assert_eq!((&first["outcome"], &first["star"], &first["denied"]), (&json!("completed"), &json!(STAR), &json!(2)));
    let why = format!("run {run} {not_a_success}");
    w.edit_record(&run, |record| {
        (record["outcome"], record["why"], record["denied"]) = (json!("failed"), json!(why), json!(77));
    });
    let said =
        json!({"run": run, "wish": done, "outcome": "completed", "star": STAR, "denied": 0, "why": first["why"]});
    assert_eq!(printed(&w.stop(&done)), said);
    assert_eq!((&w.kept(&done)["state"], &w.kept(&done)["stars"]), (&json!("done"), &json!([STAR])));
}

// The day's wishes changed by hand, so that a second wish names the first wish's run. A run is stopped, and is
// written down, on the word of its own wish only: nothing asked for the second wish reaches it.
#[test]
fn a_wish_made_to_name_another_wishs_run_stops_nothing_and_writes_nothing() {
    let w = world();
    let repo = w.repo("alpha");
    let (one, two) = (w.wish("alpha"), w.wish("alpha"));
    let (mut going, run) = w.run_going(("scribble", 0), &[], &one, &repo);
    let file = w.jm("copies").join(&run).join("scribble.txt");
    w.edit_wish(&two, |wish| (wish["state"], wish["run"]) = (json!("working"), json!(run)));
    let kept = || (tree(&w.jm("runs")), tree(&w.jm("pages")));
    let before = kept();
    let not_its_own =
        format!("jellymoon: wish {two} names run {run}, which is another wish's run, so nothing was stopped\n");
    let stop = || w.stop_with(&[ms("JELLYMOON_TEST_STOP_WAIT_MS", 2000)], &two);

    // While the run goes: no request is left, and the run goes on.
    assert_eq!(refused(&stop()), not_its_own);
    assert_eq!(kept(), before);
    assert!(going.try_wait().unwrap().is_none() && grows(&file), "the first wish's run was stopped");

    // Once its `jellymoon run` was cut off: it is not written down for the second wish, and its agent is left.
    going.kill().unwrap();
    going.wait().unwrap();
    let before = kept();
    assert_eq!(refused(&stop()), not_its_own);
    assert!(refused(&w.start(&two, &repo)).contains("has had its run"));
    assert_eq!(kept(), before);
    assert!(grows(&file), "the agent of the first wish's run was ended on the second wish's word");

    // A page writes it down on the first wish's word, as cut off, and ends the agent. The second wish is drawn as
    // it was written by hand.
    assert_eq!((&w.on_page(&one)["state"], &w.on_page(&two)["state"]), (&json!("needs_you"), &json!("working")));
    the_agent_is_gone(&w, &file);
    assert_eq!((&w.kept(&one)["why"], &w.record(&run)["wish"]), (&json!(cut(&run)), &json!(one)));
    assert_eq!((w.kept(&two)["run"].clone(), w.starts()), (json!(run), 1));
}

impl World {
    /// Changes a made wish by hand.
    pub fn edit_wish(&self, wish: &str, change: impl FnOnce(&mut Value)) {
        let file = self.jm("pages").join(format!("{}.wishes.json", super::DAY));
        let mut kept: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        change(kept["wishes"].as_array_mut().unwrap().iter_mut().find(|kept| kept["id"] == wish).unwrap());
        std::fs::write(file, format!("{kept}\n")).unwrap();
    }

    /// Changes a run's record by hand.
    pub fn edit_record(&self, run: &str, change: impl FnOnce(&mut Value)) {
        let mut record = self.record(run);
        change(&mut record);
        std::fs::write(self.jm("runs").join(run).join("run.json"), format!("{record}\n")).unwrap();
    }
}
