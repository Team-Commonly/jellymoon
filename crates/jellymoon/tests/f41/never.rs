//! A harness that never begins a run (`docs/development/run.md`, "A run that was never started"): whoever asked is
//! told why, never that a run is going, and the wish keeps the same line and needs the person.

use std::ffi::OsString;
use std::fs;

use serde_json::{Value, json};

use super::{DAY, OLD, SAID, STAR, World, found_world, printed};

const RUN: [&str; 8] = ["wish", "--date", DAY, "--say", SAID, "--project", "alpha", "--run"];
const UNSTARTABLE: &str = "nothing was started: Claude Code could not be started";

/// The wish, the asker's line and the page all say the same of a run that was never started, and nothing is left.
fn never_started(w: &World, said: &Value, why: &str, starts: usize) {
    let wish = json!({"id": "m1", "said": SAID, "project": "alpha", "state": "needs_you", "stars": []});
    assert_eq!(*said, json!({"wish": wish, "run": null, "why": why}));
    let kept = json!({"id": "m1", "said": SAID, "project": "alpha", "state": "needs_you", "stars": [], "why": why});
    assert_eq!((w.kept("m1"), &w.on_page("m1")["state"]), (kept, &json!("needs_you")));
    let runs = fs::read_dir(w.jm("runs")).unwrap().map(|entry| entry.unwrap().file_name().into_string().unwrap());
    let runs: Vec<String> = runs.filter(|name| name.starts_with('r')).collect();
    assert_eq!((w.copies(), runs, w.starts()), (vec![], vec![], starts));
}

/// The wish is run again with a harness that works, and is done.
fn runs_again(w: &World, more: &[(&'static str, OsString)]) {
    w.play("edits", 0);
    let again = printed(&w.run_with(more, &["run", "--wish", "m1", "--date", DAY]));
    assert_eq!((&again["outcome"], &again["star"]), (&json!("completed"), &json!(STAR)));
    let kept = w.kept("m1");
    assert_eq!((&kept["state"], &kept["stars"], &kept["run"]), (&json!("done"), &json!([STAR]), &again["run"]));
}

#[test]
fn a_harness_too_old_for_runs_is_told_to_the_asker_and_kept_with_the_wish_which_needs_the_person() {
    let w = found_world();
    let repo = w.repo("alpha");
    w.typed_in("-work-alpha", &repo);
    w.play("old", 0);
    never_started(&w, &printed(&w.run(&RUN)), OLD, 1);
    assert!(w.branches(&repo).is_empty(), "the branch of a run that was never started was left");
    runs_again(&w, &[]);
}

#[test]
fn a_harness_that_cannot_be_started_is_told_to_the_asker_and_kept_with_the_wish_which_needs_the_person() {
    let w = found_world();
    let repo = w.repo("alpha");
    w.typed_in("-work-alpha", &repo);
    // A file that is there and that the system refuses to start: no program on Windows, and elsewhere a script
    // whose interpreter is not there (a file with no such line would be handed to the shell).
    let broken = w.at("bin").join(format!("broken{}", std::env::consts::EXE_SUFFIX));
    fs::write(&broken, "#!/no-such-folder/no-such-interpreter\n").unwrap();
    #[cfg(unix)]
    fs::set_permissions(&broken, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    let named = [("JELLYMOON_CLAUDE", OsString::from(&broken))];
    never_started(&w, &printed(&w.run_with(&named, &RUN)), UNSTARTABLE, 0);
    assert!(w.branches(&repo).is_empty(), "the branch of a run that was never started was left");
    // Asked again the same way, it says the same: the line the wish had is not taken for an answer of its own.
    let again = super::refused(&w.run_with(&named, &["run", "--wish", "m1", "--date", DAY])).to_string();
    assert!(again.starts_with(&format!("jellymoon: {UNSTARTABLE}: ")), "{again}");
    assert_eq!((&w.kept("m1")["state"], &w.kept("m1")["why"]), (&json!("needs_you"), &json!(UNSTARTABLE)));
    runs_again(&w, &[]);
}
