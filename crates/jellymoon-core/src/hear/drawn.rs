//! What is drawn of the jellyfish (F56, ADR 0019, "Found while building F56"): which one state it is in when a
//! hearing's line and the day's made wishes each give one ([`jellyfish`]), whether a line is one a hearing prints
//! ([`Jelly::read`]), and what its bubble holds ([`Jelly::drawn`]). The shell turns a [`JellyDrawn`] into the
//! kit's pieces and decides nothing. `tests/fixtures/jelly-drawn.jsonl` is its example.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{HEAR_LINE_MAX, Jelly, JellyState, TAKE_BACK, WINDOW_MS, day};
use crate::{Error, wish};

impl JellyState {
    /// When more than one state holds, the first of these is shown (ADR 0019, decision 4).
    pub const ORDER: [JellyState; 7] = [
        JellyState::Listening,
        JellyState::Heard,
        JellyState::Cannot,
        JellyState::Asks,
        JellyState::Working,
        JellyState::Waits,
        JellyState::Quiet,
    ];

    /// Whether only a hearing knows of this state: the key, the ear and what was refused. The other four are
    /// what a day's made wishes give, with a hearing or with none.
    pub fn of_a_hearing(self) -> bool {
        let before = |state: JellyState| JellyState::ORDER.iter().position(|at| *at == state);
        before(self) < before(JellyState::Asks)
    }
}

/// Whose words a text of the bubble is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Whose {
    /// The person's own: the words so far, and a heard wish.
    Yours,
    /// The name of the project a heard wish will be made for.
    Project,
    /// A run's question: an agent's text, drawn apart from Jellymoon's own words.
    Run,
    /// One of core's own sentences.
    Jellymoon,
}

/// One text of the bubble. It is drawn as text and as nothing else.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JellyText {
    pub whose: Whose,
    pub text: String,
}

/// What is drawn of the jellyfish for one line of a hearing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JellyDrawn {
    pub state: JellyState,
    /// How many runs go, drawn on the jellyfish itself when more than one does. Left out otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
    /// The bubble's texts, in the order they are drawn. With none there is no bubble.
    pub bubble: Vec<JellyText>,
    /// How much of a heard wish's time is left, in thousandths: 1000 is the whole of it. Only with `heard`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ring: Option<u16>,
}

impl Jelly {
    /// The line `bytes` hold, when they are one line a hearing prints and nothing else: at most
    /// [`HEAR_LINE_MAX`] bytes, one object on one line with the names a line has, and in each state only what
    /// that state carries. Any other text is no line.
    pub fn read(bytes: &[u8]) -> Option<Jelly> {
        let line = bytes.trim_ascii();
        if bytes.len() > HEAR_LINE_MAX || line.contains(&b'\n') {
            return None;
        }
        serde_json::from_slice::<Jelly>(line).ok().filter(Jelly::is_printed)
    }

    /// Whether a hearing could have printed this: its sentence is the state's own, and words, a project, a
    /// time, a count and a question are with the states that have them and with no other.
    fn is_printed(&self) -> bool {
        use JellyState::{Asks, Cannot, Heard, Listening, Quiet, Waits, Working};
        let says = match self.state {
            Quiet | Listening | Working => "",
            Heard => TAKE_BACK,
            Asks => day::ASKS,
            Waits => day::WAITS,
            Cannot => return self.bare() && self.words.is_empty() && !self.says.is_empty(),
        };
        let heard = self.state == Heard;
        let words = match self.state {
            Heard => !self.words.is_empty(),
            Listening => true,
            _ => self.words.is_empty(),
        };
        self.says == says
            && words
            && heard != self.project.is_empty()
            && (if heard { self.left <= WINDOW_MS } else { self.left == 0 })
            && (self.state == Working) == self.going.is_some()
            && self.going != Some(0)
            && (self.state == Asks) == self.question.is_some()
    }

    /// With no project, no time, no count and no question.
    fn bare(&self) -> bool {
        self.project.is_empty() && self.left == 0 && self.going.is_none() && self.question.is_none()
    }

    /// What is drawn of this line. The bubble holds the words so far while they are any; a heard wish's words,
    /// its project and how to take it back, with the ring; a run's question under the sentence that says a run
    /// asks; and what was refused. `quiet`, `working` and `waits` have no bubble: the sentence a line has for
    /// work that waits is the page's to say.
    pub fn drawn(&self) -> JellyDrawn {
        let text = |whose, text: &str| (!text.is_empty()).then(|| JellyText { whose, text: text.to_string() });
        let own = text(Whose::Jellymoon, &self.says);
        let bubble = match self.state {
            JellyState::Quiet | JellyState::Working | JellyState::Waits => Vec::new(),
            JellyState::Listening => vec![text(Whose::Yours, &self.words)],
            JellyState::Heard => vec![text(Whose::Yours, &self.words), text(Whose::Project, &self.project), own],
            JellyState::Asks => vec![own, text(Whose::Run, self.question.as_deref().unwrap_or_default())],
            JellyState::Cannot => vec![own],
        };
        let left = u16::try_from(self.left.min(WINDOW_MS) * 1000 / WINDOW_MS).unwrap_or(1000);
        JellyDrawn {
            state: self.state,
            count: self.going.filter(|going| self.state == JellyState::Working && *going > 1),
            bubble: bubble.into_iter().flatten().collect(),
            ring: (self.state == JellyState::Heard).then_some(left),
        }
    }
}

/// The line that is shown of the two, by [`JellyState::ORDER`]: the hearing's while it is in a state of its
/// own, and else what the day's made wishes give now, which is newer than what the hearing last saw of them.
fn first(line: Option<&Jelly>, day: Jelly) -> Jelly {
    match line {
        Some(line) if line.state.of_a_hearing() => line.clone(),
        line => Jelly { made: line.and_then(|line| line.made.clone()), ..day },
    }
}

/// The one state of the jellyfish, from a hearing's line and the made wishes of `day` (today when none is
/// named), read now in Jellymoon's home; with no line, from the made wishes alone, which is what is shown with
/// no hearing at all. It reads, and writes nothing. Refused: a home that is not a full path, a day that is none.
pub fn jellyfish(line: Option<&Jelly>, home: &Path, day: Option<&str>) -> Result<Jelly, Error> {
    if !home.is_absolute() {
        return Err(Error::Home);
    }
    Ok(first(line, day::look(home, &wish::day_of(day)?).drawn()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn of(state: JellyState) -> Jelly {
        Jelly { state, ..Jelly::quiet() }
    }

    // ADR 0019, decision 4: listening, heard, cannot, asks, working, waits, quiet.
    #[test]
    fn of_a_line_and_a_day_the_first_in_core_s_order_is_shown_and_a_day_s_state_is_the_day_s_own() {
        use JellyState::{Asks, Cannot, Heard, Listening, Quiet, Waits, Working};
        assert_eq!(JellyState::ORDER, [Listening, Heard, Cannot, Asks, Working, Waits, Quiet]);
        for (at, line) in JellyState::ORDER.into_iter().enumerate() {
            assert_eq!(line.of_a_hearing(), at < 3, "{line:?}");
            for day in [Asks, Working, Waits, Quiet] {
                let shown = first(Some(&of(line)), of(day)).state;
                assert_eq!(shown, if at < 3 { line } else { day }, "{line:?} over a day that gives {day:?}");
                assert_eq!(first(None, of(day)).state, day);
            }
        }
        // The wish a line names as just made is still named when the day's state is drawn in the line's place.
        let made = Jelly { made: Some("m2".to_string()), ..of(Quiet) };
        assert_eq!(first(Some(&made), of(Working)), Jelly { made: Some("m2".to_string()), ..of(Working) });
    }

    #[test]
    fn a_ring_is_the_share_of_the_four_seconds_left_and_a_count_is_of_more_than_one_run() {
        let ring = |left| Jelly { left, ..of(JellyState::Heard) }.drawn().ring;
        assert_eq!(
            [ring(4000), ring(3999), ring(2000), ring(4), ring(3), ring(0)].map(Option::unwrap),
            [1000, 999, 500, 1, 0, 0]
        );
        assert_eq!(ring(u64::MAX), Some(1000), "a time longer than the four seconds is a full ring");
        let count = |state, going| Jelly { going: Some(going), ..of(state) }.drawn().count;
        assert_eq!([count(JellyState::Working, 1), count(JellyState::Working, 2)], [None, Some(2)]);
        assert_eq!(count(JellyState::Asks, 2), None);
    }
}
