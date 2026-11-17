//! The rule that holds tests to the door (ADR 0021, decision 7). The binary's test support is the one place that
//! starts the built `jellymoon` and makes a temporary folder for a home (`crates/jellymoon/tests/support/door.rs`
//! refuses a world outside the temporary folders there). A test file beside it that does either by itself is
//! refused, and so is a test in core that has the harness looked for with the process's own environment. The rule
//! reads lines, not Rust: what it cannot see is said in `docs/development/workflow.md`.

use std::path::Path;

use super::text;

const TESTS: &str = "crates/jellymoon/tests/";
const SUPPORT: &str = "crates/jellymoon/tests/support/";
const CORE: &str = "crates/jellymoon-core/";

/// What only a start of the built program needs to write: the name Cargo gives a test for its path.
const PROGRAM: &str = "CARGO_BIN_EXE_jellymoon";
/// What makes a temporary folder, in the system's or anywhere: `tempfile`'s functions and the standard library's.
const HOME: [&str; 3] = ["tempdir", "TempDir::new", "temp_dir("];
/// The look itself, and a call's options that name no harness, which is how the look is reached from outside.
const LOOK: [&str; 3] = ["harness()", "named_or_found(None", "harness: None"];

/// No test starts the program or builds a home outside the support, and none in core has the harness looked for
/// with the variables of the process that runs the tests.
pub fn outside_the_door(root: &Path, files: &[String]) -> Vec<String> {
    let mut found = Vec::new();
    for file in files.iter().filter(|file| file.ends_with(".rs")) {
        let Some(text) = text(root, file) else {
            continue;
        };
        let beside = file.starts_with(TESTS) && !file.starts_with(SUPPORT);
        let mut in_tests = file.starts_with(CORE) && (file.contains("/tests/") || file.ends_with("/tests.rs"));
        for (number, line) in text.lines().enumerate() {
            let code = line.trim_start();
            in_tests |= file.starts_with(CORE) && code == "#[cfg(test)]";
            let why = if code.starts_with("//") {
                continue;
            } else if beside && code.contains(PROGRAM) {
                "starts the built program outside the test support; start it through `support::door`"
            } else if beside && HOME.iter().any(|made| code.contains(made)) {
                "builds a home outside the test support; take the folder from `support::scratch`"
            } else if in_tests && LOOK.iter().any(|look| code.contains(look)) {
                "has the harness looked for with the process's own environment; name a made-up harness"
            } else {
                continue;
            };
            found.push(format!("{file}:{}: a test {why}", number + 1));
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{PROGRAM, outside_the_door};

    /// What the rule finds in one file of this text, at this path in a repo.
    fn found(file: &str, text: &str) -> Vec<String> {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join(file).parent().unwrap()).unwrap();
        fs::write(root.path().join(file), text).unwrap();
        outside_the_door(root.path(), &[file.to_string()])
    }

    fn starts() -> String {
        format!("use std::fs;\n\nfn run() {{\n    Command::new(env!(\"{PROGRAM}\")).status();\n}}\n")
    }

    #[test]
    fn a_test_file_that_starts_the_program_outside_the_support_is_refused_by_file_and_line() {
        let said = "a test starts the built program outside the test support; start it through `support::door`";
        for file in ["crates/jellymoon/tests/f99.rs", "crates/jellymoon/tests/f99/deep/world.rs"] {
            assert_eq!(found(file, &starts()), [format!("{file}:4: {said}")]);
        }
        // The support is where it is started, and a made-up program of the tests is not the program.
        assert_eq!(found("crates/jellymoon/tests/support/door.rs", &starts()), Vec::<String>::new());
        let other = starts().replace(PROGRAM, "CARGO_BIN_EXE_fake-claude");
        assert_eq!(found("crates/jellymoon/tests/f99.rs", &other), Vec::<String>::new());
        // Only the binary's tests, only Rust, and never a comment.
        for file in ["crates/jellymoon/src/main.rs", "crates/xtask/src/smoke.rs", "crates/jellymoon/tests/f99.md"] {
            assert_eq!(found(file, &starts()), Vec::<String>::new(), "{file}");
        }
        let comment = format!("fn run() {{}}\n    // once it was env!(\"{PROGRAM}\")\n");
        assert_eq!(found("crates/jellymoon/tests/f99.rs", &comment), Vec::<String>::new());
    }

    #[test]
    fn a_test_file_that_builds_a_home_outside_the_support_is_refused_by_file_and_line() {
        let said = "a test builds a home outside the test support; take the folder from `support::scratch`";
        let made = [
            "let dir = tempfile::tempdir().unwrap();",
            "let dir = tempfile::tempdir_in(folder).unwrap();",
            "let dir = tempfile::Builder::new().tempdir_in(folder).unwrap();",
            "let dir = TempDir::new().unwrap();",
            "let home = std::env::temp_dir().join(\"home\");",
        ];
        for line in made {
            let text = format!("fn world() {{\n    {line}\n}}\n");
            assert_eq!(
                found("crates/jellymoon/tests/f99.rs", &text),
                [format!("crates/jellymoon/tests/f99.rs:2: {said}")]
            );
            assert_eq!(found("crates/jellymoon/tests/support/mod.rs", &text), Vec::<String>::new(), "{line}");
            assert_eq!(found("crates/jellymoon-core/tests/shapes.rs", &text), Vec::<String>::new(), "{line}");
        }
        // The type of a folder that was handed over, and a file for a command's input, build no home.
        let handed = "use tempfile::TempDir;\nfn world_in(dir: TempDir) {\n    let fed = tempfile::tempfile();\n}\n";
        assert_eq!(found("crates/jellymoon/tests/f99.rs", handed), Vec::<String>::new());
    }

    #[test]
    fn a_test_in_core_that_has_the_harness_looked_for_with_its_own_environment_is_refused() {
        let said = "a test has the harness looked for with the process's own environment; name a made-up harness";
        let looks =
            ["let found = find::harness();", "named_or_found(None).unwrap();", "Options { harness: None, day }"];
        for look in looks {
            // In a file of tests, in a folder of tests, and after the line that opens a file's own tests.
            let (alone, inside) = (format!("fn t() {{\n    {look}\n}}\n"), format!("mod tests {{\n    {look}\n}}\n"));
            for file in ["crates/jellymoon-core/tests/look.rs", "crates/jellymoon-core/src/find/tests.rs"] {
                assert_eq!(found(file, &alone), [format!("{file}:2: {said}")]);
            }
            let file = "crates/jellymoon-core/src/find.rs";
            let both = format!("{alone}    #[cfg(test)]\n{inside}");
            assert_eq!(found(file, &both), [format!("{file}:6: {said}")], "what ships is before that line");
            assert_eq!(found(file, &alone), Vec::<String>::new(), "{look}: no test at all");
            // Core alone: the binary and the app name a harness in their own ways.
            assert_eq!(found("crates/jellymoon/src/tests.rs", &alone), Vec::<String>::new());
            assert_eq!(found("crates/jellymoon/src/main.rs", &both), Vec::<String>::new());
        }
        let named = "fn t() {\n    let options = Options { harness: Some(made_up), day };\n    // find::harness()\n}\n";
        assert_eq!(found("crates/jellymoon-core/tests/look.rs", named), Vec::<String>::new());
    }

    #[test]
    fn check_runs_the_rule() {
        let root = tempfile::tempdir().unwrap();
        let file = "crates/jellymoon/tests/f99.rs";
        fs::create_dir_all(root.path().join("crates/jellymoon/tests")).unwrap();
        fs::write(root.path().join(file), starts()).unwrap();
        fs::write(root.path().join("entropy.toml"), "max_file_lines = 400\n").unwrap();
        let found = super::super::check(root.path(), &[file.to_string()]).unwrap();
        assert!(
            found.len() == 1 && found[0].starts_with("crates/jellymoon/tests/f99.rs:4: a test starts"),
            "{found:?}"
        );
    }
}
