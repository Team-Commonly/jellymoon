//! The door rule's own tests: each clause for the binary's tests and core, and for the app's (F74, F75).

use std::fs;

use super::outside_the_door;

const PROGRAM: &str = "CARGO_BIN_EXE_jellymoon";
const NAMES: &str = "a test names a built program outside `support/door.rs`; start it through `support::door`";
const BUILDS: &str = "a test builds a home outside the test support; take the folder from `support::scratch`";
const LOOKS: &str = "a test has the harness looked for with the process's own environment; name a made-up harness";
const NOTHING: Vec<String> = Vec::new();

/// What the rule finds in one file of this text, at this path in a repo.
fn found(file: &str, text: &str) -> Vec<String> {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join(file).parent().unwrap()).unwrap();
    fs::write(root.path().join(file), text).unwrap();
    outside_the_door(root.path(), &[file.to_string()])
}

/// A file whose fourth line starts the program Cargo calls `program`.
fn starts(program: &str) -> String {
    format!("use std::fs;\n\nfn run() {{\n    Command::new(env!(\"{program}\")).status();\n}}\n")
}

#[test]
fn a_test_file_that_starts_the_program_outside_the_door_is_refused_by_file_and_line() {
    for file in ["crates/jellymoon/tests/f99.rs", "crates/jellymoon/tests/f99/deep/world.rs"] {
        assert_eq!(found(file, &starts(PROGRAM)), [format!("{file}:4: {NAMES}")]);
    }
    // The door is where it is started, and nowhere else in the support (the review of F74, finding 10).
    assert_eq!(found("crates/jellymoon/tests/support/door.rs", &starts(PROGRAM)), NOTHING);
    for file in ["crates/jellymoon/tests/support/mod.rs", "crates/jellymoon/tests/support/door/world.rs"] {
        assert_eq!(found(file, &starts(PROGRAM)), [format!("{file}:4: {NAMES}")]);
    }
    // A made-up program of the tests is not the program.
    assert_eq!(found("crates/jellymoon/tests/f99.rs", &starts("CARGO_BIN_EXE_fake-claude")), NOTHING);
    // Only a program's tests, only Rust, and never a comment.
    for file in ["crates/jellymoon/src/main.rs", "crates/xtask/src/smoke.rs", "crates/jellymoon/tests/f99.md"] {
        assert_eq!(found(file, &starts(PROGRAM)), NOTHING, "{file}");
    }
    let comment = format!("fn run() {{}}\n    // once it was env!(\"{PROGRAM}\")\n");
    assert_eq!(found("crates/jellymoon/tests/f99.rs", &comment), NOTHING);
}

#[test]
fn a_test_file_of_the_app_that_starts_a_program_of_its_door_outside_the_door_is_refused() {
    let programs = ["jellymoon-app", "f42-jellymoon", "f42-wisher", "f47-tapper"];
    for program in programs.map(|program| format!("CARGO_BIN_EXE_{program}")) {
        for file in ["app/tests/f99.rs", "app/tests/f99/deep.rs", "app/tests/support/wished.rs"] {
            assert_eq!(found(file, &starts(&program)), [format!("{file}:4: {NAMES}")], "{program}");
        }
        assert_eq!(found("app/tests/support/door.rs", &starts(&program)), NOTHING, "{program}");
        assert_eq!(found("app/src/main.rs", &starts(&program)), NOTHING, "{program}");
    }
    // The made-up harness and the made-up agent are no program of the door, and the windows are none of the
    // binary's.
    for made_up in ["CARGO_BIN_EXE_f34-claude", "CARGO_BIN_EXE_f42-agent"] {
        assert_eq!(found("app/tests/f99.rs", &starts(made_up)), NOTHING, "{made_up}");
    }
    assert_eq!(found("crates/jellymoon/tests/f99.rs", &starts("CARGO_BIN_EXE_f42-wisher")), NOTHING);
}

#[test]
fn a_test_file_that_builds_a_home_outside_the_support_is_refused_by_file_and_line() {
    let made = [
        "let dir = tempfile::tempdir().unwrap();",
        "let dir = tempfile::tempdir_in(folder).unwrap();",
        "let dir = tempfile::Builder::new().tempdir_in(folder).unwrap();",
        "let dir = TempDir::new().unwrap();",
        "let home = std::env::temp_dir().join(\"home\");",
    ];
    for line in made {
        let text = format!("fn world() {{\n    {line}\n}}\n");
        for file in ["crates/jellymoon/tests/f99.rs", "app/tests/f99.rs", "app/tests/f99/window.rs"] {
            assert_eq!(found(file, &text), [format!("{file}:2: {BUILDS}")]);
        }
        let allowed = [
            "crates/jellymoon/tests/support/mod.rs",
            "app/tests/support/mod.rs",
            "app/tests/support/wished.rs",
            "crates/jellymoon-core/tests/shapes.rs",
            "app/src/window.rs",
            "app/examples/snapshot.rs",
        ];
        for file in allowed {
            assert_eq!(found(file, &text), NOTHING, "{file}: {line}");
        }
    }
    // The type of a folder that was handed over, and a file for a command's input, build no home.
    let handed = "use tempfile::TempDir;\nfn world_in(dir: TempDir) {\n    let fed = tempfile::tempfile();\n}\n";
    for file in ["crates/jellymoon/tests/f99.rs", "app/tests/f99.rs"] {
        assert_eq!(found(file, handed), NOTHING);
    }
}

#[test]
fn a_test_in_core_or_the_app_that_has_the_harness_looked_for_with_its_own_environment_is_refused() {
    let looks = [
        "let found = find::harness();",
        "None => harness(),",
        "named_or_found(None).unwrap();",
        "Options { harness: None, day }",
    ];
    for look in looks {
        // In a file of tests, in a folder of tests, and in the tests a file holds itself.
        let (alone, inside) = (format!("fn t() {{\n    {look}\n}}\n"), format!("mod tests {{\n    {look}\n}}\n"));
        let files = ["crates/jellymoon-core/tests/look.rs", "crates/jellymoon-core/src/find/tests.rs"];
        for file in
            files.into_iter().chain(["app/tests/f99.rs", "app/tests/support/wished.rs", "app/tests/f99/gone.rs"])
        {
            assert_eq!(found(file, &alone), [format!("{file}:2: {LOOKS}")]);
        }
        for file in ["crates/jellymoon-core/src/find.rs", "app/src/today.rs"] {
            let both = format!("{alone}    #[cfg(test)]\n{inside}");
            assert_eq!(found(file, &both), [format!("{file}:6: {LOOKS}")], "what ships is before that line");
            assert_eq!(found(file, &alone), NOTHING, "{look}: no test at all");
        }
        // The binary names a harness in its own way, and xtask has no look.
        assert_eq!(found("crates/jellymoon/src/tests.rs", &alone), NOTHING);
        assert_eq!(found("crates/jellymoon/tests/f99.rs", &alone), NOTHING);
        assert_eq!(found("crates/xtask/src/tests.rs", &alone), NOTHING);
    }
    let named = "fn t() {\n    let options = Options { harness: Some(made_up), day };\n    // find::harness()\n}\n";
    assert_eq!(found("crates/jellymoon-core/tests/look.rs", named), NOTHING);
    assert_eq!(found("app/tests/f99.rs", named), NOTHING);
}

// The review of F74, finding 9: two shapes of rightful code were refused.
#[test]
fn only_the_look_s_own_name_is_the_look_and_only_what_mod_tests_holds_is_a_file_s_tests() {
    // A method, a longer name and a function of the name are not the look.
    for rightful in ["let h = w.harness();", "let h = made_up_harness();", "fn harness() -> PathBuf {"] {
        let text = format!("fn t() {{\n    {rightful}\n}}\n");
        assert_eq!(found("crates/jellymoon-core/tests/look.rs", &text), NOTHING, "{rightful}");
    }
    let file = "crates/jellymoon-core/src/find.rs";
    // The attribute above anything but `mod tests` opens no tests: what ships after it may call the look.
    let seam = "#[cfg(test)]\nfn seam() {}\n\nfn found() {\n    None => harness(),\n}\n";
    assert_eq!(found(file, seam), NOTHING);
    let item = "#[cfg(test)]\nimpl Touched {}\nmod tests {\n    harness();\n}\n";
    assert_eq!(found(file, item), NOTHING, "`mod tests` not under the attribute");
    // Tests end where `mod tests` ends, and are opened under another attribute, and by a test for some systems.
    let after = "#[cfg(test)]\nmod tests {\n    harness();\n}\n\nfn ships() {\n    harness();\n}\n";
    assert_eq!(found(file, after), [format!("{file}:3: {LOOKS}")]);
    let pathed = "#[cfg(all(test, unix))]\n#[path = \"x.rs\"]\nmod tests {\n    harness();\n}\n";
    assert_eq!(found(file, pathed), [format!("{file}:4: {LOOKS}")]);
    assert_eq!(found("app/src/popup.rs", pathed), [format!("app/src/popup.rs:4: {LOOKS}")]);
}

#[test]
fn check_runs_the_rule() {
    let root = tempfile::tempdir().unwrap();
    for file in ["crates/jellymoon/tests/f99.rs", "app/tests/f99.rs"] {
        fs::create_dir_all(root.path().join(file).parent().unwrap()).unwrap();
        fs::write(root.path().join(file), starts(PROGRAM)).unwrap();
        fs::write(root.path().join("entropy.toml"), "max_file_lines = 400\n").unwrap();
        let found = super::super::check(root.path(), &[file.to_string()]).unwrap();
        assert_eq!(found, [format!("{file}:4: {NAMES}")]);
    }
}
