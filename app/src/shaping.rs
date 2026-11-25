//! The wish is shaped before a run (F91, ADR 0025). On Enter the page says at once that the wish is being read
//! ([`SHAPING`]), and core asks the person's Claude Code, which only reads, to say the wish more sharply
//! (`jellymoon_core::shape`). Words that are clear, and every answer core throws away, go on as a typed wish
//! always did (`wishing.rs`), and nothing more is said. A sharper wish is shown and nothing is kept: one note of
//! the kit's holds it ([`SAID`]), every word of the model's a quote under a label of Jellymoon's, with a note of
//! the two choices ([`SHAPE`]). A tap on either keeps the wish and starts its run as a typed wish's is started;
//! only what the run is given differs, and that is core's to build. With no question that note is under the
//! sharper wish and quotes the person's own words ([`TYPED`]). With one it is the question's ([`ASKED`]) and
//! stands first: the question is its quote, the field under it is a third way on, the sharper wish with the
//! person's answer, and the two choices come after as the ways round the question ([`SKIP`], [`OWN`]). The typed
//! words are held by the page on show, with the sharper wish itself (`OnShow::typed`), and never read back from a
//! note: core keeps a wish only from the shape the page showed. The model's reason is
//! core's to read and is not drawn.

use jellymoon_core::{MADE_SAID_MAX, Shape, ShapeGo, ShapeOutcome, ShapeSeen};
use sticker_kit::{Field, Note, Scene, ToldRow};

use crate::taps::OnShow;
use crate::wishing::{Answer, NAMED_MAX, Seen, UNKEPT, Wishing, newest, of};
use crate::working::{Worked, Working, id_of};

/// The ids of the three notes: the wish is being read, the sharper wish, and the choice under it.
pub const SHAPING: &str = "shaping";
pub const SAID: &str = "shape-said";
pub const SHAPE: &str = "shape";

pub const READING: &str = "Reading your wish…";
pub const SHARPER: &str = "Your wish, sharper";
/// The title of the choice under a sharper wish that asks nothing: the person's own words are its quote.
pub const TYPED: &str = "What you typed";
/// The title of the choice under a sharper wish that asks: the question is its quote.
pub const ASKED: &str = "Only you know";
/// The two choices, in their order.
pub const GO: &str = "Go with this";
pub const GO_TYPED: &str = "Go with what I typed";
/// The same two, in the same order, as they read beside a question: what each does to it.
pub const SKIP: &str = "Skip the question";
pub const OWN: &str = "Use my own words";
/// The hint of the field the question is answered in.
pub const ANSWER_HINT: &str = "Type your answer, then Enter";
/// The labels of the sharper wish's rows, in order; an empty row is left out.
pub const LABELS: [&str; 3] = ["The wish", "Done when", "Not touching"];

/// Whether an id is one of this file's notes.
pub(crate) fn names(id: &str) -> bool {
    [SHAPING, SAID, SHAPE].contains(&id)
}

/// Whether an Enter in the entry was taken for a wish that the scene holds unsettled: the wish is being read,
/// or its sharper wish waits for a choice. A page does not say then that nothing was wished for
/// (`drawing::passed`), whatever drew the scene again meanwhile.
pub(crate) fn taken(scene: &Scene) -> bool {
    scene.notes.iter().any(|note| id_of(note).is_some_and(names))
}

/// Whether a sharper wish on the scene waits for the person's choice. An Enter in the entry takes the pair's
/// place and the first words are gone: that is for an Enter pressed in sight of the pair, and never for one the
/// entry remembered from before it stood (`Showing::takes_enter`).
pub(crate) fn waits(scene: &Scene) -> bool {
    scene.notes.iter().any(|note| id_of(note) == Some(SHAPE))
}

/// Whether a note is the one that says the day has no wish.
pub(crate) fn nothing(note: &Note) -> bool {
    matches!(note, Note::Plain { title, .. } if title == crate::scene::NOTHING)
}

fn text(id: &str, title: &str, quoted: String, options: &[&str]) -> Note {
    let options = options.iter().map(|option| option.to_string()).collect();
    Note::Text { id: id.to_string(), title: title.to_string(), body: Vec::new(), options, quoted, field: None }
}

/// `notes` in place of every note of this file's on the scene: a page holds one reading or one pair, never more.
pub(crate) fn settle(scene: &mut Scene, notes: Vec<Note>) {
    scene.notes.retain(|note| !id_of(note).is_some_and(names));
    scene.notes.splice(0..0, notes);
}

/// The two notes of a sharper wish. `said` is the typed words as core keeps them, drawn only when the sharper
/// wish asks nothing: the sharper wish, then the choices under the typed words. One that asks has the question's
/// note first, over the sharper wish: the question is its quote, directly under it is the field of an answer
/// (`Field::first`), of a wish's own length, which is core's bound on an answer, and after the field the two
/// ways round the question, small and quiet. That field asks the kit for the keys: what a person types on
/// reading the question is the answer, with no click first, unless they were typing in the entry already
/// (`Field::wants_keys`).
fn pair(shape: &Shape, said: &str) -> Vec<Note> {
    let texts = [&shape.wish, &shape.done_when, &shape.not_touching];
    let rows = LABELS.iter().zip(texts).filter(|(_, text)| !text.is_empty());
    let rows = rows.map(|(label, text)| ToldRow {
        label: label.to_string(),
        quotes: vec![text.clone()],
        ..ToldRow::default()
    });
    let sharper =
        Note::Told { id: SAID.to_string(), title: SHARPER.to_string(), rows: rows.collect(), end: Vec::new() };
    if shape.ask.is_empty() {
        return vec![sharper, text(SHAPE, TYPED, said.to_string(), &[GO, GO_TYPED])];
    }
    let mut asked = text(SHAPE, ASKED, shape.ask.clone(), &[SKIP, OWN]);
    if let Note::Text { field, .. } = &mut asked {
        let hint = ANSWER_HINT.to_string();
        *field = Some(Field { hint, most: MADE_SAID_MAX, wants_keys: true, first: true, ..Field::default() });
    }
    vec![asked, sharper]
}

impl OnShow {
    /// Enter in the entry: the scene says at once that the wish is being read, in the place of a sharper wish
    /// that stood there, and the reading is work for another thread. Its answer is a typed wish's
    /// ([`OnShow::wishing`]), or the sharper wish to show with nothing kept. With no project there is nothing to
    /// read, and the Enter is a typed wish's.
    pub fn shaping(&mut self, typed: &str, project: Option<&str>) -> Wishing {
        let Some(project) = project.map(str::to_string) else { return self.wishing(typed, None) };
        let reading = text(SHAPING, READING, jellymoon_core::wish_start(typed, NAMED_MAX), &[]);
        settle(&mut self.scene, vec![reading]);
        self.typed = None;
        let as_typed = self.wishing(typed, Some(&project));
        let (today, from, typed) = (self.today.clone(), self.scene.clone(), typed.to_string());
        Box::new(move || {
            let shaped = jellymoon_core::shape(&today.home, &today.run_of(""), &typed, &project);
            match (shaped.outcome, shaped.shape, shaped.said) {
                (ShapeOutcome::Shaped, Some(shape), Some(said)) => {
                    let (seen, notes) = (Seen { now: from.clone(), from }, pair(&shape, &said));
                    Answer::taken(seen, None, None, true).settling(notes, Some(ShapeSeen { said, project, shape }))
                }
                _ => as_typed().settling(Vec::new(), None),
            }
        })
    }

    /// The sharper wish of the pair on show with its typed words, held no more now that the pair goes: the one
    /// source for both taps and for Enter in the field. None when the note of the choices is not on show, and,
    /// for an answer, when it has no field.
    fn held(&mut self, answer: bool) -> Option<ShapeSeen> {
        let stands =
            |one: &Note| matches!(one, Note::Text { id, field, .. } if id == SHAPE && (field.is_some() || !answer));
        self.scene.notes.iter().any(stands).then(|| self.typed.take()).flatten()
    }

    /// A tap on a choice under a sharper wish: both notes go at once, and the work keeps the wish, with the
    /// sharper wish (`index` 0) or as typed, and asks for its run as a typed wish's is asked for. Core keeps it
    /// only while the shape that waits is the one this page showed. None for any other note or choice.
    pub(crate) fn shaped(&mut self, note: &str, index: usize) -> Option<(Scene, Working)> {
        let shown = (note == SHAPE && index < 2).then(|| self.held(false)).flatten()?;
        settle(&mut self.scene, Vec::new());
        // The wish is kept only by the work below: until its answer draws the page again, the scene does not
        // say that nothing was wished for.
        self.scene.notes.retain(|note| !nothing(note));
        let (today, page, from) = (self.today.clone(), self.page.clone(), self.scene.clone());
        let go = if index == 0 { ShapeGo::Shape } else { ShapeGo::Typed };
        let work = move || {
            let why = match jellymoon_core::wish_of_shape(&today.read(), &shown, go) {
                Ok(made) => (today.asking(made.wish.id, made.wish.said))(),
                Err(refused) => Some((UNKEPT, of(&shown.said, &format!("{refused}.")))),
            };
            let mut now = today.again(page.as_ref(), &from);
            newest(&mut now, why.clone());
            let notes = why.map(|(title, body)| Note::Plain { title: title.to_string(), body });
            Worked { seen: Seen { from, now }, wish: String::new(), notes: notes.into_iter().collect(), remade: None }
        };
        Some((self.scene.clone(), Box::new(work)))
    }

    /// Enter in the field under the choices, with words: both notes go at once, as at a tap, and the work keeps
    /// the wish with the sharper wish and the answer `typed`, and asks for its run as `Go with this` does. The
    /// sharper wish carried to core is the one held for the pair, question and all, and core keeps the wish only
    /// while the shape that waits is that one: an answer never goes with a question the page did not show. What
    /// core refuses keeps nothing: the pair stands again under core's reason, held again, and the field keeps
    /// its own. None when no such field is on show, and for a field
    /// with no words.
    pub(crate) fn answering_shape(&mut self, typed: &str) -> Option<(Option<Scene>, Wishing)> {
        if typed.trim().is_empty() {
            return None;
        }
        let ours = |note: &&Note| id_of(note).is_some_and(|id| [SAID, SHAPE].contains(&id));
        let stood: Vec<Note> = self.scene.notes.iter().filter(ours).cloned().collect();
        let shown = self.held(true)?;
        settle(&mut self.scene, Vec::new());
        self.scene.notes.retain(|note| !nothing(note));
        let (today, page, from, typed) = (self.today.clone(), self.page.clone(), self.scene.clone(), typed.to_string());
        let work = move || {
            let made = jellymoon_core::wish_of_shape(&today.read(), &shown, ShapeGo::Answer(&typed));
            let (why, asking, back, held) = match made {
                Ok(made) => (None, Some(today.asking(made.wish.id, made.wish.said)), Vec::new(), None),
                Err(refused) => (Some((UNKEPT, of(&shown.said, &format!("{refused}.")))), None, stood, Some(shown)),
            };
            let mut stands = from.clone();
            settle(&mut stands, back.clone());
            let mut now = today.again(page.as_ref(), &stands);
            newest(&mut now, why.clone());
            Answer::of(Seen { from, now }, why, asking).settling(back, held)
        };
        Some((Some(self.scene.clone()), Box::new(work)))
    }
}
