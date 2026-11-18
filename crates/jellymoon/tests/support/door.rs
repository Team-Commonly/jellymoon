//! The one door a test starts `jellymoon` through, or a program that starts it (ADR 0021, decision 7). A world whose
//! home, Jellymoon's folder, `CLAUDE_CONFIG_DIR`, login shell, `--logs`, `--settings` or `--repo` is a full path
//! outside the temporary folders is refused before anything is started, and every world that is started has
//! `JELLYMOON_SHELL` set to nothing unless the test named a made-up login shell under a temporary folder. The rule
//! itself is `door/world.rs`, which the app's door takes too. `cargo xtask check` refuses a test file that names
//! the built program anywhere else (`crates/xtask/src/rules/door.rs`).

use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Command;

mod world;

#[allow(unused_imports)] // each file of tests takes what it needs
pub use world::{NO_ROOT, REFUSED, UNGUARDED, no_root, refusal, refusal_as, unguarded_installer};

/// The built `jellymoon`: for a test that copies it, compares a path with it or names it to a made-up program. To
/// start it there is [`jellymoon`].
pub fn built() -> &'static str {
    env!("CARGO_BIN_EXE_jellymoon")
}

/// The temporary folders: the system's, and the one Cargo gives tests. The system's is the one the process is
/// given and, where there is one, `/tmp`: the one folder a test can name that is someone else's ([`not_ours`]).
pub fn roots() -> Vec<PathBuf> {
    world::roots().into_iter().chain(cfg!(unix).then_some(PathBuf::from("/tmp"))).collect()
}

/// A folder the door passes that is not the tests' own, to stand as the home of another owner: core takes the
/// person to be the owner of their home. It was the root folder, which is no temporary folder. None where the
/// system has no owners, or every temporary folder is the tests' own (they run as root). It is a folder anyone can
/// write to, so a world that takes it as the home names Jellymoon's folder, the harness's folder and a harness of
/// its own: then nothing is written under it, and no harness is looked for in it.
pub fn not_ours() -> Option<PathBuf> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let owner = |folder: &std::path::Path| std::fs::metadata(folder).ok().map(|folder| folder.uid());
        let ours = owner(std::path::Path::new(env!("CARGO_TARGET_TMPDIR")));
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
/// that is refused panics, as the support's other failures do, and no program is started.
pub fn command<K: AsRef<OsStr>, V: AsRef<OsStr>, A: AsRef<OsStr>>(
    program: impl AsRef<OsStr>,
    vars: impl IntoIterator<Item = (K, V)>,
    args: &[A],
) -> Command {
    world::command(&roots(), program, vars, args)
}
