//! The rule's table, and on a Mac the disk asked whether the table is whole: where the disk takes one spelling
//! for another, the rule must.

use super::*;

#[test]
fn a_path_is_obeyed_by_a_whole_name_in_it_in_any_case_and_never_by_part_of_one() {
    let yes = [
        ".claude/settings.json",
        "a/b/.CLAUDE/x",
        "claude.md",
        "deep/Agents.md",
        ".github/workflows/ci.yml",
        "x/.GitHub/WORKFLOWS/y",
        ".envrc",
        ".vscode",
        ".gitmodules",
    ];
    let no = [
        "claude/settings.json",
        ".claude.bak/x",
        "CLAUDE.md.txt",
        "my.envrc",
        ".github/ISSUE_TEMPLATE.md",
        ".github/x/workflows/ci.yml",
        "workflows/.github",
        "",
    ];
    assert_eq!(yes.map(obeyed), [true; 9]);
    assert_eq!(no.map(obeyed), [false; 8]);
}

#[test]
fn every_name_of_the_list_is_obeyed_alone_and_inside_a_folder() {
    for name in OBEYED {
        assert!(obeyed(name) && obeyed(&format!("a/{name}/b")) && obeyed(&name.to_uppercase()), "{name}");
    }
    let new = [".cursorrules", ".github/copilot-instructions.md", ".cargo", "GEMINI.md", ".pre-commit-config.yaml"];
    assert!(new.iter().all(|name| OBEYED.contains(name)));
    // A name of two folders is obeyed only as the two, one inside the other.
    let apart = ["copilot-instructions.md", ".github", ".github/x/copilot-instructions.md", "workflows/ci.yml"];
    assert_eq!(apart.map(obeyed), [false; 4]);
}

#[test]
fn every_spelling_a_disk_was_found_to_take_for_an_obeyed_name_is_obeyed() {
    let found = [
        // APFS: one character for one letter, and for two and three.
        ".cur\u{17f}or/rules/x.mdc",
        ".hu\u{17f}ky/pre-commit",
        ".v\u{17f}code/tasks.json",
        ".mcp.j\u{17f}on",
        ".gitattribute\u{17f}",
        "sub/AGENT\u{17f}.md",
        ".hus\u{212a}y/x",
        ".github/wor\u{212a}flows/x",
        ".github/work\u{fb02}ows/ci.yml",
        ".pre-commit-con\u{fb01}g.yaml",
        ".github/copilot-in\u{fb05}ructions.md",
        ".github/copilot-in\u{fb06}ructions.md",
        // HFS+: sixteen characters left out, wherever they stand.
        ".cl\u{200d}aude/x",
        "\u{200c}.claude/x",
        ".claude\u{feff}/x",
        ".co\u{200e}\u{200f}dex/x",
        ".cur\u{202a}\u{202b}\u{202c}\u{202d}\u{202e}sor/x",
        ".en\u{206a}\u{206b}\u{206c}\u{206d}\u{206e}\u{206f}vrc",
    ];
    for path in found {
        assert!(obeyed(path), "{}", path.escape_unicode());
    }
    // No obeyed name holds `ss`, `ff`, `ffi` or `ffl` today: those folds are asked for as letters.
    let letters = [("\u{df}", "ss"), ("\u{1e9e}", "ss"), ("\u{fb00}", "ff"), ("\u{fb03}", "ffi"), ("\u{fb04}", "ffl")];
    for (one, many) in letters {
        assert_eq!(folded(&format!("a{one}b")), format!("a{many}b"));
    }
}

#[test]
fn the_spellings_windows_takes_for_an_obeyed_name_are_obeyed() {
    let reasoned = [
        ".claude /x",
        ".claude./x",
        ".claude. . /x",
        ".claude\\x",
        "a\\.vscode\\tasks.json",
        ".github\\workflows/ci.yml",
        "CLAUDE.md:stream",
        ".claude::$INDEX_ALLOCATION/x",
        ".envrc .:$DATA",
        ".g\u{131}tmodules",
        "CLAUDE~1/x",
        "a/GITATT~1",
        "PRE-CO~1.YAM",
    ];
    for path in reasoned {
        assert!(obeyed(path), "{path}");
    }
}

#[test]
fn a_near_miss_no_disk_folds_is_not_obeyed() {
    let no = [
        ".claudex/a",
        "xAGENTS.md",
        "x.claude/a",
        // Cyrillic `с` and `а`, and a fullwidth `c`: three other names on every disk.
        ".\u{441}laude/x",
        ".cl\u{430}ude/x",
        ".\u{ff43}laude/x",
        // An accent is kept by every disk, and a space or dot not at the end is part of the name.
        ".cla\u{301}ude/x",
        ". claude/x",
        ".cla ude/x",
        "..claude/x",
        "x:.claude/y",
        "a~b/c~",
        "v2/notes10.txt",
        ".cargo.toml",
        "cargo/config.toml",
    ];
    for path in no {
        assert!(!obeyed(path), "{}", path.escape_unicode());
    }
}

#[cfg(target_os = "macos")]
mod disk {
    use std::collections::HashMap;
    use std::fs;
    use std::os::unix::fs::MetadataExt;
    use std::path::Path;

    use super::*;

    /// Every spelling of a name the table knows: each of the folds for its letters where the name has them, each
    /// left-out character first, inside and last, and the ends Windows drops.
    fn spellings(name: &str) -> Vec<String> {
        let lower = name.to_ascii_lowercase();
        let mut all = Vec::new();
        for (taken, letters) in FOLDS {
            for (at, _) in lower.match_indices(letters) {
                all.push(format!("{}{taken}{}", &name[..at], &name[at + letters.len()..]));
            }
        }
        for left_out in IGNORED.into_iter().flatten() {
            all.extend([format!("{left_out}{name}"), format!("{}{left_out}{}", &name[..2], &name[2..])]);
            all.push(format!("{name}{left_out}"));
        }
        all.extend([format!("{name}."), format!("{name} "), name.to_uppercase()]);
        all
    }

    #[test]
    fn where_the_disk_takes_a_spelling_for_an_obeyed_name_the_rule_says_obeyed() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("case"), "").unwrap();
        let folds = dir.path().join("CASE").exists();
        let (mut asked, mut taken) = (0, 0);
        for name in OBEYED {
            for spelling in spellings(name) {
                asked += 1;
                let at = dir.path().join(asked.to_string());
                let made = at.join(&spelling);
                fs::create_dir_all(made.parent().unwrap()).unwrap();
                fs::write(&made, "").unwrap();
                if at.join(name).exists() {
                    taken += 1;
                    assert!(obeyed(&spelling), "the disk takes {} for {name}", spelling.escape_unicode());
                }
            }
        }
        // A disk that folds case takes at least each name in capitals: the question was asked.
        assert!(asked > OBEYED.len() * 50 && (!folds || taken >= OBEYED.len()), "{asked} asked, {taken} taken");
    }

    /// Every character of the Basic Multilingual Plane that is not ASCII, and the tags and variation selectors
    /// above it.
    fn characters() -> impl Iterator<Item = char> {
        ('\u{80}'..='\u{ffff}').chain('\u{e0000}'..='\u{e01ef}')
    }

    fn file(dir: &Path, name: &str) -> u64 {
        fs::write(dir.join(name), "").unwrap();
        fs::symlink_metadata(dir.join(name)).unwrap().ino()
    }

    #[test]
    fn the_disk_takes_no_character_for_ascii_and_leaves_none_out_that_the_table_does_not_know() {
        let dir = tempfile::tempdir().unwrap();
        let (plain, joined) = (dir.path().join("plain"), dir.path().join("joined"));
        fs::create_dir_all(&plain).unwrap();
        fs::create_dir_all(&joined).unwrap();
        // Every name of one and of two ASCII letters, digits, `.`, `-` and `_`, known again by its inode.
        let ascii: Vec<char> = ('a'..='z').chain('0'..='9').chain(['.', '-', '_']).collect();
        let mut names = HashMap::new();
        for a in &ascii {
            for name in ascii.iter().map(|b| format!("{a}{b}")).chain([a.to_string()]) {
                if !name.trim_matches('.').is_empty() {
                    names.insert(file(&plain, &name), name);
                }
            }
        }
        // Every name of three is too many to make on each run: the two the measuring found are made.
        for three in ["ffi", "ffl"] {
            names.insert(file(&plain, three), three.to_string());
        }
        file(&joined, "xy");
        for c in characters() {
            if let Ok(found) = fs::symlink_metadata(plain.join(c.to_string())) {
                let name = &names[&found.ino()];
                assert!(folded(&c.to_string()).eq_ignore_ascii_case(name), "the disk takes {c:?} for {name}");
            }
            if joined.join(format!("x{c}y")).exists() {
                assert_eq!(folded(&format!("x{c}y")), "xy", "the disk leaves {c:?} out of a name");
            }
        }
    }
}
