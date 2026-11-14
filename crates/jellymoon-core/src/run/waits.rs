//! Work that waits, for a page (F47, ADR 0017, decision 9 and "Found while building F47"; `docs/development/show.md`,
//! "Work that waits"). [`waits`] says whether a made wish's work waits and what it is, [`waiting`] says it of every
//! made wish of a day, and [`work_tapped`] is a tap on one of the choices under it. Nothing new is decided of a
//! repo here: the work is read by `show.rs`, kept by `keep.rs` and tossed by `toss.rs`, each with its own checks.
//! What is here is which wishes those are, the choices, the one more yes a keep of files the person's tools obey
//! takes and what that yes is a yes to, and every word a page says of any of it, so that a shell words nothing
//! and decides nothing.
//!
//! The rule of that yes: a page keeps a file the person's tools obey only when that file's name was drawn,
//! whole, on the note the yes was given under. A count is not a name: the agent picks the files, so it picks
//! which would fall behind a count. So the note that asks names each such file in a text of its own and counts
//! none, work with more of them than a note names has no yes on a page at all, and the names the tap carried
//! are the ones the keep lands (`keep_named`). A name is drawn whole only when it is printed as it is: work with
//! one that cleaning changed, as a name that looks like a credential is, has no yes on a page either.

use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::keep::keep_named;
use super::show::{self, FileChange, FileKind, Shown, ShownFile, show_file};
use super::toss::toss_run;
use super::{RunOptions, refused};
use crate::{Error, Wish, WishState, wish};

/// The most names of files that are said of a run's work, and the most files the person's tools obey that the
/// note that asks names. Each is a text of its own there, and a text note draws its choices only under a body
/// drawn whole: six names of a line each, under the title and over the sentence and the three choices, are what
/// the smallest window a page is drawn at (960x640) has room for. Longer names take more lines, and then the
/// note has no choice either.
const NAMED: usize = 6;
/// The most characters of names on the line of a work's files. A text note draws 400 characters of one text and
/// its choices only under texts drawn whole, so a longer line would leave the work with no choice on a page.
const LINE: usize = 320;
/// The most files one tap on [`SHOW`] asks for, when one after another cannot be shown.
const TRIED: usize = 8;

const SHOW: &str = "Show me";
const KEEP: &str = "Keep it";
const KEEP_TOO: &str = "Keep it, with those files";
const TOSS: &str = "Toss it";
/// What a page says of work it has no yes for: [`Waits::over`].
const OVER: &str = "A note cannot name every file of this work that your tools obey, each whole and as it is, so a \
                    page does not keep it. It is kept from the command line, with `jellymoon run keep`.";

/// The work of one made wish's run that waits for the person.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Waits {
    pub wish: String,
    pub run: String,
    /// The wish's own words, as they are kept.
    pub said: String,
    /// Whether the work was read from its repo. Of work that was not, the counts are 0 and no file is named.
    pub read: bool,
    /// How many of the listed files the work made, changed and removed.
    pub made: usize,
    pub changed: usize,
    pub removed: usize,
    /// The first files' names, cleaned as `run show` cleans them: at most 6.
    pub names: Vec<String>,
    /// How many files of the work there are after those.
    pub more: usize,
    /// Every file of the whole work the person's tools obey, past the listed ones too, by its cleaned name.
    /// A keep from a page takes one more yes when there is one.
    pub obeyed: Vec<String>,
    /// Whether each of those names is printed as git keeps it, whole and unchanged. One that looks like a
    /// credential, or holds what is not printed, is not, and a page then has no yes for the work.
    pub plain: bool,
    /// What waits, in core's fixed words, or why the work was not read: a refusal of `run show`.
    pub why: String,
}

/// What `jellymoon run waiting` prints.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Waiting {
    pub day: String,
    /// The day's made wishes whose work waits, in their order.
    pub waits: Vec<Waits>,
}

impl fmt::Display for Waiting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_json::to_string(self).map_err(|_| fmt::Error)?)
    }
}

/// One choice under work that waits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkTap {
    /// The next file after the one of that number that is text to show: from the first again past the last.
    Show {
        after: usize,
    },
    /// `yes_to`: the words of the note that named the files the person's tools obey, when the choice was the
    /// one under it that keeps them too: each name as it was drawn, and after them the note's last sentence. The
    /// keep is done only when the work's note would say those words now, and it lands those names and no other.
    Keep {
        yes_to: Option<Vec<String>>,
    },
    Toss,
}

/// What a tap on a choice came to.
#[derive(Debug, PartialEq, Eq)]
pub enum WorkDone {
    /// One file's text. The work waits as before.
    Shown(Waits, ShownFile),
    /// The work touches files the person's tools obey: nothing was changed, and a keep is asked once more.
    Asks(Waits),
    /// It was done, or it was refused with nothing changed: a title, and core's sentence.
    Said(&'static str, String),
}

impl WorkTap {
    /// The tap on the choice that reads `choice`, under a note whose words are `drawn`: what a page drew is all
    /// a tap is made of. `after` is the number of the file on show, or 0. None for words that are no choice.
    pub fn of(choice: &str, drawn: &[String], after: usize) -> Option<WorkTap> {
        match choice {
            SHOW => Some(WorkTap::Show { after }),
            KEEP => Some(WorkTap::Keep { yes_to: None }),
            KEEP_TOO => Some(WorkTap::Keep { yes_to: Some(drawn.to_vec()) }),
            TOSS => Some(WorkTap::Toss),
            _ => None,
        }
    }

    /// What a page says of a tap whose carrying failed before core answered. It ends what the page said was
    /// being done, so the wish can be opened again.
    pub fn failed(&self) -> WorkDone {
        let unknown = "Jellymoon failed before it had an answer, so what was changed is not known here. Open the \
                       wish again to see how its work stands.";
        WorkDone::Said("That did not finish.", unknown.to_string())
    }

    /// What a page says while the tap is at work.
    pub fn doing(&self) -> &'static str {
        match self {
            WorkTap::Show { .. } => "That work is being read.",
            WorkTap::Keep { .. } => "That work is being kept.",
            WorkTap::Toss => "That work is being tossed.",
        }
    }

    fn undone(&self) -> &'static str {
        match self {
            WorkTap::Show { .. } => "No file of that work was shown.",
            WorkTap::Keep { .. } => "That work was not kept.",
            WorkTap::Toss => "That work was not tossed.",
        }
    }
}

/// Names on one line, the first [`NAMED`] of them that are no more than [`LINE`] characters together, and how
/// many there are beside. Only the note of work that waits says its files so: nothing is kept on the word of it.
fn named(names: &[String], beside: usize) -> String {
    let (mut said, mut told) = (String::new(), 0);
    for name in names.iter().take(NAMED) {
        if told > 0 && said.chars().count() + 2 + name.chars().count() > LINE {
            break;
        }
        said.push_str(if told > 0 { ", " } else { "" });
        said.push_str(name);
        told += 1;
    }
    let more = names.len() - told + beside;
    if more > 0 { format!("{said}, and {more} more") } else { said }
}

impl Waits {
    /// The title of the note of this work.
    pub fn title(&self) -> &'static str {
        if self.read { "Work waits for this wish." } else { "Work waits for this wish, and it was not read." }
    }

    /// Whether the work touches more files the person's tools obey than a note names, or one whose name is not
    /// printed as it is: a page has no yes for it.
    fn over(&self) -> bool {
        self.obeyed.len() > NAMED || !self.plain
    }

    /// What the note says: the wish's own words, what waits or why it was not read, the files' names, and of
    /// work a page does not keep that it does not.
    pub fn lines(&self) -> Vec<String> {
        let mut lines = vec![format!("“{}”", self.said), format!("{}.", self.why)];
        lines.extend((!self.names.is_empty()).then(|| named(&self.names, self.more)));
        lines.extend(self.over().then(|| OVER.to_string()));
        lines
    }

    /// The choices under the note: none of work that was not read, so a tap there changes nothing, and no keep
    /// of work a page does not keep. `sure` gives the ones under the note that asks once more.
    pub fn choices(&self, sure: bool) -> Vec<&'static str> {
        match (self.read, self.over()) {
            (false, _) => Vec::new(),
            (true, true) => vec![SHOW, TOSS],
            (true, false) => vec![SHOW, if sure { KEEP_TOO } else { KEEP }, TOSS],
        }
    }

    /// The title and the words of the note that asks once more before a keep: every file of the whole work the
    /// person's tools obey, each name a text of its own, and none counted. A yes is a yes to these names. Of
    /// work a page has no yes for ([`Waits::over`]) it says so instead, and names none.
    pub fn asks(&self) -> (&'static str, Vec<String>) {
        let title = "This work touches files your tools obey.";
        if self.over() {
            return (title, vec![OVER.to_string(), "Nothing was changed.".to_string()]);
        }
        let kept = "Nothing was changed. Kept, they are in your checkout, where your tools read them.";
        (title, self.obeyed.iter().cloned().chain([kept.to_string()]).collect())
    }
}

impl ShownFile {
    /// The title of the note a page shows the file in.
    pub fn title(&self) -> String {
        format!("File {} of the work: {}", self.n, self.name)
    }

    /// The file's text, a line of it each, and after a text that was cut a line that says so.
    pub fn lines(&self) -> Vec<String> {
        let mut lines: Vec<String> = self.text.lines().map(str::to_string).collect();
        lines.extend(self.cut.then(|| "The file goes on past what was read of it.".to_string()));
        lines
    }
}

/// What a page says of a wish that has no work waiting: how it stands, and the wish's own words.
pub fn stands(wish: &Wish) -> (&'static str, Vec<String>) {
    let stands = match wish.state {
        WishState::Working => "This wish is being worked on.",
        WishState::Done => "This wish is done.",
        WishState::NeedsYou => "This wish needs you.",
        WishState::LetGo => "This wish was let go.",
    };
    (stands, vec![format!("“{}”", wish.said)])
}

/// The work of the wish's run when it waits, and the list `run show` gives of it when the repo could be read.
fn read(home: &Path, options: &RunOptions) -> Option<(Waits, Option<Shown>)> {
    let found = show::waiting(home, options)?;
    let (wish, run, said) = (options.wish.clone(), found.run, found.made.said);
    let (shown, obeyed, plain) = match show::whole(home, options) {
        Ok(whole) => whole,
        Err(unread) => {
            let (names, obeyed, plain, why) = (Vec::new(), Vec::new(), true, unread.to_string());
            let (read, made, changed, removed, more) = (false, 0, 0, 0, 0);
            let waits = Waits { wish, run, said, read, made, changed, removed, names, more, obeyed, plain, why };
            return Some((waits, None));
        }
    };
    let count = |change: FileChange| shown.files.iter().filter(|file| file.change == change).count();
    let (made, changed, removed) = (count(FileChange::Made), count(FileChange::Changed), count(FileChange::Removed));
    let names: Vec<String> = shown.files.iter().take(NAMED).map(|file| file.name.clone()).collect();
    let more = shown.files.len() - names.len() + shown.more;
    let first = if shown.more > 0 { format!(" of its first {}", shown.files.len()) } else { String::new() };
    let why = format!("the work of run {run} waits: files made {made}, changed {changed}, removed {removed}{first}");
    let read = true;
    Some((Waits { wish, run, said, read, made, changed, removed, names, more, obeyed, plain, why }, Some(shown)))
}

/// The work of the wish's run, when it waits for the person: the run finished with its one commit, and its record
/// names that commit and says neither kept nor tossed. None of any other wish, and then no repo is asked
/// anything. The repo is found as `run show` finds it; when it is not, the answer says why and names no file.
pub fn waits(home: &Path, options: &RunOptions) -> Option<Waits> {
    read(home, options).map(|(waits, _)| waits)
}

/// [`waits`] of every made wish of the day `options` names: its `wish` is not read.
pub fn waiting(home: &Path, options: &RunOptions) -> Result<Waiting, Error> {
    let day = wish::day_of(options.day.as_deref())?;
    let made = wish::kept_of(home, &day)?;
    let one = |made: &wish::MadeWish| waits(home, &RunOptions { wish: made.id.clone(), ..options.clone() });
    Ok(Waiting { waits: made.iter().filter_map(one).collect(), day })
}

/// The first file after number `after` that `run show --file` prints, from the first again past the last.
fn next(home: &Path, options: &RunOptions, shown: &Shown, after: usize) -> Result<ShownFile, Error> {
    let text = |file: &&show::ShownEntry| file.kind == FileKind::Text && file.change != FileChange::Removed;
    let (later, earlier): (Vec<usize>, Vec<usize>) =
        shown.files.iter().filter(text).map(|file| file.n).partition(|n| *n > after);
    let mut last = refused(format!("no file of the work of run {} is text that can be shown", shown.run));
    for n in later.into_iter().chain(earlier).take(TRIED) {
        match show_file(home, options, n) {
            Ok(file) => return Ok(file),
            Err(unshown) => last = unshown,
        }
    }
    Err(last)
}

/// A tap on a choice under the work of the wish's run. Nothing is done unless the work waits now and was read
/// from its repo now: a toss asked where the repo is not found would remove what the run left in the home and
/// leave its branch, and no tap is let begin that.
///
/// A keep of work that touches files the person's tools obey, anywhere in the whole work, changes nothing the
/// first time and names them, each in a text of its own; of more than a note names, and of a name that is not
/// printed as it is, it only says that a page does not keep the work, whatever the tap. A yes to that note
/// keeps them only when the note would say the same words of the work as it is now; and the keep itself is
/// given the names the tap carried, and lands the work only when they are the files of it the person's tools
/// obey, all of them and no other (`keep_named`). So a keep that was never asked once more lands none.
pub fn work_tapped(home: &Path, options: &RunOptions, tap: WorkTap) -> WorkDone {
    let undone = tap.undone();
    let no = |why: String| WorkDone::Said(undone, format!("{why}."));
    let Some((waits, shown)) = read(home, options) else {
        return no("no work of that wish waits any more, so nothing was changed".to_string());
    };
    let Some(shown) = shown else { return no(waits.why) };
    match tap {
        WorkTap::Show { after } => match next(home, options, &shown, after) {
            Ok(file) => WorkDone::Shown(waits, file),
            Err(unshown) => no(unshown.to_string()),
        },
        WorkTap::Keep { .. } if waits.over() => WorkDone::Asks(waits),
        WorkTap::Keep { yes_to: None } if !waits.obeyed.is_empty() => WorkDone::Asks(waits),
        WorkTap::Keep { yes_to: Some(drawn) } if drawn != waits.asks().1 => no(format!(
            "the files of the work of run {} that your tools obey are not the ones that note named, so it was not \
             kept, and nothing was changed",
            waits.run
        )),
        // The names the tap carried are the note's texts but the last, its sentence: none, of a keep that was
        // not asked once more. The keep lands the work only when they are the files of it the tools obey.
        WorkTap::Keep { yes_to } => {
            let drawn = yes_to.unwrap_or_default();
            let named = drawn.split_last().map_or(&[][..], |(_, named)| named);
            match keep_named(home, options, named) {
                Ok(kept) => WorkDone::Said("That work was kept.", format!("{}.", kept.why)),
                Err(unkept) => no(unkept.to_string()),
            }
        }
        WorkTap::Toss => match toss_run(home, options) {
            Ok(tossed) => WorkDone::Said("That work was tossed.", format!("{}.", tossed.why)),
            Err(untossed) => no(untossed.to_string()),
        },
    }
}
