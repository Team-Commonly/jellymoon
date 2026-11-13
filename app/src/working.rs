//! A tap on a little one opens its wish, and work that waits is shown, kept or tossed there (F47, ADR 0017,
//! decision 9). The rules and every word are core's: `jellymoon_core::waits` says whether a made wish's work
//! waits and what the note says of it, `work_tapped` is a tap on one of its choices, and `stands` is what is said
//! of any other wish. Here a tap is carried to core and core's answer is put in notes. Each call reads files and
//! asks git, and a keep or a toss writes to the repo, so each is work the window runs off its own thread; what the
//! window's thread does with the answer opens no file. No GPUI.
//!
//! A file of the work is a note's text and nothing else: it is never handed to a program, and this crate starts
//! none. The repo is not named here: core finds it as the run did, by the wish's project in the day's logs.

use jellymoon_core::{RunOptions, Waits, WorkDone, WorkTap};
use sticker_kit::{Note, Scene};

use crate::read::Page;
use crate::taps::OnShow;
use crate::today::Today;
use crate::wishing::Seen;

/// The starts of the ids of this file's notes, each followed by the wish's id: work that waits with its choices,
/// the same asked once more, what is being done, what was done, and a wish with no work waiting.
const WORK: &str = "work-";
const SURE: &str = "sure-";
const BUSY: &str = "busy-";
const SAID: &str = "said-";
const WISH: &str = "wish-";
/// The start of the id of a file's note: `file-3-m1` is file 3 of the work of the made wish `m1`.
const FILE: &str = "file-";

/// What core answered to a tap on a choice: the scene it was asked from, the scene to show, and the notes to say.
pub struct Worked {
    seen: Seen,
    wish: String,
    notes: Vec<Note>,
}

/// A tap on a choice carried to core: the window runs it off its own thread.
pub type Working = Box<dyn FnOnce() -> Worked + Send>;

fn text(id: String, title: &str, body: Vec<String>, options: Vec<&str>) -> Note {
    Note::Text { id, title: title.to_string(), body, options: options.into_iter().map(str::to_string).collect() }
}

fn id_of(note: &Note) -> Option<&str> {
    match note {
        Note::Text { id, .. } => Some(id),
        _ => None,
    }
}

/// Whether an id is one of this file's notes: a tap on such a note is never a tap for `taps.rs`.
pub(crate) fn names(id: &str) -> bool {
    [WORK, SURE, BUSY, SAID, WISH, FILE].iter().any(|start| id.starts_with(start))
}

/// Whether a note is one of this file's: a look and a reading leave it on show.
pub(crate) fn ours(note: &Note) -> bool {
    id_of(note).is_some_and(names)
}

/// The number of a file's note of `wish`.
fn file_of(note: &Note, wish: &str) -> Option<usize> {
    let (n, of) = id_of(note)?.strip_prefix(FILE)?.split_once('-')?;
    n.parse().ok().filter(|_| of == wish)
}

/// Whether a note is of `wish`: any of this file's but the one of a wish with no work waiting.
fn of(note: &Note, wish: &str) -> bool {
    let named = |start: &&str| id_of(note).and_then(|id| id.strip_prefix(*start)) == Some(wish);
    [WORK, SURE, BUSY, SAID].iter().any(named) || file_of(note, wish).is_some()
}

/// Whether core is at work for `wish`: one tap of a wish is carried at a time.
fn busy(scene: &Scene, wish: &str) -> bool {
    scene.notes.iter().any(|note| id_of(note).and_then(|id| id.strip_prefix(BUSY)) == Some(wish))
}

/// The note of work that waits, in core's words, with core's choices under it.
fn waiting(waits: &Waits, sure: bool) -> Note {
    if sure {
        let (title, body) = waits.asks();
        return text(format!("{SURE}{}", waits.wish), title, body, waits.choices(true));
    }
    text(format!("{WORK}{}", waits.wish), waits.title(), waits.lines(), waits.choices(false))
}

/// The notes that say what core answered to a tap on a choice of `wish`.
pub fn notes_of(wish: &str, done: WorkDone) -> Vec<Note> {
    match done {
        WorkDone::Shown(waits, file) => {
            vec![
                waiting(&waits, false),
                text(format!("{FILE}{}-{wish}", file.n), &file.title(), file.lines(), Vec::new()),
            ]
        }
        WorkDone::Asks(waits) => vec![waiting(&waits, true)],
        WorkDone::Said(title, body) => vec![text(format!("{SAID}{wish}"), title, vec![body], Vec::new())],
    }
}

/// `notes` in place of every note of `wish` on the scene, where the first of those was.
fn put(scene: &mut Scene, wish: &str, notes: Vec<Note>) {
    let at = scene.notes.iter().position(|note| of(note, wish)).unwrap_or(0);
    scene.notes.retain(|note| !of(note, wish));
    let at = at.min(scene.notes.len());
    scene.notes.splice(at..at, notes);
}

impl Today {
    /// What core is asked of a made wish's run: the day, and the logs the repo is found in by the wish's project.
    fn run_of(&self, wish: &str) -> RunOptions {
        RunOptions {
            wish: wish.to_string(),
            day: Some(self.date.clone()),
            repo: None,
            logs: Some(self.logs.clone()),
            harness: self.harness.clone(),
            skip: self.skip.clone(),
        }
    }

    /// `shown` with the note of every made wish whose work waits, first. `layout` prints these, as it has no tap:
    /// the window shows one after a tap on its wish's character.
    pub fn with_work(&self, mut shown: Scene) -> Scene {
        let waiting = jellymoon_core::waiting(&self.home, &self.run_of("")).map(|day| day.waits).unwrap_or_default();
        shown.notes.splice(0..0, waiting.iter().map(|waits| self::waiting(waits, false)));
        shown
    }

    /// A tap on the character `id` that stops no run: the note of its wish over the scene `from`, which is the
    /// work that waits with its choices, or else the wish's own words and how it stands. One wish is open at a
    /// time. None for a character that is no wish, for a wish core is at work for, and when the note is on show.
    pub(crate) fn opened(&self, page: Option<&Page>, from: Scene, id: &str) -> Option<Seen> {
        if busy(&from, id) {
            return None;
        }
        let note = match jellymoon_core::waits(&self.home, &self.run_of(id)) {
            Some(waits) => waiting(&waits, false),
            None => {
                let wishes = match page {
                    Some(page) => jellymoon_core::with_made(&self.home, page.data.clone()).wishes,
                    None => jellymoon_core::made(&self.home, &self.date),
                };
                let wish = wishes.iter().find(|wish| crate::scene::drawn_id(&wish.id) == id)?;
                let (title, body) = jellymoon_core::stands(wish);
                text(format!("{WISH}{id}"), title, body, Vec::new())
            }
        };
        let mut now = from.clone();
        // What core is at work on stays; every other note of an opened wish gives way to this one.
        now.notes.retain(|one| !ours(one) || id_of(one).is_some_and(|id| id.starts_with(BUSY)));
        now.notes.insert(0, note);
        (now != from).then_some(Seen { from, now })
    }
}

impl OnShow {
    /// A tap on option `index` (from 0) of the note `note`, when that is a choice of work that waits on show: the
    /// scene to show at once, which says what is being done in the note's place, and the tap itself as work for
    /// another thread. None for any other note and for an option that is not drawn. While core is at work for a
    /// wish its choices are not on show, so a second tap does nothing.
    pub fn working(&mut self, note: &str, index: usize) -> Option<(Scene, Working)> {
        let (sure, wish) = match (note.strip_prefix(WORK), note.strip_prefix(SURE)) {
            (Some(wish), _) => (false, wish.to_string()),
            (_, Some(wish)) => (true, wish.to_string()),
            _ => return None,
        };
        let drawn = |one: &Note| matches!(one, Note::Text { id, options, .. } if id == note && index < options.len());
        let at = self.scene.notes.iter().position(drawn)?;
        let after = self.scene.notes.iter().find_map(|one| file_of(one, &wish)).unwrap_or(0);
        let tap = WorkTap::of(index, sure, after)?;
        self.scene.notes[at] = text(format!("{BUSY}{wish}"), tap.doing(), Vec::new(), Vec::new());
        let (today, page, from) = (self.today.clone(), self.page.clone(), self.scene.clone());
        let work = move || {
            let done = jellymoon_core::work_tapped(&today.home, &today.run_of(&wish), tap);
            let notes = notes_of(&wish, done);
            let mut now = today.again(page.as_ref(), &from);
            put(&mut now, &wish, notes.clone());
            Worked { seen: Seen { from, now }, wish, notes }
        };
        Some((self.scene.clone(), Box::new(work)))
    }

    /// What a [`Working`] gave: the scene to show. When the scene on show changed while core was at work, only
    /// the notes are put on it here, and the next look draws the wish.
    pub fn worked(&mut self, worked: Worked) -> Scene {
        if self.scene == worked.seen.from {
            self.scene = worked.seen.now;
        } else {
            put(&mut self.scene, &worked.wish, worked.notes);
        }
        self.scene.clone()
    }
}
