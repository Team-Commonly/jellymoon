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
                           folder, JELLYMOON_APP_DIR, JELLYMOON_BIN_DIR, CLAUDE_CONFIG_DIR, the temporary and config \
                           folders a world names, git's own config file, the login shell, a harness named with \
                           JELLYMOON_CLAUDE, --logs, --settings and --repo are each under the system's temporary \
                           folder (in the binary's tests /tmp on Unix too) or the one Cargo gives tests, none of \
                           those variables and no --logs climbs out of the folder it is read from, and no folder of \
                           PATH or JELLYMOON_SYSTEM_BIN outside them holds a claude";

/// What the door says when a temporary folder cannot be taken for one. It names no path.
pub const NO_ROOT: &str = "a test starts nothing: a temporary folder is a file system's root, or it is or holds the \
                           home of the person running the tests, or that home's .claude, .jellymoon or .local";

/// The variables that name a folder of the person's: the home on each system, Jellymoon's folder, the two an
/// install goes to and `jellymoon uninstall` removes under, the harness's, and the temporary and config folders
/// the programs hand on to a harness, to git and to the installer.
const FOLDERS: [&str; 12] = [
    "HOME",
    "USERPROFILE",
    "JELLYMOON_HOME",
    "JELLYMOON_APP_DIR",
    "JELLYMOON_BIN_DIR",
    "CLAUDE_CONFIG_DIR",
    "TMPDIR",
    "TEMP",
    "TMP",
    "XDG_CONFIG_HOME",
    "APPDATA",
    "LOCALAPPDATA",
];
/// The variable that names a file of the person's: its folder is held too.
const FILE: &str = "GIT_CONFIG_GLOBAL";
/// The variable that names the program to start as the harness: a made-up one, planted under a temporary folder.
const HARNESS: &str = "JELLYMOON_CLAUDE";
const SHELL: &str = "JELLYMOON_SHELL";
/// The folders of the machine a harness is looked for in: none, unless a test names its own.
const SYSTEM_BIN: &str = "JELLYMOON_SYSTEM_BIN";
/// The variables that list folders a harness is looked for in, written as `PATH` is.
const LISTS: [&str; 2] = ["PATH", SYSTEM_BIN];
/// The harness's file names, on any system.
const NAMES: [&str; 3] = ["claude", "claude.exe", "claude.cmd"];
/// What is the person's own under their home, whatever else a temporary folder there may hold.
const OWN: [&str; 3] = [".claude", ".jellymoon", ".local"];
/// The flags whose value names a folder or a file of the person's.
const FLAGS: [&str; 3] = ["--logs", "--settings", "--repo"];
/// The flag whose value is read as given when it is no full path, from the folder the program is started in, which
/// the door never sees; `jellymoon` refuses the two others then. A variable is held to the same rule whatever a
/// program does with it: the installer takes its folders as given.
const AS_GIVEN: &str = "--logs";
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
    let mut command = Command::new(program);
    command.args(args).env_clear().env(SHELL, "").env(SYSTEM_BIN, "").envs(vars);
    if let Some(root) = env::var_os("SystemRoot").filter(|_| cfg!(windows)) {
        command.env("SystemRoot", root); // a Windows program needs it to start
    }
    command
}

/// Whether one of these temporary folders cannot be taken for one: it is a file system's root, or it is or holds
/// one of these homes, which are the process's own, or what is the person's own under one. `TMPDIR` may name
/// anything else, and on Windows the temporary folder is the home itself when `TMP` and `TEMP` are not set.
pub fn no_root(roots: &[PathBuf], homes: &[PathBuf]) -> bool {
    let homes = homes.iter().filter(|home| home.is_absolute());
    let kept = homes.flat_map(|home| OWN.iter().map(move |own| home.join(own)).chain([home.clone()]));
    let kept: Vec<PathBuf> = kept.filter_map(|kept| resolved(&kept, LINKS)).collect();
    let held = |root: &PathBuf| root.parent().is_none() || kept.iter().any(|kept| kept.starts_with(root));
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
/// reads a world, a variable's name is the same name in any case, a path may be rooted with no drive, and one with
/// a drive and no root is read from that drive's own folder.
pub fn refusal_as<K: AsRef<OsStr>, V: AsRef<OsStr>, A: AsRef<OsStr>>(
    windows: bool,
    roots: &[PathBuf],
    vars: &[(K, V)],
    args: &[A],
) -> Option<String> {
    let roots: Vec<PathBuf> = roots.iter().filter_map(|root| resolved(root, LINKS)).collect();
    let outside = |value: &OsStr| outside(&roots, Path::new(value));
    let held = |value: &OsStr| outside(value) || leaves(windows, value);
    let is = |name: &OsStr, known: &str| if windows { name.eq_ignore_ascii_case(known) } else { name == known };
    for (name, value) in vars.iter().map(|(name, value)| (name.as_ref(), value.as_ref())) {
        let known = |names: &[&str]| names.iter().any(|known| is(name, known));
        let path = (known(&FOLDERS) || known(&[FILE, HARNESS])) && held(value);
        let file = is(name, FILE) && Path::new(value).parent().is_some_and(|folder| outside(folder.as_os_str()));
        // A login shell is a made-up program under a temporary folder, or none at all.
        let shell = is(name, SHELL) && !value.is_empty() && (outside(value) || !Path::new(value).is_absolute());
        if path || file || shell || known(&LISTS) && reaches(&roots, value, harness_in) {
            return Some(name.to_string_lossy().into_owned());
        }
    }
    let args: Vec<String> = args.iter().map(|arg| arg.as_ref().to_string_lossy().into_owned()).collect();
    for (at, arg) in args.iter().enumerate() {
        for flag in FLAGS {
            let joined = arg.strip_prefix(flag).and_then(|rest| rest.strip_prefix('='));
            let value = joined.or_else(|| args.get(at + 1).filter(|_| arg.as_str() == flag).map(String::as_str));
            let climbs = |value: &str| flag == AS_GIVEN && leaves(windows, OsStr::new(value));
            if value.is_some_and(|value| outside(OsStr::new(value)) || climbs(value)) {
                return Some(flag.to_string());
            }
        }
    }
    None
}

/// Whether a full path is under none of these temporary folders, which are resolved already.
fn outside(roots: &[PathBuf], path: &Path) -> bool {
    path.is_absolute() && !resolved(path, LINKS).is_some_and(|path| roots.iter().any(|root| path.starts_with(root)))
}

/// Whether a list of folders, written as `PATH` is, reaches a harness of the person's: one of its full-path
/// folders under none of these temporary folders holds one, as `holds` answers. A folder that is no full path is
/// not passed on by `jellymoon`, and one under a temporary folder holds only what a test planted.
pub fn reaches(roots: &[PathBuf], folders: &OsStr, holds: impl Fn(&Path) -> bool) -> bool {
    let roots: Vec<PathBuf> = roots.iter().filter_map(|root| resolved(root, LINKS)).collect();
    env::split_paths(folders).any(|folder| outside(&roots, &folder) && holds(&folder))
}

/// Whether a folder holds the harness by one of its names. The system is asked for the name alone: nothing is
/// started, and nothing of the file is read.
fn harness_in(folder: &Path) -> bool {
    NAMES.iter().any(|name| fs::symlink_metadata(folder.join(name)).is_ok())
}

/// Whether a value that is no full path leaves the folder it is read from: it climbs with `..`, or, as Windows
/// reads it, it is rooted with no drive (`\Users`) or has a drive and no root (`D:logs`).
fn leaves(windows: bool, value: &OsStr) -> bool {
    let (path, text) = (Path::new(value), value.to_string_lossy());
    let either = windows && text.split(['/', '\\']).any(|part| part == "..");
    let up = either || path.components().any(|part| part == Component::ParentDir);
    let rooted = path.has_root() || windows && text.starts_with(['/', '\\']);
    let mut letters = text.chars().skip(1);
    let drive = windows && letters.next() == Some(':') && !matches!(letters.next(), Some('/' | '\\'));
    !path.is_absolute() && (up || rooted || drive)
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
