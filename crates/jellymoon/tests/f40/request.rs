//! The request for a stop, left by hand in shapes `run stop` never leaves, and at a moment it never does
//! (`docs/development/run.md`, "The request is a file"). Only its being there is read: nothing is opened, followed
//! or written through it.

#[cfg(unix)]
use std::os::unix::fs::symlink;
#[cfg(windows)]
use std::os::windows::fs::symlink_file as symlink;
use std::time::{Duration, Instant};
use std::{fs, thread};

use super::{ended, printed, said_cancelled, world};
use crate::support::{skipped, tree};

// A folder, a link to a file outside Jellymoon's home, and a link to nothing: each stops the run whose folder it is
// in, as cancelled, and what the link names is as it was.
#[test]
fn a_request_that_is_a_folder_or_a_link_stops_the_run_and_nothing_outside_is_touched() {
    for shape in ["folder", "link", "link to nothing"] {
        let w = world();
        let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
        let outside = w.at("outside");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("theirs.txt"), "the person's own file\n").unwrap();
        let before = tree(&outside);
        let (going, run) = w.run_going(("lingers", 0), &[], &wish, &repo);
        let request = w.jm("runs").join(&run).join("stop");
        let made = match shape {
            "folder" => fs::create_dir(&request).is_ok(),
            "link" => symlink(outside.join("theirs.txt"), &request).is_ok(),
            _ => symlink(outside.join("nothing.txt"), &request).is_ok(),
        };
        if !made {
            fs::write(&request, "").unwrap(); // the run is ended all the same
            assert_eq!(printed(&ended(going))["outcome"], "cancelled");
            skipped(&format!("a {shape} cannot be made here"));
            continue;
        }
        assert_eq!(printed(&ended(going)), said_cancelled(&run, &wish, 0), "{shape}");
        // A stop asked for now prints that end, and leaves the request as it was left.
        assert_eq!(printed(&w.stop(&wish)), said_cancelled(&run, &wish, 0), "{shape}");
        w.is_cancelled(&wish, &run, &repo, 0);
        let kind = fs::symlink_metadata(&request).unwrap().file_type();
        assert_eq!(kind.is_dir(), shape == "folder", "{shape}: the request was changed");
        assert_eq!(tree(&outside), before, "{shape}: a file outside the home was written");
        assert!(w.agent_came_and_went() && w.starts() == 1, "{shape}");
    }
}

// The request is there before the agent is: it is left the moment the run's folder is. The run is cancelled and
// the agent is never started.
#[test]
fn a_request_left_as_soon_as_the_runs_folder_is_there_cancels_the_run_before_any_agent_is_started() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    w.play("lingers", 0);
    let going = w.start_going(&wish, &repo);
    let deadline = Instant::now() + Duration::from_secs(30);
    let run = loop {
        // A run's folder is named as a run is: `runs` holds other folders too.
        let names = fs::read_dir(w.jm("runs")).into_iter().flatten().flatten().map(|entry| entry.file_name());
        let mut runs = names.filter_map(|name| name.into_string().ok()).filter(|name| {
            name.len() == 17 && name.starts_with('r') && name[1..].bytes().all(|b| b.is_ascii_hexdigit())
        });
        if let Some(run) = runs.next() {
            fs::write(w.jm("runs").join(&run).join("stop"), "").unwrap();
            break run;
        }
        assert!(Instant::now() < deadline, "no run's folder was ever made");
        thread::yield_now();
    };
    assert_eq!(printed(&ended(going)), said_cancelled(&run, &wish, 0));
    assert_eq!((w.starts(), w.rec("started")), (0, None), "an agent was started for a run that was to stop");
    // Nor was one started and ended before it could say so: the record names no child.
    assert_eq!(w.record(&run)["pid"], serde_json::Value::Null);
    w.is_cancelled(&wish, &run, &repo, 0);
    // It has had its one run all the same, and a stop asked for now prints the end.
    assert_eq!(printed(&w.stop(&wish)), said_cancelled(&run, &wish, 0));
    assert!(super::refused(&w.start(&wish, &repo)).contains("has had its run"));
    assert_eq!(w.starts(), 0);
}
