//! The words a page's notes of work that waits say first, for a person who has never seen Jellymoon: where the
//! change is, what `Keep it` and `Toss it` do to their folder, and after a keep or a toss where it is now
//! (`docs/development/show.md`, "Work that waits"). They are the window's alone: the command line's `why` stays as
//! it is, and no run's id is in them. Each is true of every way its wait, keep or toss can end.

use crate::copy::How;
use crate::run::{Kept, Tossed};

/// The first line of the note of work that waits and was read, under its title that the folder is not changed
/// yet: the change is in a copy, and what each choice the note offers does. `keeps`: the note offers `Keep it`.
/// It is one line of a note, whatever the project: a note of the run's result over it has the room it leaves.
pub(super) fn waits(keeps: bool) -> &'static str {
    if keeps {
        "It is in a copy: Keep it puts it in; Toss it throws it away."
    } else {
        "It is in a copy: Toss it throws it away."
    }
}

/// The first line of the note of work that waits and could not be read: what the person can do next.
pub(super) fn unread(project: &str) -> String {
    format!(
        "Your folder {project} is not changed. To keep or toss the change here, put right what is said below, then \
         open the wish again."
    )
}

/// How many files the work made, changed and removed, as a person counts them: `1 file changed`, `2 new files,
/// 1 changed`. `of`: how many files the list of the work held, when it stopped before the end of the work.
pub(super) fn counted(made: usize, changed: usize, removed: usize, of: Option<usize>) -> String {
    let files = |n: usize| if n == 1 { "file" } else { "files" };
    let mut said: Vec<String> = Vec::new();
    for (n, how) in [(made, "new"), (changed, "changed"), (removed, "removed")] {
        match (n, said.is_empty(), how) {
            (0, ..) => {}
            (_, true, "new") => said.push(format!("{n} new {}", files(n))),
            (_, true, _) => said.push(format!("{n} {} {how}", files(n))),
            (_, false, _) => said.push(format!("{n} {how}")),
        }
    }
    let said = if said.is_empty() { "No file is listed".to_string() } else { said.join(", ") };
    match of {
        Some(of) => format!("{said}, counted in the first {of} files."),
        None => format!("{said}."),
    }
}

/// What a page says of a keep that ended well. Only a keep that landed the work says that the change is in the
/// folder now; one that found it there, or kept before, says nothing was changed. Where the folder was switched
/// while the keep was at work, or a `.gitattributes` stayed, core's own sentence follows.
pub(super) fn kept(kept: &Kept, project: &str) -> Vec<String> {
    let now = format!("The change is in your folder {project} now.");
    let first = match kept.how {
        Some(How::Forward | How::Joined | How::Finished) => now,
        Some(How::Held) => format!("Your folder {project} had this change already, so nothing in it was changed."),
        Some(How::Elsewhere | How::Loose) => {
            format!("{now} The folder was switched while it was kept, and the change went where it stood: see below.")
        }
        None => "This change was kept before, so nothing was changed now.".to_string(),
    };
    let more = matches!(kept.how, Some(How::Elsewhere | How::Loose)) || !kept.stayed.is_empty();
    [first].into_iter().chain(more.then(|| format!("{}.", kept.why))).collect()
}

/// What a page says of a toss that ended well. A toss never touches the person's folder, only what the run left.
pub(super) fn tossed(tossed: &Tossed, project: &str) -> String {
    if tossed.tossed {
        format!("The change was thrown away, and your folder {project} is as it was.")
    } else {
        "This change was thrown away before, so nothing was changed now.".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::counted;

    #[test]
    fn a_count_reads_as_a_person_counts_files() {
        assert_eq!(counted(0, 1, 0, None), "1 file changed.");
        assert_eq!(counted(2, 1, 0, None), "2 new files, 1 changed.");
        assert_eq!(counted(1, 3, 1, None), "1 new file, 3 changed, 1 removed.");
        assert_eq!(counted(0, 0, 2, None), "2 files removed.");
        assert_eq!(counted(0, 2, 1, None), "2 files changed, 1 removed.");
        assert_eq!(
            counted(198, 1, 1, Some(200)),
            "198 new files, 1 changed, 1 removed, counted in the first 200 files."
        );
        assert_eq!(counted(0, 0, 0, None), "No file is listed.");
    }
}
