//! The build the chain starts and the temporary homes it works in. The chain runs the programs of the debug build
//! in the target folder, so the build takes only arguments from a list of ones that leave it there and build all of
//! it. A list of refused ones was tried first, and twice an argument nobody had thought of passed it: `--help`
//! made cargo print its help and build nothing, and the chain then passed over programs of an earlier build.

use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

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

/// Cargo, started in the root as the build is: without the variables that move a build.
fn cargo(root: &Path) -> Command {
    let mut cargo = Command::new(env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo")));
    cargo.current_dir(root);
    for name in MOVING {
        cargo.env_remove(name);
    }
    cargo
}

/// The build of the two programs, with the arguments [`taken`] passed.
pub fn command(root: &Path, args: &[String]) -> Command {
    let mut build = cargo(root);
    build.args(["build", "-p", "jellymoon", "-p", "jellymoon-app"]).args(args);
    build
}

/// The folder the build writes to, as cargo itself says it: `cargo metadata`, started where the build is and as it
/// is. A `CARGO_TARGET_DIR`, relative or not, and a `build.target-dir` in the cargo home's `config.toml` or in a
/// `.cargo/config.toml` above the repo each move the build, and each is in cargo's answer. `set` is for the tests:
/// a name set to nothing is taken out, so a test is not moved by the variables of the build that runs it.
pub fn target(root: &Path, set: &[(&str, &OsStr)]) -> Result<PathBuf, String> {
    let mut asked = cargo(root);
    asked.args(["metadata", "--format-version", "1", "--no-deps"]);
    for (name, value) in set {
        let _ = if value.is_empty() { asked.env_remove(name) } else { asked.env(name, value) };
    }
    let said = asked.output().map_err(|e| format!("build: cargo metadata: {e}"))?;
    let read: Option<serde_json::Value> = serde_json::from_slice(&said.stdout).ok();
    let target = read.as_ref().and_then(|read| read["target_directory"].as_str());
    target.map(PathBuf::from).ok_or_else(|| {
        let why = String::from_utf8_lossy(&said.stderr);
        format!("build: cargo metadata names no target folder: {}", why.lines().next().unwrap_or_default())
    })
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
    use std::ffi::OsStr;
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
        // With its value too, what is not on the list is refused: the chain names its own packages.
        for pair in [["-p", "xtask"], ["--package", "xtask"], ["--target-dir", "t"], ["--profile", "dev"]] {
            assert!(taken(&args(&pair)).is_err(), "{pair:?}");
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

    // Whatever moves the build is in cargo's own answer: a variable, relative or not, the cargo home's
    // `config.toml`, and a `.cargo/config.toml` above the repo. A made-up package stands in for the repo.
    #[test]
    fn the_target_folder_is_the_one_cargo_names_whatever_moved_it() {
        let above = tempfile::tempdir().unwrap();
        let above = above.path().canonicalize().unwrap();
        let (root, cargo_home, moved) = (above.join("repo"), above.join("cargo-home"), above.join("moved"));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src").join("lib.rs"), "").unwrap();
        fs::write(root.join("Cargo.toml"), "[package]\nname = \"made-up\"\nversion = \"0.0.0\"\nedition = \"2021\"\n")
            .unwrap();
        fs::create_dir_all(&cargo_home).unwrap();
        let home = [("CARGO_HOME", cargo_home.as_os_str()), ("CARGO_TARGET_DIR", OsStr::new(""))];
        let at = |set: &[(&str, &OsStr)]| target(&root, &[&home[..], set].concat());
        assert_eq!(at(&[]), Ok(root.join("target")));
        assert_eq!(at(&[("CARGO_TARGET_DIR", OsStr::new("t"))]), Ok(root.join("t")), "relative: against the root");
        assert_eq!(at(&[("CARGO_TARGET_DIR", moved.as_os_str())]), Ok(moved.clone()));
        let config = format!("[build]\ntarget-dir = {:?}\n", moved.to_str().unwrap());
        fs::write(cargo_home.join("config.toml"), &config).unwrap();
        assert_eq!(at(&[]), Ok(moved.clone()), "the cargo home's own config moves the build");
        fs::remove_file(cargo_home.join("config.toml")).unwrap();
        fs::create_dir_all(above.join(".cargo")).unwrap();
        fs::write(above.join(".cargo").join("config.toml"), &config).unwrap();
        assert_eq!(at(&[]), Ok(moved), "and so does a config above the repo");
        let nowhere = target(&above.join("no-repo"), &[]).unwrap_err();
        assert!(nowhere.starts_with("build: cargo metadata") && !nowhere.contains('\n'), "{nowhere}");
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
