//! The wish is shaped before a run (F91, ADR 0025). On Enter the page says at once that the wish is being read
//! ([`SHAPING`]), and core asks the person's Claude Code, which only reads, to say the wish more sharply
//! (`jellymoon_core::shape`). Words that are clear, and every answer core throws away, go on as a typed wish
//! always did (`wishing.rs`), and nothing more is said. A sharper wish is shown and nothing is kept: one note of
//! the kit's holds it ([`SAID`]), every word of the model's a quote under a label of Jellymoon's, over a note with
//! the person's own words and the two choices ([`SHAPE`]). A tap on either keeps the wish and starts its run
//! as a typed wish's is started; only what the run is given differs, and that is core's to build. When the
//! sharper wish has a question, that note has a field under the choices ([`ANSWER`]), and Enter in it is a third
//! way on: the sharper wish with the person's answer. The model's reason is core's to read and is not drawn.

use jellymoon_core::{MADE_SAID_MAX, Shape, ShapeGo, ShapeOutcome};
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
/// The title of the choice: the person's own words stand under it as a quote.
pub const TYPED: &str = "What you typed";
/// The two choices, in their order.
pub const GO: &str = "Go with this";
pub const GO_TYPED: &str = "Go with what I typed";
/// The words over the field the sharper wish's question is answered in, under the choices, and the field's hint.
pub const ANSWER: &str = "Or go with this and your answer to the question";
pub const ANSWER_HINT: &str = "Type your answer, then Enter";
/// The labels of the sharper wish's rows, in order; an empty row is left out. The question is last: it stands
/// right over the note with the field it is answered in.
pub const LABELS: [&str; 4] = ["The wish", "Done when", "Not touching", "Only you know"];

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

/// The two notes of a sharper wish. `said` is the typed words as core keeps them. A sharper wish that asks has
/// the field of an answer under its choices, of a wish's own length, which is core's bound on an answer.
fn pair(shape: &Shape, said: String) -> Vec<Note> {
    let texts = [&shape.wish, &shape.done_when, &shape.not_touching, &shape.ask];
    let rows = LABELS.iter().zip(texts).filter(|(_, text)| !text.is_empty());
    let rows = rows.map(|(label, text)| ToldRow {
        label: label.to_string(),
        quotes: vec![text.clone()],
        ..ToldRow::default()
    });
    let sharper =
        Note::Told { id: SAID.to_string(), title: SHARPER.to_string(), rows: rows.collect(), end: Vec::new() };
    let mut choice = text(SHAPE, TYPED, said, &[GO, GO_TYPED]);
    if let (false, Note::Text { field, .. }) = (shape.ask.is_empty(), &mut choice) {
        let (offer, hint) = (ANSWER.to_string(), ANSWER_HINT.to_string());
        *field = Some(Field { offer, hint, most: MADE_SAID_MAX, key: String::new() });
    }
    vec![sharper, choice]
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
        let as_typed = self.wishing(typed, Some(&project));
        let (today, from, typed) = (self.today.clone(), self.scene.clone(), typed.to_string());
        Box::new(move || {
            let shaped = jellymoon_core::shape(&today.home, &today.run_of(""), &typed, &project);
            match (shaped.outcome, shaped.shape, shaped.said) {
                (ShapeOutcome::Shaped, Some(shape), Some(said)) => {
                    let seen = Seen { now: from.clone(), from };
                    Answer::taken(seen, None, None, true).settling(pair(&shape, said))
                }
                _ => as_typed().settling(Vec::new()),
            }
        })
    }

    /// A tap on a choice under a sharper wish: both notes go at once, and the work keeps the wish, with the
    /// sharper wish (`index` 0) or as typed, and asks for its run as a typed wish's is asked for. Core keeps it
    /// only while that shape still waits for those words. None for any other note or choice.
    pub(crate) fn shaped(&mut self, note: &str, index: usize) -> Option<(Scene, Working)> {
        let said = self.scene.notes.iter().find_map(|one| match one {
            Note::Text { id, quoted, .. } if id == SHAPE && note == SHAPE && index < 2 => Some(quoted.clone()),
            _ => None,
        })?;
        settle(&mut self.scene, Vec::new());
        // The wish is kept only by the work below: until its answer draws the page again, the scene does not
        // say that nothing was wished for.
        self.scene.notes.retain(|note| !nothing(note));
        let (today, page, from) = (self.today.clone(), self.page.clone(), self.scene.clone());
        let go = if index == 0 { ShapeGo::Shape } else { ShapeGo::Typed };
        let work = move || {
            let why = match jellymoon_core::wish_of_shape(&today.read(), &said, go) {
                Ok(made) => (today.asking(made.wish.id, made.wish.said))(),
                Err(refused) => Some((UNKEPT, of(&said, &format!("{refused}.")))),
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
    /// words carried to core are the note's that is held now, and core keeps the wish only while that shape
    /// still waits for them. What core refuses keeps nothing: the pair stands again under core's reason, and
    /// the field keeps its words. None when no such field is on show, and for a field with no words.
    pub(crate) fn answering_shape(&mut self, typed: &str) -> Option<(Option<Scene>, Wishing)> {
        let ours = |note: &&Note| id_of(note).is_some_and(|id| [SAID, SHAPE].contains(&id));
        let stood: Vec<Note> = self.scene.notes.iter().filter(ours).cloned().collect();
        let said = stood.iter().find_map(|one| match one {
            Note::Text { id, quoted, field: Some(_), .. } if id == SHAPE => Some(quoted.clone()),
            _ => None,
        })?;
        if typed.trim().is_empty() {
            return None;
        }
        settle(&mut self.scene, Vec::new());
        self.scene.notes.retain(|note| !nothing(note));
        let (today, page, from, typed) = (self.today.clone(), self.page.clone(), self.scene.clone(), typed.to_string());
        let work = move || {
            let made = jellymoon_core::wish_of_shape(&today.read(), &said, ShapeGo::Answer(&typed));
            let (why, asking, back) = match made {
                Ok(made) => (None, Some(today.asking(made.wish.id, made.wish.said)), Vec::new()),
                Err(refused) => (Some((UNKEPT, of(&said, &format!("{refused}.")))), None, stood),
            };
            let mut held = from.clone();
            settle(&mut held, back.clone());
            let mut now = today.again(page.as_ref(), &held);
            newest(&mut now, why.clone());
            Answer::of(Seen { from, now }, why, asking).settling(back)
        };
        Some((Some(self.scene.clone()), Box::new(work)))
    }
}
