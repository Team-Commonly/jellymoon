//! A name the person's tools obey is marked in every spelling a disk takes for it (`docs/development/show.md`,
//! "Files the person's tools obey"): a run's agent needs only to write a file to make such a name, and a mark
//! that a spelling slips past is no warning. The work is put in the repo's store only, so the spellings a Mac
//! or Windows could not hold apart are tested on every system.

use serde_json::Value;

use super::{Entry, Made, finished, printed};

/// Which of the names `run show` marks obeyed, after the work is made of one file for each.
fn marks(made: &Made, names: &[String]) -> Vec<bool> {
    let entries: Vec<Entry> = names.iter().map(|name| ("100644", name.as_str(), &b"x\n"[..])).collect();
    made.rework(&entries);
    let shown = printed(&made.show(&[]));
    let files = shown["files"].as_array().unwrap();
    assert_eq!((files.len(), &shown["more"]), (names.len(), &Value::from(0)), "{shown}");
    files.iter().map(|file| file["obeyed"].as_bool().unwrap()).collect()
}

fn all(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| name.to_string()).collect()
}

#[test]
fn a_spelling_a_mac_disk_takes_for_an_obeyed_name_is_marked() {
    let made = finished();
    let names = all(&[
        // APFS takes each of these characters for the letters of the name.
        ".cur\u{17f}or/rules/x.mdc",
        ".hu\u{17f}ky/pre-commit",
        ".v\u{17f}code/tasks.json",
        ".mcp.j\u{17f}on",
        ".gitattribute\u{17f}",
        "sub/AGENT\u{17f}.md",
        ".hus\u{212a}y/hook",
        ".github/work\u{fb02}ows/ci.yml",
        ".pre-commit-con\u{fb01}g.yaml",
        ".github/copilot-in\u{fb05}ructions.md",
        ".github/copilot-in\u{fb06}ructions.md",
        // HFS+ leaves the joiners out of a name, and cleaning keeps them.
        ".cl\u{200d}aude/x",
        "web/.en\u{200c}vrc",
    ]);
    assert_eq!(marks(&made, &names), vec![true; names.len()], "{names:?}");
}

#[test]
fn a_spelling_windows_takes_for_an_obeyed_name_is_marked() {
    let made = finished();
    let names = all(&[
        ".claude /x",
        ".codex./x",
        ".cursor\\rules\\x.mdc",
        "docs/CLAUDE.md:stream",
        ".g\u{131}tmodules",
        "VSCODE~1/tasks.json",
    ]);
    assert_eq!(marks(&made, &names), vec![true; names.len()], "{names:?}");
}

#[test]
fn every_name_of_the_list_is_marked_and_a_near_miss_no_disk_folds_is_not() {
    let made = finished();
    let names = all(&[
        ".cargo/config.toml",
        ".cursorrules",
        ".pre-commit-config.yaml",
        "docs/GEMINI.md",
        "sub/.github/Copilot-Instructions.md",
    ]);
    assert_eq!(marks(&made, &names), vec![true; names.len()], "{names:?}");
    let near = all(&[
        ".cargo.toml",
        ".claudex/a",
        ".github/x/copilot-instructions.md",
        // A Cyrillic letter that looks like `c`: another name on every disk.
        ".\u{441}laude/x",
        "a~b/c~",
        "copilot-instructions.md",
        "xAGENTS.md",
    ]);
    assert_eq!(marks(&made, &near), vec![false; near.len()], "{near:?}");
}

#[test]
fn a_name_is_asked_as_git_keeps_it_so_one_past_where_the_printed_name_is_cut_is_marked() {
    let made = finished();
    let long = format!("deep/{}/.claude/settings.json", "n".repeat(300));
    assert_eq!(marks(&made, &[long, "deep/plain.txt".to_string()]), [true, false]);
}
