//! Files of the home changed by hand. A run's record and a day's wishes are files that whoever can write the home
//! can write, so a stop believes of them only what core itself would have written
//! (`docs/development/run.md`, "The end is written in the record first", "A stop the person asks for").

use serde_json::{Value, json};

use super::{World, cancelled, grows, printed, said_cancelled, world};

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

impl World {
    /// Changes a run's record by hand.
    pub fn edit_record(&self, run: &str, change: impl FnOnce(&mut Value)) {
        let mut record = self.record(run);
        change(&mut record);
        std::fs::write(self.jm("runs").join(run).join("run.json"), format!("{record}\n")).unwrap();
    }
}
