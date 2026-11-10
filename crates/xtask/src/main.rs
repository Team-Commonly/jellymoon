//! The repo's own checks, run as `cargo xtask check` over the files git tracks. The rules are in `rules.rs` and their
//! numbers in `entropy.toml` at the repo's root.

mod rules;

use std::path::Path;
use std::process::{Command, ExitCode};
use std::{env, str};

fn main() -> ExitCode {
    if env::args().skip(1).collect::<Vec<_>>() != ["check"] {
        eprintln!("usage: cargo xtask check");
        return ExitCode::from(2);
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    match tracked(&root).and_then(|files| Ok((files.len(), rules::check(&root, &files)?))) {
        Ok((files, found)) if found.is_empty() => {
            println!("xtask check: ok, {files} tracked files");
            ExitCode::SUCCESS
        }
        Ok((_, found)) => {
            found.iter().for_each(|broken| eprintln!("{broken}"));
            eprintln!("xtask check: {} broken", found.len());
            ExitCode::FAILURE
        }
        Err(why) => {
            eprintln!("xtask check: {why}");
            ExitCode::FAILURE
        }
    }
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
