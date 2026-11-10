//! The rules `cargo xtask check` holds the repo to. Each takes the repo's root and the tracked files and returns one
//! line for each thing that breaks it. How long a function may be is not here: clippy checks that (`clippy.toml`).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use toml::{Table, Value};

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
    found.extend(wrong_dependencies(&crates, &table("depends"), &table("forbidden")));
    Ok(found)
}

/// A workspace crate: its folder from the root, its manifest, and the package behind each dependency it lists.
struct Crate {
    folder: String,
    manifest: Table,
    dependencies: Vec<String>,
}

impl Crate {
    fn name(&self) -> &str {
        self.manifest
            .get("package")
            .and_then(|package| package.get("name"))
            .and_then(|name| name.as_str())
            .unwrap_or(&self.folder)
    }
}

/// The package behind every dependency a manifest lists, each once. All three kinds count, at the top and under every
/// `[target.*]`. One renamed with `package = "…"` is the package, not the name it goes by here; one taken from the
/// workspace (`workspace = true`) is read from the root manifest's table, where the rename would be.
fn dependencies(manifest: &Table, workspace: &Table) -> Vec<String> {
    let targets = manifest.get("target").and_then(Value::as_table);
    let holders = targets.into_iter().flat_map(Table::values).filter_map(Value::as_table).chain([manifest]);
    let mut found = Vec::new();
    for holder in holders {
        for kind in ["dependencies", "dev-dependencies", "build-dependencies"] {
            for (name, listed) in holder.get(kind).and_then(Value::as_table).into_iter().flatten() {
                let shared = listed.get("workspace").and_then(Value::as_bool) == Some(true);
                let spec = workspace.get(name).filter(|_| shared).unwrap_or(listed);
                found.push(spec.get("package").and_then(Value::as_str).unwrap_or(name).to_string());
            }
        }
    }
    found.sort();
    found.dedup();
    found
}

/// Every crate in the repo, wherever its folder is (`crates/<name>`, and `app`): each tracked manifest below the root.
fn crates(root: &Path, files: &[String]) -> Vec<Crate> {
    let workspace = text(root, "Cargo.toml").and_then(|manifest| manifest.parse::<Table>().ok());
    let workspace = workspace
        .as_ref()
        .and_then(|manifest| manifest.get("workspace")?.get("dependencies")?.as_table())
        .cloned()
        .unwrap_or_default();
    let one = |file: &String| {
        let folder = file.strip_suffix("/Cargo.toml")?;
        let manifest: Table = fs::read_to_string(root.join(file)).ok()?.parse().ok()?;
        Some(Crate { folder: folder.to_string(), dependencies: dependencies(&manifest, &workspace), manifest })
    };
    files.iter().filter_map(one).collect()
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

/// A table of entropy.toml that gives each crate a list of names.
fn lists(table: &Table) -> BTreeMap<&str, Vec<&str>> {
    table
        .iter()
        .filter_map(|(name, list)| Some((name.as_str(), list.as_array()?.iter().filter_map(Value::as_str).collect())))
        .collect()
}

/// Every crate is listed under `[depends]` with the workspace crates it may depend on, and depends on no other. A crate
/// under `[forbidden]` lists none of the outside packages named there; a name that ends in `*` bars every package that
/// starts with what is before it. Both look at what a manifest lists, so neither sees what a dependency brings in.
fn wrong_dependencies(crates: &[Crate], depends: &Table, forbidden: &Table) -> Vec<String> {
    let (allowed, forbidden) = (lists(depends), lists(forbidden));
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
        let barred = forbidden.get(one.name()).map(Vec::as_slice).unwrap_or_default();
        for dependency in one.dependencies.iter().map(String::as_str) {
            let against = if crates.iter().any(|other| other.name() == dependency) && !allowed.contains(&dependency) {
                "depends"
            } else if barred.iter().any(|name| match name.strip_suffix('*') {
                Some(start) => dependency.starts_with(start),
                None => *name == dependency,
            }) {
                "forbidden"
            } else {
                continue;
            };
            found.push(format!(
                "{}/Cargo.toml: {} may not depend on {dependency} (entropy.toml [{against}])",
                one.folder,
                one.name()
            ));
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENTROPY: &str = "max_file_lines = 400\n[budget]\n\"crates/core\" = 500\n\"crates/bin\" = 500\n[depends]\ncore = []\nbin = [\"core\"]\n[forbidden]\ncore = [\"gpui*\", \"tokio\"]\n";

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
            ("entropy.toml", &ENTROPY.replace("[forbidden]", "new = []\n[forbidden]")),
        ];
        assert_one(&found(&unlisted), "crates/new: no budget");
    }

    #[test]
    fn a_machine_local_path_in_any_tracked_text_file_is_refused() {
        // A Mac home, a Linux home, a Windows home and the Mac's temp folder.
        let cases = [
            format!("cd {}sam/code\n", "/Users/"),
            format!("see {}runner/work\n", "/home/"),
            format!("open C:{}sam\\code\n", "\\Users\\"),
            format!("in {}/tmp/x\n", "/private"),
        ];
        for text in cases {
            let text = format!("a first line\n{text}");
            assert_one(&found(&[("docs/setup.md", &text)]), "docs/setup.md:2: a machine-local path");
            assert_one(&found(&[("crates/core/src/more.rs", &text)]), "crates/core/src/more.rs:2");
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
    fn a_dependency_against_the_direction_is_seen_under_another_name_and_in_a_target_table() {
        let manifests = [
            "[dependencies]\nother = { package = \"bin\", path = \"../bin\" }\n",
            "[target.'cfg(unix)'.dependencies]\nbin = { path = \"../bin\" }\n",
            "[target.x86_64-pc-windows-msvc.build-dependencies]\nother = { package = \"bin\", path = \"../bin\" }\n",
        ];
        for manifest in manifests {
            let core = format!("[package]\nname = \"core\"\n{manifest}");
            assert_one(
                &found(&[("crates/core/Cargo.toml", &core)]),
                "core may not depend on bin (entropy.toml [depends])",
            );
        }
    }

    #[test]
    fn a_forbidden_dependency_is_refused_in_every_table_and_under_any_name() {
        let manifests = [
            "[dependencies]\ngpui = \"1\"\n",
            "[dev-dependencies]\ngpui = \"1\"\n",
            "[build-dependencies]\ngpui = \"1\"\n",
            "[dependencies]\ngpui = \"1\"\n[dev-dependencies]\ngpui = \"1\"\n",
            "[target.'cfg(windows)'.dependencies]\ngpui = \"1\"\n",
            "[target.x86_64-apple-darwin.dev-dependencies]\ngpui = { version = \"1\" }\n",
            "[target.'cfg(unix)'.build-dependencies]\ngpui = \"1\"\n",
            "[dependencies]\nui = { package = \"gpui\", version = \"1\" }\n",
            "[dependencies]\nui.workspace = true\n",
            "[dependencies]\ngpui.workspace = true\n",
        ];
        let workspace = "[workspace.dependencies]\nui = { package = \"gpui\", version = \"1\" }\ngpui = \"1\"\n";
        for manifest in manifests {
            let core = format!("[package]\nname = \"core\"\n{manifest}");
            let found = found(&[("crates/core/Cargo.toml", &core), ("Cargo.toml", workspace)]);
            assert_one(&found, "crates/core/Cargo.toml: core may not depend on gpui (entropy.toml [forbidden])");
        }
        // `gpui*` bars every package that starts so; a name with no star bars that package alone.
        for package in ["gpui_platform", "gpui_wgpu", "tokio"] {
            let other = format!("[package]\nname = \"core\"\n[dependencies]\n{package} = \"1\"\n");
            assert_one(&found(&[("crates/core/Cargo.toml", &other)]), &format!("core may not depend on {package}"));
        }
        for package in ["gpu", "wgpui", "tokio-util"] {
            let other = format!("[package]\nname = \"core\"\n[dependencies]\n{package} = \"1\"\n");
            assert_eq!(found(&[("crates/core/Cargo.toml", &other)]), Vec::<String>::new(), "{package}");
        }
        // Only the crates the table names: the binary of this small repo is not one of them.
        let bin = "[package]\nname = \"bin\"\n[dependencies]\ncore = { path = \"../core\" }\ngpui = \"1\"\n";
        assert_eq!(found(&[("crates/bin/Cargo.toml", bin)]), Vec::<String>::new());
    }

    #[test]
    fn a_crate_outside_the_crates_folder_is_held_to_the_same_rules() {
        let shell =
            ("app/Cargo.toml", "[package]\nname = \"shell\"\n[dependencies]\ncore = { path = \"../crates/core\" }\n");
        let unlisted = found(&[shell]);
        assert!(unlisted.iter().any(|line| line.contains("app: no budget")), "{unlisted:?}");
        assert!(unlisted.iter().any(|line| line.contains("may depend on")), "{unlisted:?}");
        let entropy = |depends: &str| {
            let listed = format!("\"app\" = 10\n[depends]\nshell = [{depends}]");
            ENTROPY.replace("[depends]", &listed)
        };
        assert_one(&found(&[shell, ("entropy.toml", &entropy(""))]), "shell may not depend on core");
        let allowed = entropy("\"core\"");
        assert_eq!(found(&[shell, ("entropy.toml", &allowed)]), Vec::<String>::new());
        let long = "// line\n".repeat(11);
        assert_one(
            &found(&[shell, ("entropy.toml", &allowed), ("app/src/main.rs", &long)]),
            "app: 11 lines of Rust, over its budget of 10",
        );
    }

    #[test]
    fn a_missing_or_broken_entropy_file_is_an_error_not_a_pass() {
        let root = tempfile::tempdir().unwrap();
        assert!(check(root.path(), &[]).is_err());
        fs::write(root.path().join("entropy.toml"), "max_file_lines = \"many\"\n").unwrap();
        assert!(check(root.path(), &[]).is_err());
    }
}
