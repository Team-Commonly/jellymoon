//! The one door a test of the app starts `jellymoon-app` through, or `jellymoon`, or a program that starts either
//! (ADR 0021, decision 7). The rule is the binary's own, one file taken as a module here
//! (`crates/jellymoon/tests/support/door/world.rs`): a world whose home, Jellymoon's folder, `CLAUDE_CONFIG_DIR`
//! or `--logs` is a full path outside the temporary folders is refused before anything is started, and every world
//! that is started has `JELLYMOON_SHELL` set to nothing. The temporary folders are the system's and the one Cargo
//! gives tests, and no other: no test of the app needs a home that is someone else's. `cargo xtask check` refuses a
//! test file that names a program of this door anywhere else (`crates/xtask/src/rules/door.rs`).

use std::ffi::OsStr;
use std::process::Command;

#[path = "../../../crates/jellymoon/tests/support/door/world.rs"]
mod world;

#[allow(unused_imports)] // each file of tests takes what it needs
pub use world::{NO_ROOT, REFUSED, no_root, refusal, refusal_as, roots};

/// A world with no variable at all, as `jellymoon-app layout` is started on a page file.
pub const BARE: [(&str, &str); 0] = [];

/// `jellymoon-app` with these arguments and only these variables, in a world the door has passed. Not started yet.
pub fn app<K: AsRef<OsStr>, V: AsRef<OsStr>, A: AsRef<OsStr>>(
    vars: impl IntoIterator<Item = (K, V)>,
    args: &[A],
) -> Command {
    command(env!("CARGO_BIN_EXE_jellymoon-app"), vars, args)
}

/// The built `jellymoon` command, which the app's tests build under a name of their own: for a test that copies
/// it to where `install.sh` puts it. To start it there is [`jellymoon`].
pub fn built_jellymoon() -> &'static str {
    env!("CARGO_BIN_EXE_f42-jellymoon")
}

/// The `jellymoon` command, the same way.
pub fn jellymoon<K: AsRef<OsStr>, V: AsRef<OsStr>, A: AsRef<OsStr>>(
    vars: impl IntoIterator<Item = (K, V)>,
    args: &[A],
) -> Command {
    command(built_jellymoon(), vars, args)
}

/// A made-up window, the same way: `wisher.rs`, or `tapper.rs` when `taps` is true. Each is the app's own code in
/// a process of its own, and core starts `jellymoon` or git from it.
pub fn window<K: AsRef<OsStr>, V: AsRef<OsStr>, A: AsRef<OsStr>>(
    taps: bool,
    vars: impl IntoIterator<Item = (K, V)>,
    args: &[A],
) -> Command {
    let built = if taps { env!("CARGO_BIN_EXE_f47-tapper") } else { env!("CARGO_BIN_EXE_f42-wisher") };
    command(built, vars, args)
}

/// Any other program a test starts in a world (a made-up harness, git, a script), the same way. A world
/// that is refused panics, as the support's other failures do, and no program is started.
pub fn command<K: AsRef<OsStr>, V: AsRef<OsStr>, A: AsRef<OsStr>>(
    program: impl AsRef<OsStr>,
    vars: impl IntoIterator<Item = (K, V)>,
    args: &[A],
) -> Command {
    world::command(&roots(), program, vars, args)
}
