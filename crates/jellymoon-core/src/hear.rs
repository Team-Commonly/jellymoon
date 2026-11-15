//! A wish said aloud (F55, ADR 0019, decisions 5 and 6; `docs/development/hear.md`): what is heard while the key
//! is held, the four seconds a heard wish is shown, and the wish it is then made. A [`Hearing`] is given events
//! ([`HearEvent`]: the key down and up, what the ear said, and the time as `tick`) and says what the jellyfish
//! shows ([`Jelly`]) whenever that changes. [`hear`] is the same over lines of JSON, for `jellymoon hear`.
//!
//! Time is data: the only clock is the `tick` events, so a test waits for nothing. Core registers no key and
//! opens no ear. What the ear said is untrusted text: the final words pass the door a typed wish passes
//! (`still.rs`), and the words so far are made as a statement is, and only ever shown. A heard wish is kept
//! nowhere: nothing is written until its window has passed, and then the wish is made by the call the page's
//! entry uses ([`wish_offered`](crate::wish_offered)), once. Events out of order change nothing, and a line that
//! is no event is counted and passed over.

mod day;
mod drawn;
mod shapes;
mod voice;

use std::io::{BufRead, Write};
use std::path::PathBuf;

pub use drawn::{JellyDrawn, JellyText, Whose, jellyfish};
pub use shapes::{EarCannot, HearEvent, Jelly, JellyState, KeyMove, Said};
pub use voice::{Voice, voice};

use crate::still::{MARKS, Unkept, marks_only, statement};
use crate::{DayOptions, Error, MADE_SAID_MAX, RunOptions, session, wish};

/// A hold shorter than this is a slip of the hand: nothing is heard.
pub const SLIP_MS: u64 = 300;
/// How long a heard wish is shown before it is made a wish. A key pressed before then takes it back.
pub const WINDOW_MS: u64 = 4000;
/// How long something that was refused is said.
pub const CANNOT_MS: u64 = 6000;
/// A hold ends by itself here, which is the recognizer's own limit.
pub const HOLD_MS: u64 = 60_000;
/// How long the ear's final words are waited for once a hold is over.
pub const EAR_MS: u64 = 5000;
/// How much time passes, at least, between two looks at the day's made wishes and their runs.
pub const LOOK_MS: u64 = 1000;
/// The most bytes of one line of events, its line break among them. A longer line is read past.
pub const HEAR_LINE_MAX: usize = 64 * 1024;
/// The most times one hold's words so far are changed. What the ear says of them after that is passed over.
pub const SAID_SO_FAR_MAX: usize = 4096;

/// What a heard wish says with its words.
const TAKE_BACK: &str = "press the key again to take it back";
/// What is said when the ear gave no final words in [`EAR_MS`].
const EAR_SILENT: &str = "nothing was heard: the ear did not say what it heard";

/// One hearing: the day it makes wishes on, and how a made wish's run is started.
#[derive(Debug, Clone, Default)]
pub struct HearOptions {
    /// The day's session logs, the day, the names switched off and Jellymoon's home, as the page's entry reads
    /// them. A day that is None is today, read once when the hearing begins.
    pub read: DayOptions,
    /// With a run: the `jellymoon` command, by its full path, and the options of the run but for its wish and
    /// day, as `jellymoon wish --run` gives them to [`run_apart`](crate::run_apart). None starts no run.
    pub apart: Option<(PathBuf, RunOptions)>,
}

/// When a span that an event began is counted from: the first tick at or after the event, which is None until
/// that tick comes. The time of the tick before the event is never used: it may be long past, and a span counted
/// from it would be shorter than what the person saw, down to nothing.
type Since = Option<u64>;

/// How much of a span has passed at `now`, a tick's time. The first tick asked starts the span.
fn passed(since: &mut Since, now: u64) -> u64 {
    now - *since.get_or_insert(now)
}

/// Where a hold stands.
enum Hold {
    None,
    /// The key is down since `since`, the time of the tick before it: a hold's length is the time between the
    /// ticks before its two key events. `said` counts the times the words so far were changed.
    Held {
        since: u64,
        so_far: String,
        said: usize,
    },
    /// The hold is over since `since`, and the ear's final words are waited for.
    Over {
        since: Since,
        so_far: String,
        said: usize,
    },
    /// A heard wish, shown since `since`.
    Heard {
        words: String,
        project: String,
        since: Since,
    },
}

/// A hearing in progress. Nothing here reads a clock or writes a file but the making of a wish.
pub struct Hearing {
    read: DayOptions,
    home: PathBuf,
    day: String,
    apart: Option<(PathBuf, RunOptions)>,
    /// The first tick, which starts the clock, and the milliseconds since.
    origin: Option<u64>,
    now: u64,
    hold: Hold,
    /// What was refused, and since when it is said.
    cannot: Option<(String, Since)>,
    runs: day::Runs,
    looked: u64,
    /// The wish just made, for the next line to name.
    made: Option<String>,
    shown: Jelly,
    skipped: usize,
}

impl Hearing {
    /// Begins a hearing. It reads the day's made wishes once, and writes nothing. Refused: no home, or one that
    /// is not a full path, and a day that is none.
    pub fn begin(options: HearOptions) -> Result<Hearing, Error> {
        let home = options.read.home.clone().filter(|home| home.is_absolute()).ok_or(Error::Home)?;
        let day = wish::day_of(options.read.date.as_deref())?;
        let read = DayOptions { date: Some(day.clone()), ..options.read };
        let (runs, apart) = (day::look(&home, &day), options.apart);
        let (hold, shown) = (Hold::None, runs.drawn());
        let (origin, now, looked, cannot, made, skipped) = (None, 0, 0, None, None, 0);
        Ok(Hearing { read, home, day, apart, origin, now, hold, cannot, runs, looked, made, shown, skipped })
    }

    /// What the jellyfish shows now.
    pub fn shows(&self) -> &Jelly {
        &self.shown
    }

    /// How many lines given to [`Hearing::line`] were no event.
    pub fn skipped(&self) -> usize {
        self.skipped
    }

    /// One line of events. A line that is not one event, whole, is counted and changes nothing: text that is
    /// not JSON, a kind or a name this build does not know, a number where words belong. An empty line is none.
    pub fn line(&mut self, line: &[u8]) -> Option<Jelly> {
        let line = line.trim_ascii();
        if line.is_empty() {
            return None;
        }
        match serde_json::from_slice::<HearEvent>(line) {
            Ok(event) => self.event(event),
            Err(_) => {
                self.skipped += 1;
                None
            }
        }
    }

    /// One event. Some when what the jellyfish shows changed: the line to print. An event out of order (a key
    /// up with no down, a second down, the ear's words with no hold, a tick that goes backwards) changes nothing.
    pub fn event(&mut self, event: HearEvent) -> Option<Jelly> {
        match event {
            HearEvent::Tick { tick } => self.tick(tick),
            HearEvent::Key { key: KeyMove::Down } => self.down(),
            HearEvent::Key { key: KeyMove::Up } => self.up(),
            HearEvent::Said(said) => self.said(said),
        }
        let drawn = self.drawn();
        if drawn == self.shown {
            return None;
        }
        self.shown = drawn.clone();
        Some(Jelly { made: self.made.take(), ..drawn })
    }

    /// The first tick starts the clock, so nothing has passed at it whatever its number. A later one moves the
    /// time forward to it, and one that would move it back is passed over. Every span an event began (a heard
    /// wish's window, a refusal's time, the wait for the ear) starts at the first tick at or after that event,
    /// which may be one at the time of the last.
    fn tick(&mut self, tick: u64) {
        let now = tick.saturating_sub(*self.origin.get_or_insert(tick));
        if now < self.now {
            return;
        }
        self.now = now;
        self.cannot.take_if(|(_, since)| passed(since, now) >= CANNOT_MS);
        let mut hold = std::mem::replace(&mut self.hold, Hold::None);
        let over = match &mut hold {
            Hold::None => false,
            Hold::Held { since, .. } => now - *since >= HOLD_MS,
            Hold::Over { since, .. } => passed(since, now) >= EAR_MS,
            Hold::Heard { since, .. } => passed(since, now) >= WINDOW_MS,
        };
        match hold {
            // This tick is the one the hold ended at, so the wait for the ear starts at it.
            Hold::Held { so_far, said, .. } if over => self.hold = Hold::Over { since: Some(now), so_far, said },
            Hold::Over { .. } if over => self.refuse(EAR_SILENT, Some(now)),
            Hold::Heard { words, project, .. } if over => self.make(&words, &project),
            hold => self.hold = hold,
        }
        if now - self.looked >= LOOK_MS {
            self.look();
        }
    }

    /// A key pressed takes a heard wish back and drops a hold that was over: the ear listens anew.
    fn down(&mut self) {
        if !matches!(self.hold, Hold::Held { .. }) {
            self.hold = Hold::Held { since: self.now, so_far: String::new(), said: 0 };
        }
    }

    fn up(&mut self) {
        if let Hold::Held { since, .. } = self.hold {
            let Hold::Held { so_far, said, .. } = std::mem::replace(&mut self.hold, Hold::None) else { return };
            if self.now - since >= SLIP_MS {
                self.hold = Hold::Over { since: None, so_far, said };
            }
        }
    }

    /// What the ear said. Its words so far count while a hold is on or just over; its final words only once the
    /// hold is over, so a final that comes while the key is held is an earlier hold's, and is passed over. What
    /// it refuses is said whenever it comes, ends a hold, and leaves a heard wish as it is.
    fn said(&mut self, said: Said) {
        match (said, &mut self.hold) {
            (Said::Partial { words }, Hold::Held { so_far, said, .. } | Hold::Over { so_far, said, .. })
                if *said < SAID_SO_FAR_MAX =>
            {
                (*so_far, *said) = (statement(&words, MADE_SAID_MAX), *said + 1);
            }
            (Said::Final { words }, Hold::Over { .. }) => {
                self.hold = Hold::None;
                self.heard(&words);
            }
            (Said::Cannot { why }, hold) => {
                if matches!(hold, Hold::Held { .. } | Hold::Over { .. }) {
                    *hold = Hold::None;
                }
                self.refuse(why.says(), None);
            }
            _ => {}
        }
    }

    /// The final words of a hold, through the door. Words the door refuses are refused in a wish's own line;
    /// words that are empty once cleaned are nothing at all. Nothing is written.
    fn heard(&mut self, words: &str) {
        let refused = match wish::cleaned(words) {
            Ok(words) if words.is_empty() => return,
            Ok(words) if marks_only(&words) => MARKS,
            Ok(words) => match day::project(&self.read, &self.home, &self.day) {
                Some(project) => {
                    self.hold = Hold::Heard { words, project, since: None };
                    return;
                }
                None => day::NO_PROJECT,
            },
            Err(Unkept::Long) => wish::LONG,
            Err(Unkept::Changing) => wish::CHANGING,
        };
        self.refuse(refused, None);
    }

    /// The window has passed, at a tick: the heard wish is made a wish, and with a run asked for, its run is
    /// started. What is refused here is said from this tick.
    fn make(&mut self, words: &str, project: &str) {
        match day::make(&self.read, &self.home, self.apart.as_ref(), words, project) {
            Ok((made, unstarted)) => {
                self.made = Some(made);
                if let Some(why) = unstarted {
                    self.refuse(&why, Some(self.now));
                }
            }
            Err(unkept) => self.refuse(&unkept, Some(self.now)),
        }
        self.look();
    }

    /// `since`: the tick that refused it, or None when an event did, and the next tick starts its time.
    fn refuse(&mut self, why: &str, since: Since) {
        self.cannot = Some((why.to_string(), since));
    }

    fn look(&mut self) {
        (self.runs, self.looked) = (day::look(&self.home, &self.day), self.now);
    }

    /// What is shown: the first state of [`JellyState::ORDER`] that holds (ADR 0019, decision 4).
    fn drawn(&self) -> Jelly {
        JellyState::ORDER.into_iter().find_map(|state| self.in_state(state)).unwrap_or_else(Jelly::quiet)
    }

    /// What is shown in `state`, when the hearing, or the day's runs, are in it.
    fn in_state(&self, state: JellyState) -> Option<Jelly> {
        let blank = Jelly::quiet();
        match (state, &self.hold, &self.cannot) {
            (JellyState::Listening, Hold::Held { so_far, .. } | Hold::Over { so_far, .. }, _) => {
                Some(Jelly { state, words: so_far.clone(), ..blank })
            }
            (JellyState::Heard, Hold::Heard { words, project, since }, _) => Some(Jelly {
                state,
                says: TAKE_BACK.to_string(),
                words: words.clone(),
                project: project.clone(),
                left: WINDOW_MS.saturating_sub(since.map_or(0, |since| self.now - since)),
                ..blank
            }),
            (JellyState::Cannot, _, Some((why, _))) => Some(Jelly { state, says: why.clone(), ..blank }),
            (JellyState::Listening | JellyState::Heard | JellyState::Cannot, ..) => None,
            _ => self.runs.in_state(state),
        }
    }

    /// Every sentence of a hearing's own that it refuses with: the ear's, and its own for words that came to
    /// nothing. A refusal of a wish or of a run that is passed on in that command's words is not among them.
    pub fn refusals() -> Vec<&'static str> {
        let own = [EAR_SILENT, MARKS, day::NO_PROJECT, day::NO_HARNESS, wish::LONG, wish::CHANGING];
        EarCannot::ALL.iter().map(|why| why.says()).chain(own).collect()
    }
}

/// `jellymoon hear`: reads events from `input`, a line each, and prints to `output` what the jellyfish shows,
/// first as the hearing begins and then at each change, one line of JSON each. It ends with the input. A line
/// over [`HEAR_LINE_MAX`] bytes is read past and counted with the lines that are no event; a line may end in
/// `\r\n`. A heard wish whose window has not passed when the input ends is dropped, and nothing is kept of it.
/// Some: what to say of the lines that were passed over, with nothing of their text in it.
pub fn hear(options: HearOptions, mut input: impl BufRead, mut output: impl Write) -> Result<Option<String>, Error> {
    let mut hearing = Hearing::begin(options)?;
    let unread = |e| Error::Hear(format!("the events could not be read: {e}"));
    let mut print = |jelly: &Jelly| {
        let printed = writeln!(output, "{jelly}").and_then(|()| output.flush());
        printed.map_err(|e| Error::Hear(format!("what the jellyfish shows could not be printed: {e}")))
    };
    print(hearing.shows())?;
    let mut line = Vec::new();
    while session::next_line(&mut input, HEAR_LINE_MAX, &mut line).map_err(unread)? {
        // A line that was read past comes back with nothing in it, not even its line break. The input's last
        // line with no line break is held to the length a line has without one.
        if line.is_empty() || (line.len() == HEAR_LINE_MAX && !line.ends_with(b"\n")) {
            hearing.skipped += 1;
        } else if let Some(jelly) = hearing.line(&line) {
            print(&jelly)?;
        }
    }
    let skipped = hearing.skipped();
    Ok((skipped > 0).then(|| format!("lines of the input that were no event, and were passed over: {skipped}")))
}
