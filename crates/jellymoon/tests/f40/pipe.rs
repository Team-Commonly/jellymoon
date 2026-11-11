//! A named pipe where a file of the home is expected. Opening one waits for whoever is at its other end, and there
//! is nobody: a look or a stop that opened it before asking what kind of file it is would wait for ever
//! (`docs/development/run.md`, "A file of the home that is not a plain file"). Every command here runs under a
//! time limit of its own. A link where a file is written is the same question: nothing is written through it.

use std::fs;
#[cfg(unix)]
use std::os::unix::fs::symlink;
#[cfg(windows)]
use std::os::windows::fs::symlink_file as symlink;
use std::path::Path;
use std::process::Command;

use serde_json::json;

use super::{DAY, World, grows, printed, within, world};
use crate::support::{Ran, skipped, tree};

/// Every file of the home that a page or a stop opens for a run that was cut off, and the name beside the copy's
/// record that it was once written through. `RUN` stands for the run's id.
const PLACES: [&str; 13] = [
    "runs/RUN/run.lock",
    "runs/RUN/run.json",
    "runs/RUN/copy.json",
    "runs/RUN/copy.new",
    "runs/RUN/stop",
    "runs/clean.lock",
    "pages/2026-03-10.wishes.json",
    "pages/2026-03-10.ruled.lock",
    "pages/2026-03-10.json",
    "pages/2026-03-10.ruled.json",
    "rulings/1773140000000.json",
    "hook.json",
    "skip.txt",
];

/// Puts a named pipe at the path, in place of what is there. False, with the reason said, where none can be made.
fn pipe(path: &Path) -> bool {
    let Some(mkfifo) = ["/usr/bin/mkfifo", "/bin/mkfifo"].iter().find(|at| cfg!(unix) && Path::new(at).is_file())
    else {
        skipped("no named pipe can be made here: this system has none, or mkfifo is in neither /usr/bin nor /bin");
        return false;
    };
    let _ = fs::remove_file(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    assert!(Command::new(mkfifo).arg(path).status().unwrap().success());
    true
}

/// `run stop`, or `page`, given ten seconds: what it printed, or None if it was still waiting then.
fn look(w: &World, command: &str, wish: &str) -> Option<Ran> {
    let logs = w.at("logs");
    let page = ["page", "--date", DAY, "--logs", logs.to_str().unwrap()];
    let stop = ["run", "stop", "--wish", wish, "--date", DAY];
    within(w.going(&[], if command == "page" { &page } else { &stop }), 10)
}

#[test]
fn a_run_whose_lock_is_a_named_pipe_is_held_by_nothing_and_is_written_down_as_cut_off() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (run, file) = w.cut_off(&wish, &repo);
    if !pipe(&w.jm("runs").join(&run).join("run.lock")) {
        return w.let_end("stop");
    }
    let stop = look(&w, "run stop", &wish).expect("run stop waited on a named pipe for longer than ten seconds");
    let why = format!("run {run} was cut off before it ended: its copy is kept and no star was added");
    let said = json!({"run": run, "wish": wish, "outcome": "failed", "star": null, "denied": 0, "why": why});
    assert_eq!(printed(&stop), said);
    let page = printed(&look(&w, "page", &wish).expect("page waited on a named pipe for longer than ten seconds"));
    assert_eq!(page["wishes"][0]["state"], "needs_you");
    assert!(w.freed("held") && !grows(&file), "the agent the run left is still editing");
    assert_eq!((&w.kept(&wish)["state"], &w.record(&run)["outcome"]), (&json!("needs_you"), &json!("failed")));
}

#[test]
fn a_named_pipe_at_any_file_of_the_home_keeps_neither_a_stop_nor_a_page_waiting() {
    let mut waited = Vec::new();
    // Each look finds the run as it was cut off, in a folder of its own: the first to look writes the run down.
    for (place, command) in PLACES.iter().flat_map(|place| [(place, "run stop"), (place, "page")]) {
        let w = world();
        let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
        let (run, _) = w.cut_off(&wish, &repo);
        if !pipe(&w.at("jm").join(place.replace("RUN", &run))) {
            return w.let_end("stop");
        }
        match look(&w, command, &wish) {
            // It says one thing on one of its outputs, whatever that is, and ends.
            Some(ran) => assert_eq!(ran.stdout.lines().count() + ran.stderr.lines().count(), 1, "{place}"),
            None => waited.push(format!("{command} at {place}")),
        }
        // The agent a cut-off run left, where no look could tell which it is, is ended here.
        fs::write(w.at("rec").join("stop"), "").unwrap();
        assert!(w.freed("held"), "{place}: the agent was never ended");
    }
    assert_eq!(waited, Vec::<String>::new(), "these waited on a named pipe for longer than ten seconds");
}

// A look marks the copy of a cut-off run failed, which writes the copy's record. A link beside the record, at the
// name it was once written through, leads to a file outside the home: that file is as it was.
#[test]
fn a_link_beside_the_copys_record_to_a_file_outside_the_home_leaves_that_file_as_it_was() {
    for command in ["run stop", "page"] {
        let w = world();
        let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
        let (run, _) = w.cut_off(&wish, &repo);
        let outside = w.at("outside");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("theirs.txt"), "the person's own file\n").unwrap();
        let before = tree(&outside);
        let made = symlink(outside.join("theirs.txt"), w.jm("runs").join(&run).join("copy.new")).is_ok();
        if made {
            let ran = look(&w, command, &wish).expect("it waited for longer than ten seconds");
            assert_eq!(ran.stdout.lines().count() + ran.stderr.lines().count(), 1, "{command}");
            assert_eq!(tree(&outside), before, "{command}: a file outside the home was written");
            let copy = fs::read_to_string(w.jm("runs").join(&run).join("copy.json")).unwrap();
            let copy: serde_json::Value = serde_json::from_str(&copy).unwrap();
            assert!(copy["failed"].is_u64(), "{command}: the copy is not marked failed: {copy}");
        } else {
            skipped("a link cannot be made here");
        }
        fs::write(w.at("rec").join("stop"), "").unwrap();
        assert!(w.freed("held"), "{command}: the agent was never ended");
    }
}
