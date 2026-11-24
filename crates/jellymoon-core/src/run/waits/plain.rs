//! The words a page's notes of work that waits say first, for a person who has never seen Jellymoon: where the
//! change is, what `Keep it` and `Toss it` do to their folder, and after a keep or a toss where it is now
//! (`docs/development/show.md`, "Work that waits"). They are the window's alone: the command line's `why` stays as
//! it is, and no run's id is in them. Each is true of every way its wait, keep or toss can end.

use super::{UNKEPT, WorkDone, again};
use crate::copy::{How, Unkept};
use crate::run::show::Alone;
use crate::run::{Kept, Tossed};
use crate::{Wish, WishState};

/// The first line of the note of work that waits and was read, under its title that the folder is not changed
/// yet: this change was not kept by itself (F92) and waits for a yes, and what each choice the note offers does.
/// `keeps`: the note offers `Keep it`. It is one line of a note, whatever the project: a note of the run's result
/// over it has the room it leaves.
pub(super) fn waits(keeps: bool) -> &'static str {
    if keeps {
        "It waits for your yes: Keep it puts it in; Toss it throws it away."
    } else {
        "It waits for your yes: Toss it throws it away."
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

/// A keep by a tap that core refused as the folder changed in the same place since, and as changes of the
/// person's own are in the way: what a page says of each, with no run's id and no word of git's.
const CLASH: &str = "Your folder changed in the same place after this change was made, so the two do not fit together.";
const UNCHANGED: &str = "Nothing in your folder was changed.";
const IN_WAY: &str =
    "Your folder has changes of your own that are not saved to the project yet, where this change would go.";
const SAVE: &str = "Nothing in your folder was changed. Save or put away your own changes, then tap Keep it again.";

/// The note of a keep by a tap that was refused for `why`, when that is one of the two reasons a page says in
/// its own sentences. A change that no longer fits is never kept by a later tap, so its note has the one way
/// on, and says what that does. None of every other reason: it is told in the command line's sentence, as it was.
pub(super) fn unkept(why: Option<Unkept>) -> Option<WorkDone> {
    let said = |texts: &[&str]| texts.iter().map(|text| text.to_string()).collect();
    match why? {
        Unkept::Clash => Some(WorkDone::Offers(UNKEPT, said(&[CLASH, UNCHANGED, again::DOES]), again::AGAIN)),
        Unkept::InWay => Some(WorkDone::Said(UNKEPT, said(&[IN_WAY, SAVE]))),
        _ => None,
    }
}

/// What a page says of a toss that ended well. A toss never touches the person's folder, only what the run left.
pub(super) fn tossed(tossed: &Tossed, project: &str) -> String {
    if tossed.tossed {
        format!("The change was thrown away, and your folder {project} is as it was.")
    } else {
        "This change was thrown away before, so nothing was changed now.".to_string()
    }
}

/// What a page says after the wish's words of a run that did not finish (told alone): what came of it and what the
/// person can do, and no reason of core's, which names a run and a word of the harness's. True of every failed
/// run: some stopped early, and some had finished when their work could not be committed or their end recorded.
const UNFINISHED: [&str; 2] = [
    "It did not finish, and nothing in your folder was changed.",
    "You can type the wish again, or type it with what should be different.",
];

/// What a page says of a wish that has no work waiting: how it stands, and the wish's own words; and of a wish
/// that needs the person because its run did not finish, [`UNFINISHED`]. `alone` is how its run ended.
pub fn stands(wish: &Wish, alone: Option<&Alone>) -> (&'static str, Vec<String>) {
    let stands = match wish.state {
        WishState::Working => "This wish is being worked on.",
        WishState::Done => "This wish is done.",
        WishState::NeedsYou => "This wish needs you.",
        WishState::LetGo => "This wish was let go.",
    };
    let mut said = vec![format!("“{}”", wish.said)];
    if wish.state == WishState::NeedsYou && alone == Some(&Alone::Unfinished) {
        said.extend(UNFINISHED.map(str::to_string));
    }
    (stands, said)
}

#[cfg(test)]
mod tests {
    use super::{Unkept, WorkDone, counted, unkept};

    // Word for word, and with nothing in them that a run's id or a sentence of the command line's could be.
    #[test]
    fn a_keep_that_no_longer_fits_and_one_with_changes_in_the_way_are_said_in_the_window_s_own_sentences() {
        let title = "The change was not kept.";
        let clash = [
            "Your folder changed in the same place after this change was made, so the two do not fit together.",
            "Nothing in your folder was changed.",
            "Make it again throws this one away and makes the wish again on your folder as it is now.",
        ];
        let in_way = [
            "Your folder has changes of your own that are not saved to the project yet, where this change would go.",
            "Nothing in your folder was changed. Save or put away your own changes, then tap Keep it again.",
        ];
        let texts = |said: &[&str]| said.iter().map(|text| text.to_string()).collect::<Vec<_>>();
        assert_eq!(unkept(Some(Unkept::Clash)), Some(WorkDone::Offers(title, texts(&clash), "Make it again")));
        // Changes in the way go with a save, so `Keep it` is the way on and no other choice is offered.
        assert_eq!(unkept(Some(Unkept::InWay)), Some(WorkDone::Said(title, texts(&in_way))));
        // Every other reason, and a refusal that came to none, is told as the command line tells it.
        for other in [Unkept::Busy, Unkept::History, Unkept::Midway, Unkept::Settings, Unkept::Stepped] {
            assert_eq!(unkept(Some(other)), None, "{other:?}");
        }
        assert_eq!(unkept(None), None);
    }

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
