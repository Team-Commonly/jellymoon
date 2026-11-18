//! Asking the login shell for its `PATH`, on a Mac and on Linux. Which program it is comes from `JELLYMOON_SHELL`,
//! or with that unset from the system's record of the user (`record.rs`); set to nothing, no login shell is asked.
//! It is started only when it is named by a full path, is a file that can be run, nobody but its owner can write
//! to it, not everyone can write to its folder or to the folder its link-resolved file is in, and its last name
//! is on [`LISTED`]. It gets one fixed command, nothing on its input, the variables a harness gets and a process
//! group of its own; it has [`LIMIT`] to answer, at most [`MOST`] bytes of it are read, and the whole group is
//! killed before this returns, answered or not. Of what it printed only what stands between the two marks counts,
//! and only when there is exactly one pair of them, on one line. An answer is remembered for as long as this
//! process lives, in its memory alone; an ask that failed is made again by the next look.

use std::ffi::OsStr;
use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::mpsc::{RecvTimeoutError, channel};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};
use std::{env, fs, thread};

use rustix::process::{Pid, Signal, kill_process_group};

use super::Shell;
use crate::call;

mod record;

/// The login shells that are asked, by the last name of their program. Both take `-l -i -c <command>` and give
/// `printf` and `"$PATH"` the same meaning; a shell that is not one of them is not started.
const LISTED: [&str; 2] = ["zsh", "bash"];
/// The mark that stands before and after the `PATH` in the answer.
const MARK: &[u8] = b"__jellymoon_path__";
/// The one command. The mark is in two pieces in it, so that a shell that prints the command back prints no mark.
const COMMAND: &str = r#"printf '%s%s%s%s%s\n' __jellymoon _path__ "$PATH" __jellymoon _path__"#;
/// How long the login shell has to answer.
const LIMIT: Duration = Duration::from_secs(5);
/// The most bytes read of what it prints: a profile may say a lot before the command runs, and a `PATH` is short.
const MOST: usize = 64 * 1024;

/// The answer of this process's one successful ask.
static ANSWER: Mutex<Option<Vec<PathBuf>>> = Mutex::new(None);

/// The folders the login shell named by a full path, in its order, or how the ask ended with none.
pub(in crate::find) fn folders() -> Result<Vec<PathBuf>, Shell> {
    // Held for the whole ask: two looks at once are one ask.
    let mut answer = ANSWER.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(folders) = answer.as_ref() {
        return Ok(folders.clone());
    }
    let shell = match env::var_os("JELLYMOON_SHELL") {
        Some(named) if named.is_empty() => return Err(Shell::Off),
        Some(named) => PathBuf::from(named),
        None => record::shell().ok_or(Shell::NoRecord)?,
    };
    fit(&shell)?;
    let folders = named(&asked(&shell)?)?;
    *answer = Some(folders.clone());
    Ok(folders)
}

/// Whether a login shell may be started, and how it is refused when not.
fn fit(shell: &Path) -> Result<(), Shell> {
    if !shell.is_absolute() {
        return Err(Shell::NotFull);
    }
    let mode = |meta: fs::Metadata| meta.permissions().mode();
    let program = fs::metadata(shell).ok().filter(fs::Metadata::is_file).map(mode);
    let program = program.filter(|mode| mode & 0o111 != 0).ok_or(Shell::NotAProgram)?;
    let real = fs::canonicalize(shell).map_err(|_| Shell::NotAProgram)?;
    // A folder that cannot be read is taken as one everyone can write to.
    let open = |folder: Option<&Path>| {
        folder.and_then(|folder| fs::metadata(folder).ok()).is_none_or(|meta| mode(meta) & 0o002 != 0)
    };
    if program & 0o022 != 0 || open(shell.parent()) || open(real.parent()) {
        return Err(Shell::Open);
    }
    let listed = shell.file_name().and_then(OsStr::to_str).is_some_and(|name| LISTED.contains(&name));
    if listed { Ok(()) } else { Err(Shell::Unlisted) }
}

/// Starts the login shell with the one command and returns what was read of its answer. It and every program of
/// its group are killed first, whether it answered or not.
fn asked(shell: &Path) -> Result<Vec<u8>, Shell> {
    let mut command = Command::new(shell);
    command.args(["-l", "-i", "-c", COMMAND]).env_clear().envs(call::environment()).current_dir("/");
    command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).process_group(0);
    let mut child = command.spawn().map_err(|_| Shell::NotStarted)?;
    let said = read(child.stdout.take());
    end(&mut child);
    said
}

/// Reads what the login shell prints until both marks are there, it prints no more, [`MOST`] bytes were read, or
/// [`LIMIT`] is over, which alone is an ask with no answer. The pipe is read on a thread of its own, which ends
/// when the pipe does: a program that kept it open and left the group keeps that thread, and nothing else.
fn read(stdout: Option<ChildStdout>) -> Result<Vec<u8>, Shell> {
    let (pieces, read) = channel();
    thread::spawn(move || {
        let (Some(mut stdout), mut piece) = (stdout, [0; 4096]) else { return };
        while let Ok(n @ 1..) = stdout.read(&mut piece) {
            if pieces.send(piece[..n].to_vec()).is_err() {
                return;
            }
        }
    });
    let (deadline, mut said) = (Instant::now() + LIMIT, Vec::new());
    while marks(&said).len() < 2 && said.len() < MOST {
        match read.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(piece) => said.extend(piece),
            Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => return Err(Shell::TooLong),
        }
    }
    said.truncate(MOST);
    Ok(said)
}

/// Kills the login shell's group, which is named by its pid for as long as it has not been waited for, and then
/// waits for it: nothing it started in its group is left.
fn end(child: &mut Child) {
    if let Some(group) = i32::try_from(child.id()).ok().filter(|pid| *pid > 1).and_then(Pid::from_raw) {
        let _ = kill_process_group(group, Signal::KILL);
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// Where the mark stands in what was printed.
fn marks(said: &[u8]) -> Vec<usize> {
    said.windows(MARK.len()).enumerate().filter(|(_, window)| *window == MARK).map(|(at, _)| at).collect()
}

/// The folders named by a full path between the one pair of marks. No pair, a pair with a line's end inside it,
/// or more marks than two name no folder.
fn named(said: &[u8]) -> Result<Vec<PathBuf>, Shell> {
    match marks(said)[..] {
        [from, to] => {
            let between =
                said.get(from + MARK.len()..to).filter(|path| !path.contains(&b'\n') && !path.contains(&b'\r'));
            Ok(call::full_folders(OsStr::from_bytes(between.ok_or(Shell::NoMarks)?)))
        }
        [] | [_] => Err(Shell::NoMarks),
        _ => Err(Shell::TwoPairs),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn said(text: &str) -> Result<Vec<PathBuf>, Shell> {
        named(text.as_bytes())
    }

    #[test]
    fn only_what_stands_between_one_pair_of_marks_on_one_line_names_folders() {
        let mark = "__jellymoon_path__";
        let both = |path: &str| format!("{mark}{path}{mark}\n");
        let folders = |names: &[&str]| Ok(names.iter().map(PathBuf::from).collect::<Vec<_>>());
        assert_eq!(said(&both("/a/bin:rel/bin:/b c/bin:")), folders(&["/a/bin", "/b c/bin"]));
        assert_eq!(said(&format!("/before/bin\nhello {}/after/bin\n", both("/a/bin"))), folders(&["/a/bin"]));
        assert_eq!(said(&both("")), folders(&[]));
        for none in ["", "/a/bin\n", &format!("{mark}/a/bin\n"), &format!("{mark}/a\n/b{mark}\n"), &both("/a\r/b")] {
            assert_eq!(said(none), Err(Shell::NoMarks), "{none:?}");
        }
        // The command printed back holds no mark: it is in two pieces there.
        assert_eq!(said(&format!("{COMMAND}\n")), Err(Shell::NoMarks));
        assert_eq!(said(&format!("{COMMAND}\n{}", both("/a/bin"))), folders(&["/a/bin"]));
        for more in [both("/a") + &both("/b"), both("/a") + mark, format!("{mark}{}", both("/a"))] {
            assert_eq!(said(&more), Err(Shell::TwoPairs), "{more:?}");
        }
        // Two marks that share their underscores are no pair.
        assert_eq!(said("__jellymoon_path__jellymoon_path__"), Err(Shell::NoMarks));
    }

    // The door lets no test of the binary name a login shell that is not a full path, so that rule is held here.
    // No program is started by this: `fit` reads a file's mode and nothing more.
    #[test]
    fn a_login_shell_that_is_not_named_by_a_full_path_is_refused_before_anything_is_read() {
        for shell in ["zsh", "bin/zsh", "./zsh", "../bin/bash", ""] {
            assert_eq!(fit(Path::new(shell)), Err(Shell::NotFull), "{shell:?}");
        }
        assert_eq!(fit(Path::new("/there/is/no/such/folder/zsh")), Err(Shell::NotAProgram));
    }

    #[test]
    fn the_command_prints_the_mark_it_does_not_hold() {
        let whole: String = COMMAND.split(' ').filter(|piece| piece.contains('_')).collect();
        assert_eq!(whole.as_bytes(), [MARK, MARK].concat());
        assert!(marks(COMMAND.as_bytes()).is_empty() && LIMIT == Duration::from_secs(5) && MOST == 65536);
    }
}
