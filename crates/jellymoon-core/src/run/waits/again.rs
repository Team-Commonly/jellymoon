//! `Make it again` (ADR 0030, decision 10; `docs/development/show.md`, "Work that waits"): the one way on from
//! a change that no longer fits the person's folder, which no later `Keep it` would keep. The wish is made again
//! with its own words, as typed words make one, and only once it was made is the work that does not fit tossed:
//! a wish that was refused leaves that work where it was. Its run is started after the toss, since a toss asks
//! where every copy of the repo stands and is refused while a run's copy is being made. Core says which words
//! and which project (`work_tapped`), and every word a page says of it is here; the shell makes the wish and
//! starts its run as it does for the entry, and asks for the toss as a tap on `Toss it` does.

use super::WorkDone;

pub(super) const AGAIN: &str = "Make it again";
/// What the note that offers the choice says of it, before it.
pub(super) const DOES: &str =
    "Make it again throws this one away and makes the wish again on your folder as it is now.";
pub(super) const NOT_AGAIN: &str = "The wish was not made again.";
const NOT_TOSSED: &str = "The change that does not fit was not thrown away.";
const MADE: &str = "Your wish was made again, and it is being worked on.";

impl WorkDone {
    /// What a page says when the wish was not made again: `title` and `why` are what a typed wish that was not
    /// kept is told, and after them that nothing was tossed.
    pub fn unmade(title: &'static str, why: String) -> WorkDone {
        WorkDone::Said(title, vec![why, NOT_TOSSED.to_string()])
    }

    /// What a page says once the new wish was made: this, which is what the toss of the work that does not fit
    /// came to, done or refused, and after it that the wish was made again, or `unstarted`, what a typed wish
    /// that was kept and has no run is told.
    pub fn remade(self, unstarted: Option<String>) -> WorkDone {
        match self {
            WorkDone::Said(title, mut said) => {
                said.push(unstarted.unwrap_or_else(|| MADE.to_string()));
                WorkDone::Said(title, said)
            }
            other => other,
        }
    }
}
