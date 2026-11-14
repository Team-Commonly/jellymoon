//! What a hearing asks of the day: which project a heard wish is for, the making of the wish, and what the day's
//! runs give the jellyfish to show. Only [`make`] writes, and only as the page's entry does.

use std::path::{Path, PathBuf};

use super::{Jelly, JellyState};
use crate::{DayOptions, RunOptions, WishState, run, run_apart, wish, wish_offered};

/// What is said when no project can be picked for a heard wish.
pub(super) const NO_PROJECT: &str = "nothing was kept: make the day's first wish on the page, so that Jellymoon knows \
                                     which project a wish said aloud is for";
const ASKS: &str = "a run asks, and waits for your answer on the page";
const WAITS: &str = "a run's work waits for you on the page";

/// The day's runs, as its made wishes tell of them.
#[derive(Default)]
pub(super) struct Runs {
    going: usize,
    /// The first question a run asks.
    question: Option<String>,
    waits: bool,
}

impl Runs {
    /// What the jellyfish shows when nothing is heard: a run that asks, then runs going, then work that waits.
    pub(super) fn drawn(&self) -> Jelly {
        let blank = Jelly::quiet();
        match (&self.question, self.going, self.waits) {
            (Some(question), ..) => {
                let (says, question) = (ASKS.to_string(), Some(question.clone()));
                Jelly { state: JellyState::Asks, says, question, ..blank }
            }
            (None, going @ 1.., _) => Jelly { state: JellyState::Working, going: Some(going), ..blank },
            (None, 0, true) => Jelly { state: JellyState::Waits, says: WAITS.to_string(), ..blank },
            (None, 0, false) => blank,
        }
    }
}

/// One look at the day's made wishes, which writes nothing: a run cut off since is still read as going, until
/// the page is looked at (`wish::made`). Made wishes that cannot be used give nothing to show.
pub(super) fn look(home: &Path, day: &str) -> Runs {
    let mut runs = Runs::default();
    for wish in wish::kept_of(home, day).unwrap_or_default() {
        let Some(run) = &wish.run else { continue };
        match wish.state {
            WishState::NeedsYou if runs.question.is_none() => runs.question = wish.question,
            WishState::Working => runs.going += 1,
            WishState::Done => runs.waits |= run::work_waits(home, run),
            _ => {}
        }
    }
    runs
}

/// The project a wish said aloud is for: the one of the day's last made wish, when a wish can still be made for
/// it, else the only one a wish can be made for. None when there is none, or more than one and nothing to choose by.
pub(super) fn project(read: &DayOptions, home: &Path, day: &str) -> Option<String> {
    let mut offered = wish::offered(read);
    let last = wish::kept_of(home, day).ok().and_then(|made| made.into_iter().last()).map(|wish| wish.project);
    match last.filter(|last| offered.contains(last)) {
        Some(last) => Some(last),
        None if offered.len() == 1 => offered.pop(),
        None => None,
    }
}

/// Makes the heard wish a wish, by the call the page's entry uses, and with `apart` starts its run as
/// `jellymoon wish --run` does. Ok: the wish's id, and why no run was started when one was asked for and none
/// was. Err: core's reason, and nothing was written.
pub(super) fn make(
    read: &DayOptions,
    home: &Path,
    apart: Option<&(PathBuf, RunOptions)>,
    words: &str,
    project: &str,
) -> Result<(String, Option<String>), String> {
    let made = wish_offered(read, words, project).map_err(|e| e.to_string())?.wish.id;
    let Some((program, options)) = apart else { return Ok((made, None)) };
    let options = RunOptions { wish: made.clone(), day: read.date.clone(), ..options.clone() };
    let unstarted = match run_apart(home, program, &options) {
        Ok(started) => started.run.is_none().then_some(started.why),
        Err(e) => Some(e.to_string()),
    };
    Ok((made, unstarted))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ADR 0019, decision 4: a run that asks is shown before runs that go, and those before work that waits.
    #[test]
    fn a_run_that_asks_is_shown_before_runs_going_and_those_before_work_that_waits() {
        let drawn = |going, question: Option<&str>, waits| {
            let shown = Runs { going, question: question.map(str::to_string), waits }.drawn();
            (shown.state, shown.going, shown.question, shown.says)
        };
        let none = String::new();
        assert_eq!(drawn(0, None, false), (JellyState::Quiet, None, None, none.clone()));
        assert_eq!(drawn(0, None, true), (JellyState::Waits, None, None, WAITS.to_string()));
        assert_eq!(drawn(2, None, true), (JellyState::Working, Some(2), None, none));
        let asked = Some("Which one?".to_string());
        assert_eq!(drawn(2, Some("Which one?"), true), (JellyState::Asks, None, asked, ASKS.to_string()));
    }
}
