//! A run's folder in `runs` goes in one step. A look that finds the folder finds all of it, its lock and its record
//! with it. Removed file by file there was a moment with the lock gone, the record gone and the folder still there,
//! and a look in that moment took a run that was never started for one cut off (`run/end.rs`, `bury`).

use std::fs;
use std::io::{self, ErrorKind};
use std::path::Path;
use std::time::Duration;

use crate::busy::retried_for;

/// How long the move is tried again where the folder is in use (Windows). A look has the run's lock open for less
/// than this. A `jellymoon run` that takes its own never-started run back has it open until it lets go, and tries
/// again then (`run.rs`): the day's own wait here would hold that run, and the lock a copy is cleaned behind, for
/// three seconds and still fail.
const MOMENT: Duration = Duration::from_millis(200);

/// Moves `runs/<run>` to a name no look takes for a run, then removes it. What is at that name already goes first,
/// or the move would fail every time: a folder left by a removal that failed half way, where a file still open
/// keeps its folder, or anything else put there. On Windows the move is refused while a process has a file of the
/// folder open, as a look has the run's lock for a moment: it is tried again for [`MOMENT`] (`busy.rs`).
pub(super) fn remove(runs: &Path, run: &str) -> io::Result<()> {
    let gone = runs.join(format!("gone-{run}"));
    clear(&gone)?;
    absent(retried_for(MOMENT, || fs::rename(runs.join(run), &gone)))?;
    clear(&gone)
}

/// Removes a link that stands at a copy's name, as itself: what it leads to is never touched. False when
/// `copies` is not a real folder, when no link is at the name, or when the link could not be removed.
pub(super) fn unlink(root: &Path, folder: &Path) -> bool {
    let is = |path: &Path, link: bool| {
        fs::symlink_metadata(path).is_ok_and(|meta| if link { meta.is_symlink() } else { meta.file_type().is_dir() })
    };
    is(root, false) && is(folder, true) && fs::remove_file(folder).or_else(|_| fs::remove_dir(folder)).is_ok()
}

fn absent(done: io::Result<()>) -> io::Result<()> {
    done.or_else(|e| if e.kind() == ErrorKind::NotFound { Ok(()) } else { Err(e) })
}

/// Removes what is at a name, and nothing is no error: a real folder with all that is in it, and a file or a link
/// as itself. A link is never followed. On Windows a link to a folder is removed as a folder is.
fn clear(at: &Path) -> io::Result<()> {
    match fs::symlink_metadata(at) {
        Ok(meta) if meta.file_type().is_dir() => absent(fs::remove_dir_all(at)),
        Ok(_) => absent(fs::remove_file(at).or_else(|_| fs::remove_dir(at))),
        Err(e) => absent(Err(e)),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::{fs, thread};

    use super::remove;

    // With `fs::remove_dir_all` on the folder itself this fails: among three hundred files, a look finds one of the
    // two it asks for gone and the folder still there.
    #[test]
    fn a_look_finds_a_run_s_folder_whole_or_not_at_all() -> Result<(), Box<dyn std::error::Error>> {
        for _ in 0..5 {
            let (runs, run) = (tempfile::tempdir()?, "r0123456789abcdef");
            let kept = runs.path().join(run);
            fs::create_dir(&kept)?;
            for file in 0..300 {
                fs::write(kept.join(format!("f{file}")), "")?;
            }
            let asked = [kept.join("run.lock"), kept.join("run.json")];
            asked.iter().try_for_each(|file| fs::write(file, ""))?;
            let done = AtomicBool::new(false);
            let torn = thread::scope(|scope| {
                let look = scope.spawn(|| {
                    let mut torn = false;
                    while !done.load(Ordering::Relaxed) {
                        // As `bury` looks: the files first, the folder after.
                        let missing = asked.iter().any(|file| !file.is_file());
                        torn |= missing && kept.is_dir();
                    }
                    torn
                });
                let removed = remove(runs.path(), run);
                done.store(true, Ordering::Relaxed);
                removed.map(|()| look.join().unwrap_or(true))
            })?;
            assert!(!torn, "the folder was seen with a file of it gone");
            assert_eq!(fs::read_dir(runs.path())?.count(), 0, "nothing of the run is left, under any name");
            remove(runs.path(), run)?; // nothing to remove is no error: a second removal follows a first that failed
        }
        Ok(())
    }

    #[cfg(unix)]
    fn link(to: &std::path::Path, at: &std::path::Path) -> bool {
        std::os::unix::fs::symlink(to, at).is_ok()
    }

    #[cfg(windows)]
    fn link(to: &std::path::Path, at: &std::path::Path) -> bool {
        std::os::windows::fs::symlink_dir(to, at).is_ok() // refused where the account may not make links
    }

    // Whatever is at `gone-<id>` already: a folder a removal left half way, a file, or a link to a folder elsewhere,
    // which is removed as a link and never followed.
    #[test]
    fn what_is_in_the_way_at_the_gone_name_is_removed_first_and_a_link_there_is_never_followed()
    -> Result<(), Box<dyn std::error::Error>> {
        for way in ["folder", "file", "link"] {
            let (home, run) = (tempfile::tempdir()?, "r0123456789abcdef");
            let (runs, elsewhere) = (home.path().join("runs"), home.path().join("elsewhere"));
            let (kept, gone) = (runs.join(run), runs.join(format!("gone-{run}")));
            for folder in [&kept, &elsewhere] {
                fs::create_dir_all(folder)?;
                fs::write(folder.join("run.json"), "kept\n")?;
            }
            match way {
                "folder" => fs::create_dir(&gone).and_then(|()| fs::write(gone.join("left"), ""))?,
                "file" => fs::write(&gone, "")?,
                _ if !link(&elsewhere, &gone) => continue,
                _ => {}
            }
            remove(&runs, run).map_err(|e| format!("{way}: {e}"))?;
            assert_eq!(fs::read_dir(&runs)?.count(), 0, "{way}: nothing of the run is left, under any name");
            assert_eq!(fs::read_to_string(elsewhere.join("run.json"))?, "kept\n", "{way}: outside `runs`");
        }
        Ok(())
    }
}
