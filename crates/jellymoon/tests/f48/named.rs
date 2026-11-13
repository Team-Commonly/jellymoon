//! A record's start and end against the record's own words (`docs/development/show.md`, "Which run, and which
//! repo"): a run that made its one commit ended as another commit than it started from, and a run that changed
//! nothing ended as its start. A record that says otherwise is not one a run wrote, and it is refused before git
//! is asked anything and before anything of the run is printed.

use serde_json::json;

use super::shown::{NOT_CORES, names, no, with};
use super::{finished, played, printed};

#[test]
fn a_run_that_made_a_commit_and_names_one_commit_as_its_start_and_its_end_is_refused() {
    let made = finished();
    let kept = made.record();
    assert!(kept["why"].as_str().unwrap().contains("its work is one commit"), "{kept}");
    for name in ["start", "end"] {
        let other = if name == "start" { "end" } else { "start" };
        let same = with(&kept, name, kept[other].clone());
        // Whatever the state of its work: nothing of such a record is printed.
        for record in [same.clone(), with(&same, "work", json!("kept")), with(&same, "work", json!("tossed"))] {
            made.write(&record);
            no(&made, &made.repo, NOT_CORES);
        }
    }
    made.write(&kept);
    assert_eq!(names(&printed(&made.show())), ["a.txt", "b.txt", "n.txt"]);
}

#[test]
fn a_run_that_changed_nothing_and_names_two_commits_is_refused() {
    let made = played("nothing", "completed");
    let kept = made.record();
    assert!(kept["why"].as_str().unwrap().contains("changed nothing"), "{kept}");
    assert_eq!(kept["start"], kept["end"]);
    // A commit that is in the repo, one parent away from the start, and a full id that names nothing there.
    let by_hand = made.by_hand(kept["start"].as_str().unwrap(), &made.message(), "main");
    for other in [by_hand, "0123456789abcdef0123456789abcdef01234567".to_string()] {
        for name in ["start", "end"] {
            made.write(&with(&kept, name, json!(other)));
            no(&made, &made.repo, NOT_CORES);
        }
    }
    made.write(&kept);
    let shown = printed(&made.show());
    assert_eq!((&shown["work"], &shown["files"]), (&json!("waits"), &json!([])));
}
