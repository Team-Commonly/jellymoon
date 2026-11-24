//! The wish is shaped before a run (F91, ADR 0025). On Enter the page says at once that the wish is being read
//! ([`SHAPING`]), and core asks the person's Claude Code, which only reads, to say the wish more sharply
//! (`jellymoon_core::shape`). Words that are clear, and every answer core throws away, go on as a typed wish
//! always did (`wishing.rs`), and nothing more is said. A sharper wish is shown and nothing is kept: one note of
//! the kit's holds it ([`SAID`]), every word of the model's a quote under a label of Jellymoon's, over a note with
//! the person's own words and the two choices ([`SHAPE`]). A tap on either keeps the wish and starts its run
//! as a typed wish's is started; only what the run is given differs, and that is core's to build.

use jellymoon_core::{Shape, ShapeOutcome};
use sticker_kit::{Note, Scene, ToldRow};

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
/// Said when `Go with this` was tapped and the wish could only be kept as typed: its run has the typed words.
pub const WENT_TYPED: &str = "It goes on with the words you typed.";
/// The labels of the sharper wish's rows, in order; an empty row is left out. The question stands before the reason.
pub const LABELS: [&str; 5] = ["The wish", "Done when", "Not touching", "Only you know", "Why"];

/// Whether an id is one of this file's notes.
pub(crate) fn names(id: &str) -> bool {
    [SHAPING, SAID, SHAPE].contains(&id)
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

/// The two notes of a sharper wish. `said` is the typed words as core keeps them.
fn pair(shape: &Shape, said: String) -> Vec<Note> {
    let texts = [&shape.wish, &shape.done_when, &shape.not_touching, &shape.ask, &shape.why];
    let rows = LABELS.iter().zip(texts).filter(|(_, text)| !text.is_empty());
    let rows = rows.map(|(label, text)| ToldRow {
        label: label.to_string(),
        quotes: vec![text.clone()],
        ..ToldRow::default()
    });
    let sharper =
        Note::Told { id: SAID.to_string(), title: SHARPER.to_string(), rows: rows.collect(), end: Vec::new() };
    vec![sharper, text(SHAPE, TYPED, said, &[GO, GO_TYPED])]
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
        let (today, page, from) = (self.today.clone(), self.page.clone(), self.scene.clone());
        let work = move || {
            let why = match jellymoon_core::wish_of_shape(&today.read(), &said, index == 0) {
                Ok((made, carried)) => {
                    let unstarted = (today.asking(made.wish.id, made.wish.said))();
                    let went = (index == 0 && !carried)
                        .then(|| (WENT_TYPED, of(&said, "the sharper wish could not be kept.")));
                    unstarted.or(went)
                }
                Err(refused) => Some((UNKEPT, of(&said, &format!("{refused}.")))),
            };
            let mut now = today.again(page.as_ref(), &from);
            newest(&mut now, why.clone());
            let notes = why.map(|(title, body)| Note::Plain { title: title.to_string(), body });
            Worked { seen: Seen { from, now }, wish: String::new(), notes: notes.into_iter().collect() }
        };
        Some((self.scene.clone(), Box::new(work)))
    }
}
