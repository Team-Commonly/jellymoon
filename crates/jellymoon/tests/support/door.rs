//! The one door a test starts `jellymoon` through, or a program that starts it (ADR 0021, decision 7). A world whose
//! home, Jellymoon's folder, `CLAUDE_CONFIG_DIR`, login shell, `--logs`, `--settings` or `--repo` is a full path
//! outside the temporary folders is refused before anything is started, and every world that is started has
//! `JELLYMOON_SHELL` set to nothing unless the test named a made-up login shell under a temporary folder.
//! `cargo xtask check` refuses a test file that names the built program anywhere else
//! (`crates/xtask/src/rules/door.rs`).

use std::ffi::{OsStr, OsString};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::{env, fs};

/// What a refusal says, whatever was refused. After it comes the name of the variable or the flag, never a path.
pub const REFUSED: &str = "a test starts nothing in a world outside the temporary folders: the home, Jellymoon's \
                           folder, CLAUDE_CONFIG_DIR, the login shell, --logs, --settings and --repo are each under \
                           the system's temporary folder or the one Cargo gives tests";

/// The variables that name a folder of the person's: the home on each system, Jellymoon's folder, the harness's.
const FOLDERS: [&str; 4] = ["HOME", "USERPROFILE", "JELLYMOON_HOME", "CLAUDE_CONFIG_DIR"];
const SHELL: &str = "JELLYMOON_SHELL";
/// The flags whose value names a folder or a file of the person's.
const FLAGS: [&str; 3] = ["--logs", "--settings", "--repo"];
/// How many links whose end is not there are read on the way to one path.
const LINKS: u8 = 32;

/// The built `jellymoon`: for a test that copies it, compares a path with it or names it to a made-up program. To
/// start it there is [`jellymoon`].
pub fn built() -> &'static str {
    env!("CARGO_BIN_EXE_jellymoon")
}

/// The temporary folders: the system's, and the one Cargo gives tests. The system's is the one the process is
/// given and, where there is one, `/tmp`: the one folder a test can name that is someone else's ([`not_ours`]).
pub fn roots() -> Vec<PathBuf> {
    let shared = cfg!(unix).then_some(PathBuf::from("/tmp"));
    [env::temp_dir(), PathBuf::from(env!("CARGO_TARGET_TMPDIR"))].into_iter().chain(shared).collect()
}

/// A folder the door passes that is not the tests' own, to stand as the home of another owner: core takes the
/// person to be the owner of their home. It was the root folder, which is no temporary folder. None where the
/// system has no owners, or every temporary folder is the tests' own (they run as root).
pub fn not_ours() -> Option<PathBuf> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let owner = |folder: &Path| fs::metadata(folder).ok().map(|folder| folder.uid());
        let ours = owner(Path::new(env!("CARGO_TARGET_TMPDIR")));
        roots().into_iter().find(|folder| owner(folder).is_some_and(|theirs| Some(theirs) != ours))
    }
    #[cfg(not(unix))]
    None
}

/// `jellymoon` with these arguments and only these variables, in a world the door has passed. Not started yet.
pub fn jellymoon<K: AsRef<OsStr>, V: AsRef<OsStr>, A: AsRef<OsStr>>(
    vars: impl IntoIterator<Item = (K, V)>,
    args: &[A],
) -> Command {
    command(built(), vars, args)
}

/// Any program that may start `jellymoon` (a hook's entry, the installer, a made-up asker), the same way. A world
/// that is refused panics here, as the support's other failures do, and no program is started.
pub fn command<K: AsRef<OsStr>, V: AsRef<OsStr>, A: AsRef<OsStr>>(
    program: impl AsRef<OsStr>,
    vars: impl IntoIterator<Item = (K, V)>,
    args: &[A],
) -> Command {
    let vars: Vec<(OsString, OsString)> =
        vars.into_iter().map(|(name, value)| (name.as_ref().to_os_string(), value.as_ref().to_os_string())).collect();
    if let Some(what) = refusal(&roots(), vars.as_slice(), args) {
        panic!("{REFUSED}; this one named {what}");
    }
    let mut command = Command::new(program);
    command.args(args).env_clear().env(SHELL, "").envs(vars);
    if let Some(root) = env::var_os("SystemRoot").filter(|_| cfg!(windows)) {
        command.env("SystemRoot", root); // a Windows program needs it to start
    }
    command
}

/// The first variable or flag of a world that the door refuses with these temporary folders, by its name.
pub fn refusal<K: AsRef<OsStr>, V: AsRef<OsStr>, A: AsRef<OsStr>>(
    roots: &[PathBuf],
    vars: &[(K, V)],
    args: &[A],
) -> Option<String> {
    let roots: Vec<PathBuf> = roots.iter().map(|root| resolved(root, LINKS)).collect();
    let outside = |value: &OsStr| {
        let path = Path::new(value);
        path.is_absolute() && !roots.iter().any(|root| resolved(path, LINKS).starts_with(root))
    };
    for (name, value) in vars.iter().map(|(name, value)| (name.as_ref(), value.as_ref())) {
        let folder = FOLDERS.iter().any(|known| name == OsStr::new(known)) && outside(value);
        // A login shell is a made-up program under a temporary folder, or none at all.
        let shell =
            name == OsStr::new(SHELL) && !value.is_empty() && (outside(value) || !Path::new(value).is_absolute());
        if folder || shell {
            return Some(name.to_string_lossy().into_owned());
        }
    }
    let args: Vec<String> = args.iter().map(|arg| arg.as_ref().to_string_lossy().into_owned()).collect();
    for (at, arg) in args.iter().enumerate() {
        for flag in FLAGS {
            let joined = arg.strip_prefix(flag).and_then(|rest| rest.strip_prefix('='));
            let value = joined.or_else(|| args.get(at + 1).filter(|_| arg.as_str() == flag).map(String::as_str));
            if value.is_some_and(|value| outside(OsStr::new(value))) {
                return Some(flag.to_string());
            }
        }
    }
    None
}

/// A path with its links resolved as far as it exists: the longest start of it that is there, as the system
/// resolves it, then the rest as written, with `.` and `..` worked out. A link whose end is not there is read, and
/// the path it names resolved in turn.
fn resolved(path: &Path, links: u8) -> PathBuf {
    for above in path.ancestors() {
        let rest = path.strip_prefix(above).unwrap_or(Path::new(""));
        if let Ok(real) = fs::canonicalize(above) {
            return plain(real, rest);
        }
        if let Some(to) = fs::read_link(above).ok().filter(|_| links > 0) {
            let named = above.parent().unwrap_or(above).join(to);
            return resolved(&named.join(rest), links - 1);
        }
    }
    plain(PathBuf::new(), path)
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
