//! A run's folder in `runs` goes in one step. A look that finds the folder finds all of it, its lock and its record
//! with it. Removed file by file there was a moment with the lock gone, the record gone and the folder still there,
//! and a look in that moment took a run that was never started for one cut off (`run/end.rs`, `bury`).

use std::fs;
use std::io::{self, ErrorKind};
use std::path::Path;

/// Moves `runs/<run>` to a name no look takes for a run, then removes it. One left by a removal that failed half
/// way, where a file still open keeps its folder, is removed by the next.
pub(super) fn remove(runs: &Path, run: &str) -> io::Result<()> {
    let gone = runs.join(format!("gone-{run}"));
    let absent = |done: io::Result<()>| match done {
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        done => done,
    };
    absent(fs::rename(runs.join(run), &gone))?;
    absent(fs::remove_dir_all(&gone))
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
}
