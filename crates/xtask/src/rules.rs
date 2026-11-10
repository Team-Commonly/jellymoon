//! The rules `cargo xtask check` holds the repo to. Each takes the repo's root and the tracked files and returns one
//! line for each thing that breaks it. How long a function may be is not here: clippy checks that (`clippy.toml`).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use toml::Table;

/// Runs every rule with the numbers in `entropy.toml`.
pub fn check(root: &Path, files: &[String]) -> Result<Vec<String>, String> {
    let config = fs::read_to_string(root.join("entropy.toml")).map_err(|e| format!("entropy.toml: {e}"))?;
    let config: Table = config.parse().map_err(|e| format!("entropy.toml: {e}"))?;
    let max_lines = config
        .get("max_file_lines")
        .and_then(|max| max.as_integer())
        .ok_or("entropy.toml: max_file_lines is not a number")?;
    let table = |name: &str| config.get(name).and_then(|table| table.as_table()).cloned().unwrap_or_default();
    let crates = crates(root, files);
    let mut found = long_files(root, files, usize::try_from(max_lines).unwrap_or(0));
    found.extend(over_budget(root, files, &crates, &table("budget")));
    found.extend(local_paths(root, files));
    found.extend(wrong_direction(&crates, &table("depends")));
    Ok(found)
}

/// A workspace crate: its folder from the root, and its manifest.
struct Crate {
    folder: String,
    manifest: Table,
}

impl Crate {
    fn name(&self) -> &str {
        self.manifest
            .get("package")
            .and_then(|package| package.get("name"))
            .and_then(|name| name.as_str())
            .unwrap_or(&self.folder)
    }

    /// Every crate this one names as a dependency of any kind.
    fn dependencies(&self) -> Vec<&str> {
        let sections = ["dependencies", "dev-dependencies", "build-dependencies"];
        sections
            .iter()
            .filter_map(|section| self.manifest.get(*section)?.as_table())
            .flat_map(|table| table.keys().map(String::as_str))
            .collect()
    }
}

/// The crates under `crates/`, by the manifests that are tracked.
fn crates(root: &Path, files: &[String]) -> Vec<Crate> {
    let manifest = |file: &String| {
        let folder = file
            .strip_suffix("/Cargo.toml")
            .filter(|folder| folder.starts_with("crates/") && folder.matches('/').count() == 1)?;
        Some(Crate { folder: folder.to_string(), manifest: fs::read_to_string(root.join(file)).ok()?.parse().ok()? })
    };
    files.iter().filter_map(manifest).collect()
}

/// A tracked file's text. None for a file that is not text, or that is tracked and no longer there.
fn text(root: &Path, file: &str) -> Option<String> {
    String::from_utf8(fs::read(root.join(file)).ok()?).ok()
}

fn rust_lines(root: &Path, file: &str) -> Option<usize> {
    Some(text(root, file).filter(|_| file.ends_with(".rs"))?.lines().count())
}

/// No Rust file is longer than `max` lines.
fn long_files(root: &Path, files: &[String], max: usize) -> Vec<String> {
    let long = |file: &String| {
        let lines = rust_lines(root, file).filter(|lines| *lines > max)?;
        Some(format!("{file}: {lines} lines, over the {max} a Rust file may have"))
    };
    files.iter().filter_map(long).collect()
}

/// Every crate has a budget of Rust lines, tests included, and is within it.
fn over_budget(root: &Path, files: &[String], crates: &[Crate], budget: &Table) -> Vec<String> {
    let mut found = Vec::new();
    for folder in crates.iter().map(|one| &one.folder) {
        let inside = format!("{folder}/");
        let lines: usize =
            files.iter().filter(|file| file.starts_with(&inside)).filter_map(|file| rust_lines(root, file)).sum();
        match budget.get(folder).and_then(|max| max.as_integer()) {
            None => found.push(format!("{folder}: no budget of Rust lines in entropy.toml [budget]")),
            Some(max) if i64::try_from(lines).unwrap_or(i64::MAX) > max => {
                found.push(format!("{folder}: {lines} lines of Rust, over its budget of {max} (entropy.toml)"));
            }
            Some(_) => {}
        }
    }
    found
}

/// What a machine-local path starts with, whether it only counts at the start of a path, and what must follow it.
type Shape = (&'static str, bool, Option<fn(char) -> bool>);

/// A home directory of any user, on a Mac, on Linux and on Windows, and the Mac's own temp folder. The last is written
/// in two pieces so this file does not hold it.
const LOCAL: [Shape; 4] = [
    ("/Users/", true, Some(char::is_alphanumeric)),
    ("/home/", true, Some(|c| c.is_ascii_lowercase())),
    ("\\Users\\", false, Some(char::is_alphanumeric)),
    (concat!("/private", "/tmp"), false, None),
];

/// No tracked file names a path that exists on one machine only.
fn local_paths(root: &Path, files: &[String]) -> Vec<String> {
    let mut found = Vec::new();
    for file in files {
        let Some(text) = text(root, file) else {
            continue;
        };
        for (number, line) in text.lines().enumerate() {
            if let Some(path) = LOCAL.iter().find_map(|shape| local_path(line, shape)) {
                found
                    .push(format!("{file}:{}: a machine-local path ({path}…); use a path inside the repo", number + 1));
            }
        }
    }
    found
}

/// The start of a machine-local path of this shape in the line, when it holds one.
fn local_path<'a>(line: &'a str, (start, at_path_start, next): &Shape) -> Option<&'a str> {
    line.match_indices(start).find_map(|(at, _)| {
        // "/home/" in the middle of a longer path or a URL (a site's home page, say) is a folder of that name, not a home.
        let inside_a_path =
            line[..at].chars().next_back().is_some_and(|before| before.is_alphanumeric() || "_./-".contains(before));
        let rest = &line[at + start.len()..];
        let followed = next.is_none_or(|is| rest.chars().next().is_some_and(is));
        (followed && !(*at_path_start && inside_a_path)).then(|| &line[at..at + start.len()])
    })
}

/// Every crate is listed under `[depends]` with the workspace crates it may depend on, and depends on no other.
fn wrong_direction(crates: &[Crate], depends: &Table) -> Vec<String> {
    let allowed: BTreeMap<&str, Vec<&str>> = depends
        .iter()
        .filter_map(|(name, list)| {
            Some((name.as_str(), list.as_array()?.iter().filter_map(|one| one.as_str()).collect()))
        })
        .collect();
    let mut found = Vec::new();
    for one in crates {
        let Some(allowed) = allowed.get(one.name()) else {
            found.push(format!(
                "{}: entropy.toml [depends] does not say which workspace crates {} may depend on",
                one.folder,
                one.name()
            ));
            continue;
        };
        for dependency in one.dependencies() {
            if crates.iter().any(|other| other.name() == dependency) && !allowed.contains(&dependency) {
                found.push(format!(
                    "{}/Cargo.toml: {} may not depend on {dependency} (entropy.toml [depends])",
                    one.folder,
                    one.name()
                ));
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENTROPY: &str = "max_file_lines = 400\n[budget]\n\"crates/core\" = 500\n\"crates/bin\" = 500\n[depends]\ncore = []\nbin = [\"core\"]\n";

    /// A small repo that breaks no rule, then `change` applied to it: its files by name, and what `check` finds.
    fn found(change: &[(&str, &str)]) -> Vec<String> {
        let root = tempfile::tempdir().unwrap();
        let manifest = |name: &str, depends: &str| format!("[package]\nname = \"{name}\"\n[dependencies]\n{depends}");
        let mut files = BTreeMap::from([
            ("entropy.toml", ENTROPY.to_string()),
            ("crates/core/Cargo.toml", manifest("core", "serde = \"1\"\n")),
            ("crates/core/src/lib.rs", "pub fn one() {}\n".repeat(300)),
            ("crates/bin/Cargo.toml", manifest("bin", "core = { path = \"../core\" }\n")),
            ("crates/bin/src/main.rs", "fn main() {}\n".to_string()),
            ("docs/notes.md", format!("See https://example.com{}page and docs{}guide.\n", "/home/", "/Users/")),
            ("art/dot.png", String::new()),
        ]);
        files.extend(change.iter().map(|(name, text)| (*name, text.to_string())));
        for (name, text) in &files {
            fs::create_dir_all(root.path().join(name).parent().unwrap()).unwrap();
            fs::write(root.path().join(name), text).unwrap();
        }
        fs::write(root.path().join("art/dot.png"), [0x89, 0x50, 0xff, 0x00]).unwrap();
        let names: Vec<String> = files.keys().map(|name| name.to_string()).collect();
        check(root.path(), &names).unwrap()
    }

    fn assert_one(found: &[String], want: &str) {
        assert!(found.len() == 1 && found[0].contains(want), "want one line holding {want:?}, found {found:?}");
    }

    #[test]
    fn a_repo_that_breaks_no_rule_passes() {
        assert_eq!(found(&[]), Vec::<String>::new());
    }

    #[test]
    fn a_rust_file_over_400_lines_is_refused_and_one_of_400_is_not() {
        assert_eq!(found(&[("crates/core/src/lib.rs", &"// line\n".repeat(400))]), Vec::<String>::new());
        assert_one(
            &found(&[("crates/core/src/lib.rs", &"// line\n".repeat(401))]),
            "crates/core/src/lib.rs: 401 lines",
        );
        assert_eq!(
            found(&[("docs/long.md", &"line\n".repeat(900))]),
            Vec::<String>::new(),
            "only Rust files are counted"
        );
    }

    #[test]
    fn a_crate_over_its_budget_or_with_none_is_refused() {
        let more = [
            ("crates/core/src/more.rs", "pub fn two() {}\n".repeat(150)),
            ("crates/core/tests/t.rs", "// a test\n".repeat(51)),
        ];
        let more: Vec<(&str, &str)> = more.iter().map(|(name, text)| (*name, text.as_str())).collect();
        assert_one(&found(&more), "crates/core: 501 lines of Rust, over its budget of 500");
        let unlisted = [
            ("crates/new/Cargo.toml", "[package]\nname = \"new\"\n"),
            ("entropy.toml", &format!("{ENTROPY}new = []\n")),
        ];
        assert_one(&found(&unlisted), "crates/new: no budget");
    }

    #[test]
    fn a_machine_local_path_in_any_tracked_text_file_is_refused() {
        let cases = [
            ("a Mac home", format!("cd {}sam/code\n", "/Users/")),
            ("a Linux home", format!("see {}runner/work\n", "/home/")),
            ("a Windows home", format!("open C:{}sam\\code\n", "\\Users\\")),
            ("the Mac's temp folder", format!("in {}/tmp/x\n", "/private")),
        ];
        for (name, text) in cases {
            let text = format!("a first line\n{text}");
            assert_one(&found(&[("docs/setup.md", &text)]), "docs/setup.md:2: a machine-local path");
            assert_one(&found(&[("crates/core/src/more.rs", &text)]), "crates/core/src/more.rs:2");
            assert!(!text.is_empty(), "{name}");
        }
    }

    #[test]
    fn a_crate_that_depends_against_the_direction_or_is_not_listed_is_refused() {
        let core_on_bin = "[package]\nname = \"core\"\n[dev-dependencies]\nbin = { path = \"../bin\" }\n";
        assert_one(&found(&[("crates/core/Cargo.toml", core_on_bin)]), "core may not depend on bin");
        let unlisted = [
            ("crates/new/Cargo.toml", "[package]\nname = \"new\"\n"),
            ("entropy.toml", &ENTROPY.replace("[depends]", "\"crates/new\" = 10\n[depends]")),
        ];
        assert_one(&found(&unlisted), "does not say which workspace crates new may depend on");
    }

    #[test]
    fn a_missing_or_broken_entropy_file_is_an_error_not_a_pass() {
        let root = tempfile::tempdir().unwrap();
        assert!(check(root.path(), &[]).is_err());
        fs::write(root.path().join("entropy.toml"), "max_file_lines = \"many\"\n").unwrap();
        assert!(check(root.path(), &[]).is_err());
    }
}
