//! The build the chain starts and the temporary homes it works in. The chain runs the programs of the debug build
//! in the target folder, so the build takes only arguments from a list of ones that leave it there and build all of
//! it. A list of refused ones was tried first, and twice an argument nobody had thought of passed it: `--help`
//! made cargo print its help and build nothing, and the chain then passed over programs of an earlier build.

use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, path};

/// The build arguments that stand alone. Each builds the same programs in the same place.
const ALONE: [&str; 8] = ["--locked", "--offline", "--frozen", "-v", "-vv", "--verbose", "-q", "--quiet"];

/// The build arguments that take a value, after a space or an `=`, and for the short ones written on to them.
/// `--features` and `-F` only add to what is built; `--no-default-features` would take away, and is not here.
const VALUED: [&str; 4] = ["--jobs", "-j", "--features", "-F"];

/// The variables that move a build as `--target` and `--target-dir` do. The build is started without them.
const MOVING: [&str; 2] = ["CARGO_BUILD_TARGET", "CARGO_BUILD_TARGET_DIR"];

/// The file a run writes into its home. The sweep removes only a folder that holds it.
const MARKER: &str = "jellymoon-smoke-home";

/// `Err` with one line for the first build argument that is not on the list.
pub fn taken(args: &[String]) -> Result<(), String> {
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let valued = VALUED.iter().find(|flag| arg.starts_with(**flag) && !ALONE.contains(&arg.as_str()));
        let value = match valued.map(|flag| &arg[flag.len()..]) {
            _ if ALONE.contains(&arg.as_str()) => continue,
            Some("") => args.next().map(String::as_str),
            Some(joined) if valued.is_some_and(|flag| flag.starts_with("--")) => joined.strip_prefix('='),
            Some(joined) => Some(joined),
            None => None,
        };
        if !value.is_some_and(|value| !value.is_empty() && !value.starts_with('-')) {
            return Err(format!(
                "`{arg}` is not taken after `--`: the chain runs the programs this build leaves in the target \
                 folder, so the build takes only {} and, with a value, {}",
                ALONE.join(" "),
                VALUED.join(" ")
            ));
        }
    }
    Ok(())
}

/// The build of the two programs, with the arguments [`taken`] passed.
pub fn command(root: &Path, args: &[String]) -> Command {
    let mut build = Command::new(env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo")));
    build.current_dir(root).args(["build", "-p", "jellymoon", "-p", "jellymoon-app"]).args(args);
    for name in MOVING {
        build.env_remove(name);
    }
    build
}

/// The folder the build writes to. The build runs in `root`, and cargo reads a relative `CARGO_TARGET_DIR`
/// against the folder it runs in: so is it read here, wherever `cargo xtask` was started.
pub fn target(root: &Path, named: Option<OsString>) -> Result<PathBuf, String> {
    let target = named.map_or_else(|| root.join("target"), |named| root.join(named));
    path::absolute(target).map_err(|e| format!("build: the target folder: {e}"))
}

/// A run's temporary home, named for its process and for when it started: `smoke-home-<pid>-<nanoseconds>`. A
/// later run never takes the name of a home an earlier one left, even when the system gives it the same process
/// id. So a name is one run's for good, and whoever takes the lock beside it holds a home no living run has.
pub fn home_name() -> String {
    let started = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_nanos());
    format!("smoke-home-{}-{started}", process::id())
}

/// Makes a run's home, empty but for the marker the sweep knows it by.
pub fn make_home(home: &Path) -> Result<(), String> {
    let _ = fs::remove_dir_all(home);
    let made = fs::create_dir_all(home).and_then(|()| fs::write(home.join(MARKER), ""));
    made.map_err(|e| format!("home: {e}"))
}

/// The system's lock on the file beside a temporary home, `<home>.lock`, which a run holds for as long as it lives.
/// None while another process holds it.
pub fn lock_beside(home: &Path) -> Option<File> {
    let lock = OpenOptions::new().write(true).create(true).truncate(false).open(home.with_extension("lock")).ok()?;
    lock.try_lock().ok().map(|()| lock)
}

/// Removes the temporary homes earlier runs left in the target folder: a run that was stopped never removed its
/// own, and with `--real` it holds the person's page. A home whose run still lives is left: that run holds its lock.
/// A folder with a home's name and no marker in it is somebody else's, and is left.
pub fn sweep(target: &Path) {
    let left = fs::read_dir(target).into_iter().flatten().flatten().map(|entry| entry.path());
    let named = |path: &PathBuf| path.file_name().is_some_and(|name| name.to_string_lossy().starts_with("smoke-home-"));
    for home in left.filter(|path| named(path) && path.extension().is_none() && path.join(MARKER).is_file()) {
        if let Some(held) = lock_beside(&home) {
            let _ = fs::remove_dir_all(&home);
            drop(held);
            let _ = fs::remove_file(home.with_extension("lock"));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::{MARKER, command, home_name, lock_beside, make_home, sweep, taken, target};

    fn args(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| (*arg).to_string()).collect()
    }

    // With `--help` cargo prints its help, builds nothing and exits 0: the chain then ran what an earlier build
    // left. No list of refused arguments held, so the build takes arguments from a list of its own.
    #[test]
    fn a_build_argument_is_taken_from_a_list_and_any_other_is_refused_in_one_line() {
        let help = ["--help", "-h", "-vh", "--version", "-V", "-Z", "--unit-graph", "--build-plan", "--message-format"];
        let elsewhere = ["--release", "-r", "-vr", "--target", "--target-dir", "--profile=dev", "--config", "-C"];
        let fewer = ["--bin", "--bins", "--lib", "--tests", "--exclude", "--manifest-path", "--no-default-features"];
        let other = ["-p", "--package", "--workspace", "--all-features", "--timings", "--keep-going", "xtask", "", "-"];
        let halves = ["--jobs", "-j", "--features", "-F", "--features=", "--jobs=", "--jobsx", "--lockedx"];
        for flag in help.into_iter().chain(elsewhere).chain(fewer).chain(other).chain(halves) {
            let Err(why) = taken(&args(&["--locked", "--features", "x/y", flag])) else {
                panic!("{flag} was taken");
            };
            assert!(why.starts_with(&format!("`{flag}` is not taken after `--`: ")) && !why.contains('\n'), "{why}");
        }
        for value in ["--help", "-h", "--release"] {
            assert!(taken(&args(&["--features", value])).is_err(), "a value is not a way round: {value}");
            assert!(taken(&args(&["-j", value])).is_err(), "{value}");
        }
        let all = ["--locked", "--offline", "--frozen", "-v", "-vv", "--verbose", "-q", "--quiet", "--jobs", "4"];
        let valued = ["--jobs=4", "-j", "4", "-j4", "--features", "release", "--features=a/b,c", "-F", "x/y", "-Fx/y"];
        assert_eq!(taken(&args(&[&all[..], &valued].concat())), Ok(()));
        assert_eq!(taken(&[]), Ok(()));
    }

    #[test]
    fn the_build_is_started_without_the_variables_that_would_move_it() {
        let build = command(Path::new("."), &args(&["--locked"]));
        let removed = |name: &str| build.get_envs().any(|(var, value)| var == name && value.is_none());
        assert!(removed("CARGO_BUILD_TARGET") && removed("CARGO_BUILD_TARGET_DIR"));
        let args: Vec<_> = build.get_args().map(|arg| arg.to_string_lossy().into_owned()).collect();
        assert_eq!(args, ["build", "-p", "jellymoon", "-p", "jellymoon-app", "--locked"]);
    }

    // The build runs in the root, and cargo reads a relative target folder against where it runs.
    #[test]
    fn a_relative_target_folder_is_read_against_the_root_as_the_build_reads_it() {
        let root = tempfile::tempdir().unwrap();
        let at = |named: Option<&str>| target(root.path(), named.map(Into::into)).unwrap();
        assert_eq!(at(None), root.path().join("target"));
        assert_eq!(at(Some("t")), root.path().join("t"));
        let elsewhere = tempfile::tempdir().unwrap();
        assert_eq!(at(elsewhere.path().to_str()), elsewhere.path(), "one that is absolute is taken as it is");
    }

    // A stale home and a living one never share a name, so whoever holds the lock beside a name may remove it.
    #[test]
    fn a_home_is_named_for_its_process_and_its_start_and_no_two_runs_share_a_name() {
        let (first, second) = (home_name(), home_name());
        let pid = format!("smoke-home-{}-", std::process::id());
        assert!(first.starts_with(&pid) && second.starts_with(&pid) && first != second, "{first} {second}");
        assert!(!first.contains('.'), "the sweep takes a name with no extension for a home");
    }

    #[test]
    fn the_homes_earlier_runs_left_are_removed_and_the_home_of_a_run_that_lives_is_not() {
        let target = tempfile::tempdir().unwrap();
        let at = |name: &str| target.path().join(name);
        for home in ["smoke-home-1", "smoke-home-2", "smoke-home-3"] {
            make_home(&at(home)).unwrap();
            fs::create_dir_all(at(home).join("jm")).unwrap();
            fs::write(at(home).join("jm").join("page.json"), "{}").unwrap();
        }
        // A folder with a home's name that no run made, and one whose marker is a folder: neither is a home.
        for other in ["debug", "smoke-home-4"] {
            fs::create_dir_all(at(other).join("jm")).unwrap();
        }
        fs::create_dir_all(at("smoke-home-5").join(MARKER)).unwrap();
        fs::write(at("smoke-home-1.lock"), "").unwrap();
        fs::write(at("smoke-home-9"), "a file").unwrap();
        let living = lock_beside(&at("smoke-home-2")).unwrap();
        sweep(target.path());
        let mut left: Vec<String> = fs::read_dir(target.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        left.sort();
        let kept = ["debug", "smoke-home-2", "smoke-home-2.lock", "smoke-home-4", "smoke-home-5", "smoke-home-9"];
        assert_eq!(left, kept);
        drop(living);
        sweep(target.path());
        assert!(!at("smoke-home-2").exists() && !at("smoke-home-2.lock").exists() && at("smoke-home-4").is_dir());
    }
}
