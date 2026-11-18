//! The rule that holds tests to the door (ADR 0021, decision 7). Each program's tests have one file that names the
//! built program, `support/door.rs`, which refuses a world outside the temporary folders, and one folder that makes
//! a temporary folder for a home, `support/`. A test file that does either anywhere else is refused, and so is a
//! test of core or of the app that has the harness looked for with the process's own environment. The rule reads
//! lines, not Rust: what it cannot see is said in `docs/development/workflow.md`.

use std::path::Path;

use super::text;

/// One program's tests: their folder, the programs only `support/door.rs` in it may name (what Cargo calls the
/// path of each, which only a start or a copy needs to write), and the crate whose own tests reach the look.
struct Door {
    tests: &'static str,
    programs: &'static [&'static str],
    look: &'static str,
}

/// The binary's tests, with core's; and the app's, which build `jellymoon` and two made-up windows that are the
/// app's own code under names of their own.
const DOORS: [Door; 2] = [
    Door { tests: "crates/jellymoon/tests/", programs: &["CARGO_BIN_EXE_jellymoon"], look: "crates/jellymoon-core/" },
    Door {
        tests: "app/tests/",
        programs: &[
            "CARGO_BIN_EXE_jellymoon",
            "CARGO_BIN_EXE_f42-jellymoon",
            "CARGO_BIN_EXE_f42-wisher",
            "CARGO_BIN_EXE_f47-tapper",
        ],
        look: "app/",
    },
];

/// What makes a temporary folder, in the system's or anywhere: `tempfile`'s functions and the standard library's.
const HOME: [&str; 3] = ["tempdir", "TempDir::new", "temp_dir("];
/// The look's two names in core, each a call with nothing handed to it.
const LOOK: [&str; 2] = ["harness()", "search()"];
/// The ways into the look that are handed no harness, and a call's options that name none, which is how the look is
/// reached from outside.
const UNNAMED: [&str; 3] = ["named_or_found(None", "harness_found(None", "harness: None"];

/// Whether a line has the harness looked for: the look called by a name of its own (`find::harness()`, `search()`
/// alone, but no method or longer name that ends so, and not where a function of that name is written), or a way
/// in or options that name no harness.
fn looks(code: &str) -> bool {
    let named = |before: &str| before.ends_with(|last: char| last.is_alphanumeric() || last == '_' || last == '.');
    let called = |(at, _): (usize, &str)| !named(&code[..at]) && !code[..at].ends_with("fn ");
    LOOK.iter().any(|look| code.match_indices(look).any(called)) || UNNAMED.iter().any(|look| code.contains(look))
}

/// No test names a built program outside the door or builds a home outside the support, and none in core or in
/// the app has the harness looked for with the variables of the process that runs the tests.
pub fn outside_the_door(root: &Path, files: &[String]) -> Vec<String> {
    let mut found = Vec::new();
    for file in files.iter().filter(|file| file.ends_with(".rs")) {
        let Some(door) = DOORS.iter().find(|door| file.starts_with(door.tests) || file.starts_with(door.look)) else {
            continue;
        };
        let Some(text) = text(root, file) else {
            continue;
        };
        let (beside, support) = (file.starts_with(door.tests), format!("{}support/", door.tests));
        let is_door = *file == format!("{support}door.rs");
        // A file of tests is tests throughout. Any other file's tests are what `mod tests` holds, where that
        // stands under `#[cfg(test)]`: the attribute above anything else opens no tests.
        let whole = file.starts_with(door.look) && (file.contains("/tests/") || file.ends_with("/tests.rs"));
        let (mut armed, mut inside) = (false, false);
        for (number, line) in text.lines().enumerate() {
            let code = line.trim_start();
            inside |= armed && code.starts_with("mod tests") && file.starts_with(door.look);
            let attribute = code.starts_with("#[cfg(test)]") || code.starts_with("#[cfg(all(test");
            armed = attribute || armed && code.starts_with("#[");
            let why = if code.starts_with("//") {
                continue;
            } else if beside && !is_door && door.programs.iter().any(|program| code.contains(program)) {
                "names a built program outside `support/door.rs`; start it through `support::door`"
            } else if beside && !file.starts_with(&support) && HOME.iter().any(|made| code.contains(made)) {
                "builds a home outside the test support; take the folder from `support::scratch`"
            } else if (whole || inside) && looks(code) {
                "has the harness looked for with the process's own environment; name a made-up harness"
            } else {
                inside &= !line.starts_with('}');
                continue;
            };
            found.push(format!("{file}:{}: a test {why}", number + 1));
        }
    }
    found
}

#[cfg(test)]
#[path = "door_tests.rs"]
mod tests;
