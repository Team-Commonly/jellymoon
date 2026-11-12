//! The repo's own commands. `cargo xtask check` runs the rules in `rules.rs`, whose numbers are in `entropy.toml`
//! at the repo's root, over the files git tracks. `cargo xtask smoke` builds the real programs and runs them
//! together (`smoke.rs`, `docs/development/smoke.md`).

mod rules;
mod smoke;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::{env, str};

const USAGE: &str = "usage:
  cargo xtask check
  cargo xtask smoke [--window] [--real [--skip <folder names, comma separated>]] [-- <cargo build arguments>]";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let Some(root) = env::current_dir().ok().and_then(|dir| root(&dir)) else {
        eprintln!("xtask: run it inside the repo: no folder above this one holds entropy.toml");
        return ExitCode::FAILURE;
    };
    let done = match args.split_first() {
        Some((command, [])) if command == "check" => check(&root),
        Some((command, rest)) if command == "smoke" => match smoke::Options::parse(rest) {
            Ok(options) => smoke::run(&root, &options).map_err(|why| format!("smoke: FAILED at {why}")),
            Err(Some(why)) => Err(format!("smoke: {why}")),
            Err(None) => return usage(),
        },
        _ => return usage(),
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("{why}");
            ExitCode::FAILURE
        }
    }
}

fn usage() -> ExitCode {
    eprintln!("{USAGE}");
    ExitCode::from(2)
}

/// The repo's root: the nearest folder at or above `dir` that holds `entropy.toml`. Found when the program runs, not
/// when it is built: worktrees that share a target folder also share this binary, and each must check its own files.
fn root(dir: &Path) -> Option<PathBuf> {
    dir.ancestors().find(|dir| dir.join("entropy.toml").is_file()).map(Path::to_path_buf)
}

fn check(root: &Path) -> Result<(), String> {
    let files = tracked(root).map_err(|why| format!("xtask check: {why}"))?;
    let found = rules::check(root, &files).map_err(|why| format!("xtask check: {why}"))?;
    if found.is_empty() {
        println!("xtask check: ok, {} tracked files", files.len());
        return Ok(());
    }
    found.iter().for_each(|broken| eprintln!("{broken}"));
    Err(format!("xtask check: {} broken", found.len()))
}

/// The files git tracks, as paths from the repo's root with `/` between segments.
fn tracked(root: &Path) -> Result<Vec<String>, String> {
    let listed = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-z"])
        .output()
        .map_err(|e| format!("running git: {e}"))?;
    if !listed.status.success() {
        return Err(format!("git ls-files: {}", String::from_utf8_lossy(&listed.stderr).trim()));
    }
    let names = str::from_utf8(&listed.stdout).map_err(|e| format!("git ls-files: {e}"))?;
    Ok(names.split('\0').filter(|name| !name.is_empty()).map(str::to_string).collect())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use std::fs;

    #[test]
    fn the_root_is_found_from_where_the_program_runs_not_where_it_was_built() {
        let repo = tempfile::tempdir().unwrap();
        let inside = repo.path().join("crates").join("one");
        fs::create_dir_all(&inside).unwrap();
        assert_eq!(super::root(&inside), None, "no entropy.toml above");
        fs::write(repo.path().join("entropy.toml"), "").unwrap();
        assert_eq!(super::root(&inside).as_deref(), Some(repo.path()));
        assert_eq!(super::root(repo.path()).as_deref(), Some(repo.path()));
    }
}
