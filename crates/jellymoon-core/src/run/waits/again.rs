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
/// The same once that wish is worked on no more: only what stays true.
const MADE_SINCE: &str = "Your wish was made again.";

impl WorkDone {
    /// What a page says when the wish was not made again: `title` and `why` are what a typed wish that was not
    /// kept is told, and after them that nothing was tossed.
    pub fn unmade(title: &'static str, why: String) -> WorkDone {
        WorkDone::Said(title, vec![why, NOT_TOSSED.to_string()])
    }

    /// What a page says once the new wish was made: this, which is what the toss of the work that does not fit
    /// came to, done or refused, and after it that the wish was made again, or `unstarted`, the title and the
    /// reason a typed wish that was kept and has no run is told, both.
    pub fn remade(self, unstarted: Option<(&'static str, String)>) -> WorkDone {
        match self {
            WorkDone::Said(title, mut said) => {
                match unstarted {
                    Some((kept, why)) => said.extend([kept.to_string(), why]),
                    None => said.push(MADE.to_string()),
                }
                WorkDone::Said(title, said)
            }
            other => other,
        }
    }

    /// [`WorkDone::remade`] of a wish whose run was started, as a page says it once that wish is worked on no
    /// more: what the toss came to, and that the wish was made again, with no word of work that has ended.
    /// Asked of the toss's answer before it is remade. Which wish that is, and whether it is at work, is the
    /// shell's to know: it made the wish. None of an answer that is no sentence.
    pub fn remade_since(&self) -> Option<WorkDone> {
        let WorkDone::Said(title, said) = self else { return None };
        Some(WorkDone::Said(title, said.iter().cloned().chain([MADE_SINCE.to_string()]).collect()))
    }
}

#[cfg(test)]
mod tests {
    use super::{MADE, MADE_SINCE, WorkDone};

    #[test]
    fn a_wish_made_again_that_has_no_run_is_told_so_before_the_reason() {
        let tossed = || WorkDone::Said("The change was thrown away.", vec!["Your folder is as it was.".to_string()]);
        let said = |texts: &[&str]| texts.iter().map(|text| text.to_string()).collect::<Vec<_>>();
        let kept = "Your wish is kept, and no run was started for it.";
        assert_eq!(
            tossed().remade(Some((kept, "the reason".to_string()))),
            WorkDone::Said("The change was thrown away.", said(&["Your folder is as it was.", kept, "the reason"]))
        );
        assert_eq!(
            tossed().remade(None),
            WorkDone::Said("The change was thrown away.", said(&["Your folder is as it was.", MADE]))
        );
        // Once the new wish is worked on no more, the same answer says only that it was made.
        assert_eq!(
            tossed().remade_since(),
            Some(WorkDone::Said("The change was thrown away.", said(&["Your folder is as it was.", MADE_SINCE])))
        );
    }
}
