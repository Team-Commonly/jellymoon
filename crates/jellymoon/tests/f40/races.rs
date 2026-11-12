//! A stop against the run's own finish. The made-up agent waits for the test to leave `go`, then does its work and
//! reports a success. The stop is asked just before, just after or at that moment. Either the run finished, with
//! its star, or it was cancelled, with none: never both, never neither, and the stop and the run say the same.

use std::time::Duration;
use std::{fs, thread};

use serde_json::json;

use super::{STAR, ended, printed, world};

#[test]
fn a_stop_racing_the_runs_own_finish_gives_one_end_which_both_commands_print() {
    let w = world();
    let repo = w.repo("alpha");
    let base = w.commit(&repo, "HEAD");
    let mut ends = Vec::new();
    for round in 0..8_u64 {
        let wish = w.wish("alpha");
        for left in ["started", "go"] {
            let _ = fs::remove_file(w.at("rec").join(left));
        }
        let (going, run) = w.run_going(("slow", 0), &[], &wish, &repo);
        // The agent is let go first, by more each round, or the stop is asked first, by more each round.
        let (first, wait) = (round % 2 == 0, Duration::from_millis(round / 2 * 15));
        let stopped = thread::scope(|both| {
            let stop = both.spawn(|| {
                thread::sleep(if first { wait } else { Duration::ZERO });
                w.stop(&wish)
            });
            thread::sleep(if first { Duration::ZERO } else { wait });
            fs::write(w.at("rec").join("go"), "").unwrap();
            stop.join().unwrap()
        });
        let (stopped, said) = (printed(&stopped), printed(&ended(going)));
        assert_eq!(stopped, said, "round {round}: the stop and the run say the same");
        assert!(w.agent_came_and_went(), "round {round}");
        let (kept, record) = (w.kept(&wish), w.record(&run));
        let (branch, copy) = (w.commit(&repo, &format!("jellymoon/run-{run}")), w.jm("copies").join(&run));
        let outcome = said["outcome"].as_str().unwrap().to_string();
        assert_eq!((&record["outcome"], &record["why"], &kept["why"]), (&said["outcome"], &said["why"], &said["why"]));
        let (star, state, stars) = (&said["star"], &kept["state"], &kept["stars"]);
        match outcome.as_str() {
            "completed" => {
                assert_eq!((star, state, stars), (&json!(STAR), &json!("done"), &json!([STAR])), "round {round}");
                assert!(!copy.exists() && branch != base, "round {round}: a finished run's work is on its branch");
            }
            "cancelled" => {
                assert_eq!((star, state, stars), (&json!(null), &json!("needs_you"), &json!([])), "round {round}");
                assert!(copy.is_dir() && branch == base, "round {round}: a stopped run keeps its copy");
            }
            other => panic!("round {round}: {other} is neither end: {said}"),
        }
        assert_eq!(w.on_page(&wish)["state"], kept["state"], "round {round}");
        ends.push(outcome);
    }
    assert_eq!(w.starts(), 8);
    eprintln!("a stop against the finish, eight rounds: {}", ends.join(", "));
}
