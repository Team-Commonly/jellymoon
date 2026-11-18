//! What a door refuses of a world, and the command it makes for one it has passed (ADR 0021, decision 7): the one
//! rule of the two doors. The binary's tests and the app's are different crates, so each door (`../door.rs`, and
//! `app/tests/support/door.rs`) takes this file as a module of its own, and adds the programs it starts and its
//! temporary folders.

use std::ffi::{OsStr, OsString};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::{env, fs};

/// What a refusal says, whatever was refused. After it comes the name of the variable or the flag, never a path.
pub const REFUSED: &str = "a test starts nothing in a world outside the temporary folders: the home, Jellymoon's \
                           folder, JELLYMOON_APP_DIR, JELLYMOON_BIN_DIR, CLAUDE_CONFIG_DIR, the login shell, --logs, \
                           --settings and --repo are each under the system's temporary folder (in the binary's \
                           tests /tmp on Unix too) or the one Cargo gives tests, and no --logs or CLAUDE_CONFIG_DIR \
                           climbs out of the folder it is read from";

/// What the door says when a temporary folder cannot be taken for one. It names no path.
pub const NO_ROOT: &str = "a test starts nothing: a temporary folder is a file system's root, or it is or holds the \
                           home of the person running the tests";

/// What the door says of a world that runs the installer and could open the real app. It names no path.
pub const UNGUARDED: &str = "a test runs the installer only with JELLYMOON_NO_OPEN set to something, or with a \
                             made-up `open` in the first folder of PATH, under a temporary folder: the real app is \
                             never opened";

/// The variables that name a folder of the person's: the home on each system, Jellymoon's folder, the two an
/// install goes to and `jellymoon uninstall` removes under, and the harness's.
const FOLDERS: [&str; 6] =
    ["HOME", "USERPROFILE", "JELLYMOON_HOME", "JELLYMOON_APP_DIR", "JELLYMOON_BIN_DIR", "CLAUDE_CONFIG_DIR"];
const SHELL: &str = "JELLYMOON_SHELL";
/// The folders of the machine a harness is looked for in: none, unless a test names its own.
const SYSTEM_BIN: &str = "JELLYMOON_SYSTEM_BIN";
/// The flags whose value names a folder or a file of the person's.
const FLAGS: [&str; 3] = ["--logs", "--settings", "--repo"];
/// The variable and the flag whose value is read as given when it is no full path, from the folder the program is
/// started in, which the door never sees. Every other one is then ignored or refused by the program itself.
const AS_GIVEN: [&str; 2] = ["CLAUDE_CONFIG_DIR", "--logs"];
/// How many links whose end is not there are read on the way to one path. One more, and the path is refused.
const LINKS: u8 = 32;

/// The temporary folders of both doors: the one the system gives the process, and the one Cargo gives tests.
pub fn roots() -> Vec<PathBuf> {
    vec![env::temp_dir(), PathBuf::from(env!("CARGO_TARGET_TMPDIR"))]
}

/// A program with these arguments and only these variables, in a world passed with these temporary folders. Not
/// started yet. `JELLYMOON_SHELL` and `JELLYMOON_SYSTEM_BIN` are set to nothing unless the world names its own. A
/// world that is refused panics here, as the support's other failures do, and no program is started.
pub fn command<K: AsRef<OsStr>, V: AsRef<OsStr>, A: AsRef<OsStr>>(
    roots: &[PathBuf],
    program: impl AsRef<OsStr>,
    vars: impl IntoIterator<Item = (K, V)>,
    args: &[A],
) -> Command {
    let vars: Vec<(OsString, OsString)> =
        vars.into_iter().map(|(name, value)| (name.as_ref().to_os_string(), value.as_ref().to_os_string())).collect();
    let own: Vec<PathBuf> = ["HOME", "USERPROFILE"].iter().filter_map(env::var_os).map(PathBuf::from).collect();
    assert!(!no_root(roots, &own), "{NO_ROOT}");
    if let Some(what) = refusal(roots, vars.as_slice(), args) {
        panic!("{REFUSED}; this one named {what}");
    }
    assert!(!unguarded_installer(roots, vars.as_slice(), args), "{UNGUARDED}");
    let mut command = Command::new(program);
    command.args(args).env_clear().env(SHELL, "").env(SYSTEM_BIN, "").envs(vars);
    if let Some(root) = env::var_os("SystemRoot").filter(|_| cfg!(windows)) {
        command.env("SystemRoot", root); // a Windows program needs it to start
    }
    command
}

/// Whether one of these temporary folders cannot be taken for one: it is a file system's root, or it is or holds
/// one of these homes, which are the process's own. `TMPDIR` may name anything, and on Windows the temporary
/// folder is the home itself when `TMP` and `TEMP` are not set.
pub fn no_root(roots: &[PathBuf], homes: &[PathBuf]) -> bool {
    let homes: Vec<PathBuf> =
        homes.iter().filter(|home| home.is_absolute()).filter_map(|home| resolved(home, LINKS)).collect();
    let held = |root: &PathBuf| root.parent().is_none() || homes.iter().any(|home| home.starts_with(root));
    roots.iter().filter_map(|root| resolved(root, LINKS)).any(|root| held(&root))
}

/// The first variable or flag of a world that the door refuses with these temporary folders, by its name as the
/// world wrote it.
pub fn refusal<K: AsRef<OsStr>, V: AsRef<OsStr>, A: AsRef<OsStr>>(
    roots: &[PathBuf],
    vars: &[(K, V)],
    args: &[A],
) -> Option<String> {
    refusal_as(cfg!(windows), roots, vars, args)
}

/// [`refusal`], with the system given as a value, so that a test on any system holds both readings. As Windows
/// reads a world, a variable's name is the same name in any case, and a path may be rooted with no drive.
pub fn refusal_as<K: AsRef<OsStr>, V: AsRef<OsStr>, A: AsRef<OsStr>>(
    windows: bool,
    roots: &[PathBuf],
    vars: &[(K, V)],
    args: &[A],
) -> Option<String> {
    let roots: Vec<PathBuf> = roots.iter().filter_map(|root| resolved(root, LINKS)).collect();
    let under = |path: PathBuf| roots.iter().any(|root| path.starts_with(root));
    let outside =
        |value: &OsStr| Path::new(value).is_absolute() && !resolved(Path::new(value), LINKS).is_some_and(under);
    let is = |name: &OsStr, known: &str| if windows { name.eq_ignore_ascii_case(known) } else { name == known };
    for (name, value) in vars.iter().map(|(name, value)| (name.as_ref(), value.as_ref())) {
        let folder = FOLDERS.iter().any(|known| is(name, known)) && outside(value);
        let climbs = is(name, AS_GIVEN[0]) && leaves(windows, value);
        // A login shell is a made-up program under a temporary folder, or none at all.
        let shell = is(name, SHELL) && !value.is_empty() && (outside(value) || !Path::new(value).is_absolute());
        if folder || climbs || shell {
            return Some(name.to_string_lossy().into_owned());
        }
    }
    let args: Vec<String> = args.iter().map(|arg| arg.as_ref().to_string_lossy().into_owned()).collect();
    for (at, arg) in args.iter().enumerate() {
        for flag in FLAGS {
            let joined = arg.strip_prefix(flag).and_then(|rest| rest.strip_prefix('='));
            let value = joined.or_else(|| args.get(at + 1).filter(|_| arg.as_str() == flag).map(String::as_str));
            let climbs = |value: &str| flag == AS_GIVEN[1] && leaves(windows, OsStr::new(value));
            if value.is_some_and(|value| outside(OsStr::new(value)) || climbs(value)) {
                return Some(flag.to_string());
            }
        }
    }
    None
}

/// Whether a world runs the installer (it names `JELLYMOON_RELEASE_URL`, or an argument is `install.sh`) and is
/// guarded against opening the real app by neither of the two ways: `JELLYMOON_NO_OPEN` set to something, or a
/// made-up `open` in the first folder of `PATH`, which is under a temporary folder.
pub fn unguarded_installer<K: AsRef<OsStr>, V: AsRef<OsStr>, A: AsRef<OsStr>>(
    roots: &[PathBuf],
    vars: &[(K, V)],
    args: &[A],
) -> bool {
    let value = |known: &str| vars.iter().find(|(name, _)| name.as_ref() == known).map(|(_, value)| value.as_ref());
    let script = args.iter().any(|arg| Path::new(arg.as_ref()).file_name() == Some(OsStr::new("install.sh")));
    if !script && value("JELLYMOON_RELEASE_URL").is_none() {
        return false;
    }
    if value("JELLYMOON_NO_OPEN").is_some_and(|set| !set.is_empty()) {
        return false;
    }
    let first = value("PATH").and_then(|path| env::split_paths(path).next()).filter(|first| first.is_absolute());
    let roots: Vec<PathBuf> = roots.iter().filter_map(|root| resolved(root, LINKS)).collect();
    let under = |first: PathBuf| roots.iter().any(|root| first.starts_with(root));
    let made_up = first.is_some_and(|first| first.join("open").is_file() && resolved(&first, LINKS).is_some_and(under));
    !made_up
}

/// Whether a value that is no full path leaves the folder it is read from: it climbs with `..`, or it is rooted
/// with no drive, as Windows reads `\Users`.
fn leaves(windows: bool, value: &OsStr) -> bool {
    let (path, text) = (Path::new(value), value.to_string_lossy());
    let either = windows && text.split(['/', '\\']).any(|part| part == "..");
    let up = either || path.components().any(|part| part == Component::ParentDir);
    let rooted = path.has_root() || windows && text.starts_with(['/', '\\']);
    !path.is_absolute() && (up || rooted)
}

/// A path with its links resolved as far as it exists: the longest start of it that is there, as the system
/// resolves it, then the rest as written, with `.` and `..` worked out. A link whose end is not there is read, and
/// the path it names resolved in turn. None when more links are in the way than are read: such a path is under no
/// folder. The path is taken part by part first: one that ends in a separator is not read as a link by the system.
fn resolved(path: &Path, links: u8) -> Option<PathBuf> {
    let path: PathBuf = path.components().collect();
    for above in path.ancestors() {
        let rest = path.strip_prefix(above).unwrap_or(Path::new(""));
        if let Ok(real) = fs::canonicalize(above) {
            return Some(plain(real, rest));
        }
        if let Ok(to) = fs::read_link(above) {
            let named = above.parent().unwrap_or(above).join(to);
            return resolved(&named.join(rest), links.checked_sub(1)?);
        }
    }
    Some(plain(PathBuf::new(), &path))
}

/// `rest` after `real`, where nothing of `rest` is there: no link is in it, so `..` is the folder above.
fn plain(mut real: PathBuf, rest: &Path) -> PathBuf {
    for part in rest.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                real.pop();
            }
            other => real.push(other),
        }
    }
    real
}
