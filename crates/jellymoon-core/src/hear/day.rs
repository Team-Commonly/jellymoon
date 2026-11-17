//! What a hearing asks of the day: which project a heard wish is for, the making of the wish, and what the day's
//! runs give the jellyfish to show. Only [`make`] writes, and only as the page's entry does.

use std::path::{Path, PathBuf};

use super::{Jelly, JellyState};
use crate::{DayOptions, RunOptions, WishState, find, run, run_apart, wish, wish_offered};

/// What is said when no project can be picked for a heard wish.
pub(super) const NO_PROJECT: &str = "nothing was kept: make the day's first wish on the page, so that Jellymoon knows \
                                     which project a wish said aloud is for";
/// What is said when a run was asked for and no harness is there to run it, each short enough for the bubble:
/// where it was looked for, and which program was passed over, is what `jellymoon wish --run` says. This one is
/// for a search that found nothing, with a home to look in.
pub(super) const NO_HARNESS: &str =
    "the wish is kept, but no run was started: no `claude` program was found (JELLYMOON_CLAUDE names one)";
/// A `claude` was found and passed over, so "none was found" would be untrue.
pub(super) const PASSED_OVER: &str = "the wish is kept, but no run was started: a `claude` program was found but is \
                                      not used, as others can write to it or its folder";
/// Nothing was found, and the places under a home were not looked in.
pub(super) const NO_HOME: &str = "the wish is kept, but no run was started: no `claude` program was found, and there \
                                  is no home directory to look in";
pub(super) const ASKS: &str = "a run asks, and waits for your answer on the page";
pub(super) const WAITS: &str = "a run's work waits for you on the page";

/// The day's runs, as its made wishes tell of them.
#[derive(Default)]
pub(super) struct Runs {
    going: usize,
    /// The first question a run asks.
    question: Option<String>,
    waits: bool,
}

impl Runs {
    /// What the jellyfish shows when nothing is heard: the first state of [`JellyState::ORDER`] the day's runs
    /// are in.
    pub(super) fn drawn(&self) -> Jelly {
        JellyState::ORDER.into_iter().find_map(|state| self.in_state(state)).unwrap_or_else(Jelly::quiet)
    }

    /// What is shown in `state`, when the day's runs are in it: a run that asks, runs going, work that waits.
    /// None for a state only a hearing knows.
    pub(super) fn in_state(&self, state: JellyState) -> Option<Jelly> {
        let blank = Jelly::quiet();
        match state {
            JellyState::Asks => {
                let question = Some(self.question.clone()?);
                Some(Jelly { state, says: ASKS.to_string(), question, ..blank })
            }
            JellyState::Working => (self.going > 0).then_some(Jelly { state, going: Some(self.going), ..blank }),
            JellyState::Waits => self.waits.then(|| Jelly { state, says: WAITS.to_string(), ..blank }),
            JellyState::Quiet => Some(blank),
            JellyState::Listening | JellyState::Heard | JellyState::Cannot => None,
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
        // A run that was named and has ended already, needing the person, is said as it ended, as `jellymoon
        // wish --run` prints it. One that finished is not a thing that could not be done: the day's rest shows it.
        Ok(started) if started.run.is_none() => Some(no_harness(&options).map_or(started.why, str::to_string)),
        Ok(started) => (started.state == WishState::NeedsYou).then_some(started.why),
        Err(e) => Some(e.to_string()),
    };
    Ok((made, unstarted))
}

/// The sentence for a run that was not started because there is no harness, asked as `run_apart` asks it: a
/// program passed over is said first, since that is what the person can mend. None when one was named or found.
fn no_harness(options: &RunOptions) -> Option<&'static str> {
    if options.harness.is_some() {
        return None;
    }
    let missed = find::search().ok()?.err()?;
    Some(match (missed.passed_over.is_empty(), missed.home) {
        (false, _) => PASSED_OVER,
        (true, true) => NO_HARNESS,
        (true, false) => NO_HOME,
    })
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
