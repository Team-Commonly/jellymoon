//! A tap on a little one opens its wish, and work that waits is shown, kept or tossed there (F47, ADR 0017,
//! decision 9). The rules are core's, and every word but the result's note's, which are the app's (`told.rs`,
//! ADR 0023, "Found while building F86"): `jellymoon_core::waits` says whether a made wish's work
//! waits and what the note says of it, `work_tapped` is a tap on one of its choices, and `stands` is what is said
//! of any other wish. Here a tap is carried to core and core's answer is put in notes. Each call reads files and
//! asks git, and a keep or a toss writes to the repo, so each is work the window runs off its own thread; what the
//! window's thread does with the answer opens no file. No GPUI.
//!
//! A file of the work is a note's text and nothing else: it is never handed to a program, and this crate starts
//! none. The repo is not named here: core finds it as the run did, by the wish's project in the day's logs.

use std::panic::{self, AssertUnwindSafe};

use jellymoon_core::{Alone, RunOptions, Waits, Wish, WishState, WorkDone, WorkTap};
use sticker_kit::{Note, Scene};

use crate::answering::{with_go_on, with_more, with_question};
use crate::read::Page;
use crate::taps::OnShow;
use crate::today::Today;
use crate::told::{TOLD, told, told_alone};
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

/// The call to core a tap is carried by: `jellymoon_core::work_tapped`, of the wish's run.
pub trait Ask: FnOnce(&Today, &str, WorkTap) -> WorkDone + Send + 'static {}
impl<A: FnOnce(&Today, &str, WorkTap) -> WorkDone + Send + 'static> Ask for A {}

fn text(id: String, title: &str, body: Vec<String>, options: Vec<&str>) -> Note {
    let options = options.into_iter().map(str::to_string).collect();
    Note::Text { id, title: title.to_string(), body, options, quoted: String::new(), field: None }
}

fn id_of(note: &Note) -> Option<&str> {
    match note {
        Note::Text { id, .. } | Note::Told { id, .. } => Some(id),
        _ => None,
    }
}

/// Whether an id is one of this file's notes: a tap on such a note is never a tap for `taps.rs`.
pub(crate) fn names(id: &str) -> bool {
    [WORK, SURE, BUSY, SAID, WISH, FILE, TOLD].iter().any(|start| id.starts_with(start))
}

/// Whether a note is one of this file's: a look and a reading leave it on show.
pub(crate) fn ours(note: &Note) -> bool {
    id_of(note).is_some_and(names)
}

/// Whether a scene holds a note a person may be about to tap or type in, so nothing new is put over it: a
/// choice with an option (a clash's, the one that turns delivery on, a stop's); a text note with an option or
/// a field (work that waits, the same asked once more, a question, a follow-up); or a note core's answer comes
/// in the place of (what is being done with work, a run being stopped, a wish that is being worked on, which is
/// also the note of a wish whose answer is with its run). It is the scene that is asked, placed or not.
pub(crate) fn in_hand(scene: &Scene) -> bool {
    scene.notes.iter().any(|note| match note {
        Note::Choice { options, .. } => !options.is_empty(),
        Note::Text { id, options, field, .. } => {
            !options.is_empty() || field.is_some() || id.starts_with(BUSY) || at_work(note) == *note
        }
        Note::Plain { title, .. } => title == crate::stopping::STOPPING,
        _ => false,
    })
}

/// The wishes whose work that waits, or what their run says it did, is on a scene.
pub(crate) fn shown(scene: &Scene) -> impl Iterator<Item = &str> {
    scene.notes.iter().filter_map(|note| id_of(note).and_then(|id| id.strip_prefix(WORK).or(id.strip_prefix(TOLD))))
}

/// Whether a scene holds what a run says it did with no work that waits under it (told alone): a note with
/// nothing to choose, which the looks after it leave on the page as they leave a choice's answer.
pub(crate) fn told_on(scene: &Scene) -> bool {
    let has = |start: &str| scene.notes.iter().filter_map(id_of).any(|id| id.starts_with(start));
    has(TOLD) && !has(WORK)
}

/// `notes` over the scene `from`, as the notes of an opened wish go: what core is at work on stays, and every
/// other note of an opened wish gives way to these. With `said`, what was done with work stays too: the answer
/// to a person's own tap is not taken away by a look.
fn over(from: &Scene, notes: Vec<Note>, said: bool) -> Scene {
    let stays = |id: &str| id.starts_with(BUSY) || said && id.starts_with(SAID);
    let mut now = from.clone();
    now.notes.retain(|one| !ours(one) || id_of(one).is_some_and(stays));
    now.notes.splice(0..0, notes);
    now
}

/// The number of a file's note of `wish`.
fn file_of(note: &Note, wish: &str) -> Option<usize> {
    let (n, of) = id_of(note)?.strip_prefix(FILE)?.split_once('-')?;
    n.parse().ok().filter(|_| of == wish)
}

/// Whether a note is of `wish`: any of this file's but the one of a wish with no work waiting.
fn of(note: &Note, wish: &str) -> bool {
    let named = |start: &&str| id_of(note).and_then(|id| id.strip_prefix(*start)) == Some(wish);
    [WORK, SURE, BUSY, SAID, TOLD].iter().any(named) || file_of(note, wish).is_some()
}

/// The wish a note of work that waits, or of how a wish stands, is of.
pub(crate) fn wish_of(id: &str) -> Option<&str> {
    id.strip_prefix(WORK).or_else(|| id.strip_prefix(WISH))
}

/// The wish a note with a field can be of: its work, what is being done with that work, or how it stands.
pub(crate) fn typed_under(id: &str) -> Option<&str> {
    wish_of(id).or_else(|| id.strip_prefix(BUSY))
}

/// `note` in place of the note of `wish` that is its work or how it stands, when one is on show. When none is,
/// the scene is left as it is and the note is given back.
pub(crate) fn reopen(scene: &mut Scene, wish: &str, note: Note) -> Option<Note> {
    let is = |one: &Note| id_of(one).and_then(wish_of) == Some(wish);
    match scene.notes.iter().position(is) {
        Some(at) => scene.notes[at] = note,
        None => return Some(note),
    }
    None
}

/// How a made wish with no work waiting stands, in core's words, with the question its run asks. `alone` is how
/// its run ended, when it left no work.
fn standing(wish: &Wish, alone: Option<&Alone>) -> Note {
    let (title, body) = jellymoon_core::stands(wish, alone);
    let id = crate::scene::drawn_id(&wish.id);
    let note = text(format!("{WISH}{id}"), title, body, Vec::new());
    with_question(note, &id, wish.question.as_deref())
}

/// The note of a wish whose run goes on with an answer, in the place of `asked`, the note the answer was typed
/// under: what core says of a wish that is being worked on, over the wish's own words, which `asked` holds. It
/// has no question and no field.
pub(crate) fn at_work(asked: &Note) -> Note {
    let Note::Text { id, body, .. } = asked else { return asked.clone() };
    let (none, state) = (String::new, WishState::Working);
    let wish =
        Wish { id: none(), label: none(), said: none(), project: none(), state, stars: Vec::new(), question: None };
    text(id.clone(), jellymoon_core::stands(&wish, None).0, body.clone(), Vec::new())
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

/// The notes of work that waits: what its run says it did, when core printed a piece of it, and the work's own
/// note, with core's choices, after it.
pub fn waiting_notes(waits: &Waits) -> Vec<Note> {
    told(waits).into_iter().chain([waiting(waits, false)]).collect()
}

/// The notes that say what core answered to a tap on a choice of `wish`.
pub fn notes_of(wish: &str, done: WorkDone) -> Vec<Note> {
    match done {
        WorkDone::Shown(waits, file) => {
            let file = text(format!("{FILE}{}-{wish}", file.n), &file.title(), file.lines(), Vec::new());
            waiting_notes(&waits).into_iter().chain([file]).collect()
        }
        WorkDone::Asks(waits) => vec![waiting(&waits, true)],
        WorkDone::Said(title, body) => vec![text(format!("{SAID}{wish}"), title, body, Vec::new())],
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
    pub(crate) fn run_of(&self, wish: &str) -> RunOptions {
        RunOptions {
            wish: wish.to_string(),
            day: Some(self.date.clone()),
            repo: None,
            logs: Some(self.logs.clone()),
            harness: self.harness.clone(),
            skip: self.skip.clone(),
        }
    }

    /// `shown` with the note of every made wish whose work waits, first, and then of every one whose run asks
    /// or whose work was kept: the wishes a person may type to (`answering.rs`). `layout` prints these, as it has
    /// no tap: the window shows one after a tap on its wish's character.
    pub fn with_work(&self, mut shown: Scene) -> Scene {
        let waits = jellymoon_core::waiting(&self.home, &self.run_of("")).map(|day| day.waits).unwrap_or_default();
        let work = |waits: &Waits| {
            let work = with_more(waiting(waits, false), &waits.wish, self.follows(&waits.wish));
            told(waits).into_iter().chain([work])
        };
        let mut notes: Vec<Note> = waits.iter().flat_map(work).collect();
        let made = jellymoon_core::made(&self.home, &self.date);
        let stands = |wish: &Wish| {
            let follows = self.follows(&wish.id);
            let typed_to = (wish.question.is_some() || follows) && !waits.iter().any(|waits| waits.wish == wish.id);
            typed_to.then(|| with_more(standing(wish, None), &wish.id, follows))
        };
        notes.extend(made.iter().filter_map(stands));
        shown.notes.splice(0..0, notes);
        shown
    }

    /// Whether core says a follow-up of its work may be made of the made wish `wish` (`jellymoon_core::followable`).
    fn follows(&self, wish: &str) -> bool {
        jellymoon_core::followable(&self.home, &self.date, true).iter().any(|one| one == wish)
    }

    /// Whether core says a follow-up may be made of the made wish `wish`, whose run changed nothing.
    fn goes_on(&self, wish: &str) -> bool {
        jellymoon_core::followable(&self.home, &self.date, false).iter().any(|one| one == wish)
    }

    /// The note of the made wish `id` as core says the wish stands now: its work that waits with its choices,
    /// or else its own words and how it stands, with the question its run asks when it is a made wish. Under
    /// either, the offer of a follow-up when core says one may be made of the wish. None for an id that is no
    /// wish.
    pub(crate) fn note_of(&self, page: Option<&Page>, id: &str) -> Option<Note> {
        self.notes_on_open(page, id).and_then(|mut notes| notes.pop())
    }

    /// The notes of the work of the made wish `id` that waits: what its run says it did, then the work with its
    /// choices and the offer of a follow-up. None when no work of it waits.
    fn work_notes(&self, id: &str) -> Option<Vec<Note>> {
        let waits = jellymoon_core::waits(&self.home, &self.run_of(id))?;
        let work = with_more(waiting(&waits, false), id, self.follows(id));
        Some(told(&waits).into_iter().chain([work]).collect())
    }

    /// [`Today::note_of`], with what the run says it did before the note of its work that waits.
    fn notes_on_open(&self, page: Option<&Page>, id: &str) -> Option<Vec<Note>> {
        if let Some(notes) = self.work_notes(id) {
            return Some(notes);
        }
        // The day's made wishes stand after the page's own, as they are drawn (`taps.rs`).
        let (wishes, read) = match page {
            Some(page) => (jellymoon_core::with_made(&self.home, page.data.clone()).wishes, page.data.wishes.len()),
            None => (jellymoon_core::made(&self.home, &self.date), 0),
        };
        let at = wishes.iter().position(|wish| crate::scene::drawn_id(&wish.id) == id)?;
        // A made wish's run that left no work says how it ended: what it says it did, over the wish's note.
        let made = at >= read;
        let alone = made.then(|| jellymoon_core::ended_alone(&self.home, &self.run_of(id))).flatten();
        let told = alone.as_ref().and_then(|alone| told_alone(id, alone));
        let note = if made { standing(&wishes[at], alone.as_ref()) } else { self.read_note(&wishes[at]) };
        let note = with_go_on(with_more(note, id, self.follows(id)), id, made && self.goes_on(id));
        Some(told.into_iter().chain([note]).collect())
    }

    /// The note of a wish of the reading: Jellymoon ran nothing for it, so it says what the reading took it for
    /// and how to go on (`scene::read_said`). A question a kept page file gives it is not drawn, and gets no field.
    fn read_note(&self, wish: &Wish) -> Note {
        let chip = jellymoon_core::offered(&self.read()).contains(&wish.project);
        let (title, body) = crate::scene::read_said(wish, chip);
        text(format!("{WISH}{}", crate::scene::drawn_id(&wish.id)), title, body, Vec::new())
    }

    /// The work of the made wish `id` that waits, or what its run that changed nothing says it did, over the scene
    /// `from`, as a tap on its character shows it ([`Today::opened`]), but over what was said of work kept or
    /// tossed, which a tap takes away: for a look that found the wish's run ended (`showing.rs`). The calls to
    /// core are a tap's. None when neither is; with the scene, whether it is work that waits.
    pub(crate) fn waiting_over(&self, page: Option<&Page>, from: &Scene, id: &str) -> Option<(Scene, bool)> {
        let notes = self.notes_on_open(page, id)?;
        let has = |start: &str| notes.iter().filter_map(id_of).any(|one| one.starts_with(start));
        let (work, told) = (has(WORK), has(TOLD));
        (work || told).then(|| (over(from, notes, true), work))
    }

    /// `notes` of `wish`, its work that waits among them with the offer of a follow-up when core says one may
    /// be made of the wish: a tap on a choice leaves the offer where it was. The note that says what is being
    /// done carries the same field meanwhile ([`OnShow::carrying`]). One that asks once more has none: what
    /// was typed is the kit's to keep by the field's key, and is there when the work's note is back.
    fn with_offer(&self, wish: &str, notes: Vec<Note>) -> Vec<Note> {
        let follows = self.follows(wish);
        let work = |note: &Note| id_of(note).is_some_and(|id| id.starts_with(WORK));
        notes.into_iter().map(|note| if work(&note) { with_more(note, wish, follows) } else { note }).collect()
    }

    /// A tap on the character `id` that stops no run: the note of its wish over the scene `from`
    /// ([`Today::note_of`]). One wish is open at a
    /// time. None for a character that is no wish, for a wish core is at work for, and when the note is on show.
    pub(crate) fn opened(&self, page: Option<&Page>, from: Scene, id: &str) -> Option<Seen> {
        if busy(&from, id) {
            return None;
        }
        let now = over(&from, self.notes_on_open(page, id)?, false);
        (now != from).then_some(Seen { from, now })
    }
}

impl OnShow {
    /// A tap on option `index` (from 0) of the note `note`, when that is a choice of work that waits on show: the
    /// scene to show at once, which says what is being done in the note's place, over the field the note had, and
    /// the tap itself as work for another thread. None for any other note and for an option that is not drawn.
    /// While core is at work for a wish its choices are not on show, so a second tap does nothing.
    pub fn working(&mut self, note: &str, index: usize) -> Option<(Scene, Working)> {
        self.carrying(note, index, |today, wish, tap| {
            jellymoon_core::work_tapped(&today.home, &today.run_of(wish), tap)
        })
    }

    /// [`OnShow::working`] with the call to core given, so a test can stand in for it. Which tap it is, core says
    /// from what was drawn: the words of the option and the texts of its note. Nothing is decided here by a
    /// note's id but whose wish it is.
    ///
    /// The note's texts are what was drawn: the kit's view gives a tap only on an option it placed, and it places
    /// a text note's options only under a body drawn whole, every text to its last character. So the names a
    /// yes carries to core are names the person was shown.
    ///
    /// A carried call that panics is caught where it runs, and answered with a note in core's words in place of
    /// the one that says what is being done: the wish is never left as being worked on for good.
    pub fn carrying(&mut self, note: &str, index: usize, ask: impl Ask) -> Option<(Scene, Working)> {
        let wish = note.strip_prefix(WORK).or_else(|| note.strip_prefix(SURE))?.to_string();
        let at = self.scene.notes.iter().position(|one| id_of(one) == Some(note))?;
        let Note::Text { body, options, field, .. } = &self.scene.notes[at] else { return None };
        let after = self.scene.notes.iter().find_map(|one| file_of(one, &wish)).unwrap_or(0);
        let tap = WorkTap::of(options.get(index)?, body, after)?;
        // The field of `More like this` is the wish's, by its key: the kit keeps what is typed in it.
        let (field, mut doing) = (field.clone(), text(format!("{BUSY}{wish}"), tap.doing(), Vec::new(), Vec::new()));
        if let Note::Text { field: under, .. } = &mut doing {
            *under = field;
        }
        self.scene.notes[at] = doing;
        let (today, page, from) = (self.today.clone(), self.page.clone(), self.scene.clone());
        let work = move || {
            let failed = tap.failed();
            let asked = || {
                let notes = today.with_offer(&wish, notes_of(&wish, ask(&today, &wish, tap)));
                let mut now = today.again(page.as_ref(), &from);
                put(&mut now, &wish, notes.clone());
                (now, notes)
            };
            let (now, notes) = panic::catch_unwind(AssertUnwindSafe(asked)).unwrap_or_else(|_| {
                let (notes, mut now) = (notes_of(&wish, failed), from.clone());
                put(&mut now, &wish, notes.clone());
                (now, notes)
            });
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
