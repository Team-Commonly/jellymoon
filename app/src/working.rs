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

use crate::again::Remade;
use crate::answering::{with_go_on, with_more, with_question};
use crate::landing::KEPT;
use crate::placing::put;
use crate::read::Page;
use crate::taps::OnShow;
use crate::today::Today;
use crate::told::{TOLD, in_turn, rows_shown, told, told_alone};
use crate::wishing::Seen;

/// The starts of the ids of this file's notes, each followed by the wish's id: work that waits with its choices,
/// the same asked once more, what is being done, what was done, and a wish with no work waiting.
pub(crate) const WORK: &str = "work-";
pub(crate) const SURE: &str = "sure-";
pub(crate) const BUSY: &str = "busy-";
pub(crate) const SAID: &str = "said-";
pub(crate) const WISH: &str = "wish-";
/// The start of the id of a file's note: `file-3-m1` is file 3 of the work of the made wish `m1`.
pub(crate) const FILE: &str = "file-";

/// What core answered to a tap on a choice: the scene it was asked from, the scene to show, and the notes to say.
pub struct Worked {
    pub(crate) seen: Seen,
    pub(crate) wish: String,
    pub(crate) notes: Vec<Note>,
    /// When the answer says a wish was made again and is being worked on: that wish, and the answer afterwards.
    pub(crate) remade: Option<Box<Remade>>,
}

/// A tap on a choice carried to core: the window runs it off its own thread.
pub type Working = Box<dyn FnOnce() -> Worked + Send>;

/// The call to core a tap is carried by: `jellymoon_core::work_tapped`, of the wish's run. `Make it again` is
/// carried by two: the tap, and then the toss (`again.rs`).
pub trait Ask: Fn(&Today, &str, WorkTap) -> WorkDone + Send + 'static {}
impl<A: Fn(&Today, &str, WorkTap) -> WorkDone + Send + 'static> Ask for A {}

pub(crate) fn text(id: String, title: &str, body: Vec<String>, options: Vec<&str>) -> Note {
    let options = options.into_iter().map(str::to_string).collect();
    Note::Text { id, title: title.to_string(), body, options, quoted: String::new(), field: None }
}

pub(crate) fn id_of(note: &Note) -> Option<&str> {
    match note {
        Note::Text { id, .. } | Note::Told { id, .. } | Note::Drawn { id, .. } => Some(id),
        _ => None,
    }
}

/// Whether an id is one of this file's notes: a tap on such a note is never a tap for `taps.rs`.
pub(crate) fn names(id: &str) -> bool {
    let ours = [WORK, SURE, BUSY, SAID, WISH, FILE, TOLD, KEPT];
    ours.iter().any(|start| id.starts_with(start)) || crate::shaping::names(id)
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
pub(crate) fn of(note: &Note, wish: &str) -> bool {
    let named = |start: &&str| id_of(note).and_then(|id| id.strip_prefix(*start)) == Some(wish);
    [WORK, SURE, BUSY, SAID, TOLD, KEPT].iter().any(named) || file_of(note, wish).is_some()
}

/// The wish a note of work that waits, or of how a wish stands, is of.
pub(crate) fn wish_of(id: &str) -> Option<&str> {
    [WORK, WISH, KEPT].iter().find_map(|start| id.strip_prefix(start))
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

/// The note of work that waits and was read, drawn, with core's words and choices (`drawing::sheets`). Before `Show me`
/// (`shown`) it names no file: core is asked of the same work with every name among those it only counts, and says
/// the same but them. Asked once more (`sure`) it is a text note: every name a yes is given to is drawn.
fn waiting(waits: &Waits, sure: bool, shown: bool) -> Note {
    if sure {
        let (title, body) = waits.asks();
        return text(format!("{SURE}{}", waits.wish), title, body, waits.choices(true));
    }
    let counted = Waits { names: Vec::new(), more: waits.more + waits.names.len(), ..waits.clone() };
    let lines = if shown { waits.lines() } else { counted.lines() };
    if !waits.read {
        // Core's reason is the whole note, and it has no choice: it stays sentences.
        return text(format!("{WORK}{}", waits.wish), waits.title(), lines, waits.choices(false));
    }
    crate::drawing::sheets(waits, lines)
}

/// The notes of work that waits: what its run says it did, when core printed a piece of it, and the work's own
/// note, with core's choices, after it. `shown`: the person tapped `Show me`, so the changes have their rows and
/// the files their names; a look, a tap on the wish's character and `layout` show neither (ADR 0023).
pub fn waiting_notes(waits: &Waits, shown: bool) -> Vec<Note> {
    told(waits, shown).into_iter().chain([waiting(waits, false, shown)]).collect()
}

/// The notes that say what core answered to a tap on a choice of `wish`.
pub fn notes_of(wish: &str, done: WorkDone) -> Vec<Note> {
    match done {
        WorkDone::Shown(waits, file) => {
            let file = text(format!("{FILE}{}-{wish}", file.n), &file.title(), file.lines(), Vec::new());
            waiting_notes(&waits, true).into_iter().chain([file]).collect()
        }
        WorkDone::Asks(waits) => vec![waiting(&waits, true, false)],
        WorkDone::Said(title, body) => vec![text(format!("{SAID}{wish}"), title, body, Vec::new())],
        WorkDone::Offers(title, body, choice) => vec![text(format!("{SAID}{wish}"), title, body, vec![choice])],
        // It is answered where the tap is carried (`again.rs`), and is nothing to show.
        WorkDone::Again(..) => Vec::new(),
    }
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
            let work = with_more(waiting(waits, false, false), &waits.wish, self.follows(&waits.wish));
            told(waits, false).into_iter().chain([work])
        };
        let mut notes: Vec<Note> = waits.iter().flat_map(work).collect();
        let made = jellymoon_core::made(&self.home, &self.date);
        let stands = |wish: &Wish| {
            let follows = self.follows(&wish.id);
            let typed_to = (wish.question.is_some() || follows) && !waits.iter().any(|waits| waits.wish == wish.id);
            let stands = typed_to.then(|| vec![with_more(standing(wish, None), &wish.id, follows)]);
            self.kept_notes(&wish.id).or(stands).unwrap_or_default()
        };
        notes.extend(made.iter().flat_map(stands));
        shown.notes.splice(0..0, notes);
        shown
    }

    /// Whether core says a follow-up of its work may be made of the made wish `wish` (`jellymoon_core::followable`).
    pub(crate) fn follows(&self, wish: &str) -> bool {
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
    /// choices and the offer of a follow-up, as they first come or as `Show me` shows them. None when none waits.
    pub(crate) fn work_notes(&self, id: &str, shown: bool) -> Option<Vec<Note>> {
        let waits = jellymoon_core::waits(&self.home, &self.run_of(id))?;
        let work = with_more(waiting(&waits, false, shown), id, self.follows(id));
        Some(told(&waits, shown).into_iter().chain([work]).collect())
    }

    /// [`Today::note_of`], with what the run says it did before the note of its work that waits: the notes a
    /// made wish that has ended has on the page (`placing.rs`).
    pub(crate) fn notes_on_open(&self, page: Option<&Page>, id: &str) -> Option<Vec<Note>> {
        if let Some(notes) = self.work_notes(id, false).or_else(|| self.kept_notes(id)) {
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

    /// `notes` of `wish`, its work that waits among them with the offer of a follow-up when core says one may
    /// be made of the wish: a tap on a choice leaves the offer where it was. The note that says what is being
    /// done carries the same field meanwhile ([`OnShow::carrying`]). One that asks once more has none: what
    /// was typed is the kit's to keep by the field's key, and is there when the work's note is back.
    fn with_offer(&self, wish: &str, notes: Vec<Note>) -> Vec<Note> {
        let follows = self.follows(wish);
        let work = |note: &Note| id_of(note).is_some_and(|id| id.starts_with(WORK));
        notes.into_iter().map(|note| if work(&note) { with_more(note, wish, follows) } else { note }).collect()
    }
}

impl OnShow {
    /// A tap on option `index` (from 0) of the note `note`, when that is a choice of work that waits on show, or
    /// the one choice under core's answer that a change no longer fits (`said-<wish>`, `again.rs`): the
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
    /// yes carries to core are names the person was shown. The drawn note of work that waits has no such choice:
    /// its words may be closed, and none of its three choices takes a name.
    ///
    /// A carried call that panics is caught where it runs, and answered with a note in core's words in place of
    /// the one that says what is being done: the wish is never left as being worked on for good.
    pub fn carrying(&mut self, note: &str, index: usize, ask: impl Ask) -> Option<(Scene, Working)> {
        let wish = [WORK, SURE, KEPT, SAID].iter().find_map(|start| note.strip_prefix(start))?.to_string();
        let at = self.scene.notes.iter().position(|one| id_of(one) == Some(note))?;
        let (body, options, field) = crate::drawing::held(&self.scene.notes[at])?;
        let after = self.scene.notes.iter().find_map(|one| file_of(one, &wish)).unwrap_or(0);
        let seen = after > 0 || self.scene.notes.iter().any(|one| of(one, &wish) && rows_shown(one));
        let tap = WorkTap::of(options.get(index)?, body, after)?;
        // The field of `More like this` is the wish's, by its key: the kit keeps what is typed in it.
        let (field, mut doing) = (field.cloned(), text(format!("{BUSY}{wish}"), tap.doing(), Vec::new(), Vec::new()));
        if let Note::Text { field: under, .. } = &mut doing {
            *under = field;
        }
        self.scene.notes[at] = doing;
        let (today, page, from) = (self.today.clone(), self.page.clone(), self.scene.clone());
        let work = move || {
            let (failed, shows, undo) = (tap.failed(), matches!(tap, WorkTap::Show { .. }), tap == WorkTap::Undo);
            let asked = || {
                let (done, since) = today.asked(&wish, tap, &ask);
                let said = notes_of(&wish, done);
                let remade = since.and_then(|(new, since)| Remade::of(new, &said, notes_of(&wish, since)));
                let mut notes = in_turn(said, seen);
                // A `Show me` that showed no file still brings the names and the rows: no other tap does.
                let unshown = shows && notes.iter().all(|note| id_of(note).is_some_and(|id| id.starts_with(SAID)));
                notes.extend(unshown.then(|| today.work_notes(&wish, true)).flatten().into_iter().flatten());
                // Under the answer to an undo, the work waits again, or the change stays (`landing.rs`).
                notes.extend(undo.then(|| today.after_undo(&wish)).into_iter().flatten());
                let notes = today.with_offer(&wish, notes);
                let mut now = today.again(page.as_ref(), &from);
                put(&mut now, &wish, notes.clone());
                (now, notes, remade)
            };
            let (now, notes, remade) = panic::catch_unwind(AssertUnwindSafe(asked)).unwrap_or_else(|_| {
                let (notes, mut now) = (notes_of(&wish, failed), from.clone());
                put(&mut now, &wish, notes.clone());
                (now, notes, None)
            });
            Worked { seen: Seen { from, now }, wish, notes, remade }
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
