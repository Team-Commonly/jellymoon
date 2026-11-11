//! A run's lock that cannot be opened. A lock that cannot be asked about is no lock: the run is held by nothing,
//! so it was cut off (`docs/development/run.md`, "A file of the home that is not a plain file").

use std::path::Path;

use serde_json::json;

use super::{grows, printed, world};
use crate::support::skipped;

/// Takes every permission off a file. False where that does not keep this user from opening it.
#[cfg(unix)]
fn closed(file: &Path) -> bool {
    std::fs::set_permissions(file, std::os::unix::fs::PermissionsExt::from_mode(0o000)).unwrap();
    std::fs::File::open(file).is_err()
}

#[cfg(not(unix))]
fn closed(_: &Path) -> bool {
    false
}

#[test]
fn a_cut_off_run_whose_lock_cannot_be_opened_is_written_down_and_its_agent_ended() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (run, file) = w.cut_off(&wish, &repo);
    if !closed(&w.jm("runs").join(&run).join("run.lock")) {
        skipped("no mode keeps this user from opening a file here: the system has none, or the user is root");
        return w.let_end("stop");
    }
    assert_eq!(w.on_page(&wish)["state"], "needs_you", "a lock nobody can ask about was taken for one held");
    assert!(w.freed("held") && !grows(&file), "the agent the run left is still editing");
    let why = format!("run {run} was cut off before it ended: its copy is kept and no star was added");
    let said = json!({"run": run, "wish": wish, "outcome": "failed", "star": null, "denied": 0, "why": why});
    assert_eq!(printed(&w.stop(&wish)), said);
    assert_eq!((&w.kept(&wish)["state"], &w.record(&run)["outcome"]), (&json!("needs_you"), &json!("failed")));
}
