//! What is typed under a made wish (F54): an answer to the question its run asks, and a follow-up to work that
//! waits or was kept. The rules and every sentence are core's. Whether a wish asks, and what, is the `question`
//! core keeps with it; whether a follow-up may be made of it is `jellymoon_core::followable`; an answer is
//! `answer_run`, a follow-up `wish_after`, and what either refuses is said in core's words under a title of this
//! file. Here a field is put under the note core's words are in, and what is typed in it is carried to core off
//! the window's thread, as a wish typed in the entry is. No GPUI.
//!
//! A question is a run's text: it is drawn as the kit draws someone else's words, apart from the note's own, and
//! is nothing but text here. The answer is the person's and goes to core alone: it is in no note and no line.
//!
//! An answered run goes on in this process, where a typed wish's run is a process of its own: core has one way
//! to answer, `answer_run`, which waits for the run to end. So the field waits until then, and a window closed
//! meanwhile cuts the run off, which the next look ends as core ends any run that was cut off.

use jellymoon_core::MADE_SAID_MAX;
use sticker_kit::{Field, Note, Scene};

use crate::read::Page;
use crate::taps::OnShow;
use crate::today::Today;
use crate::wishing::{self, Answer, Seen, UNKEPT, Wishing};
use crate::working;

/// The words over the field a follow-up is typed in.
pub const MORE: &str = "More like this";

const MORE_HINT: &str = "Type what to do next, then Enter";
const ANSWER_HINT: &str = "Type your answer, then Enter";

/// The title of the note for an answer core did not take. Its body is core's reason.
pub const UNANSWERED: &str = "That answer was not taken.";

fn fielded(note: Note, quoted: &str, field: Field) -> Note {
    match note {
        Note::Text { id, title, body, options, .. } => {
            Note::Text { id, title, body, options, quoted: quoted.to_string(), field: Some(field) }
        }
        other => other,
    }
}

/// A wish's note with the question its run asks, when core keeps one with the wish, and a field to answer in.
/// The kit holds the field to its own most characters, which is under core's bound on an answer.
pub(crate) fn asking(note: Note, question: Option<&str>) -> Note {
    let field = Field { offer: String::new(), hint: ANSWER_HINT.to_string(), most: 0 };
    match question {
        Some(question) => fielded(note, question, field),
        None => note,
    }
}

/// A wish's note with the offer of a follow-up and its field, when core says one may be made of the wish.
pub(crate) fn offering(note: Note, follows: bool) -> Note {
    let field = Field { offer: MORE.to_string(), hint: MORE_HINT.to_string(), most: MADE_SAID_MAX };
    if follows { fielded(note, "", field) } else { note }
}

/// The field of the note `note` on a scene: whose wish it is under, and whether it is a follow-up's.
fn field_of(scene: &Scene, note: &str) -> Option<(String, bool)> {
    let field = scene.notes.iter().find_map(|one| match one {
        Note::Text { id, field: Some(field), .. } if id == note => Some(field),
        _ => None,
    })?;
    Some((working::wish_of(note)?.to_string(), field.offer == MORE))
}

impl Today {
    /// A follow-up to the made wish `wish`, by core's `wish_after`, and the asking for its run, as a typed wish's.
    fn following(&self, page: Option<&Page>, from: Scene, typed: &str, wish: &str) -> Answer {
        let made = jellymoon_core::wish_after(&self.home, Some(&self.date), typed, wish);
        let (said, asking) = match made {
            Ok(made) => (None, Some(self.asking(made.wish.id, made.wish.said))),
            Err(why) => (Some((UNKEPT, wishing::of(typed, &format!("{why}.")))), None),
        };
        let mut now = self.again(page, &from);
        wishing::newest(&mut now, said.clone());
        Answer::of(Seen { from, now }, said, asking)
    }

    /// An answer to the run of the made wish `wish`, by core's `answer_run`, which waits for the run to end. An
    /// answer core took leaves the wish's note as core says the wish stands then; one it refused leaves the note
    /// as it was, with its field, under core's reason.
    fn answering(&self, page: Option<&Page>, from: Scene, typed: &str, wish: &str) -> Answer {
        let taken = jellymoon_core::answer_run(&self.home, &self.run_of(wish), typed.as_bytes());
        let said = taken.err().map(|why| (UNANSWERED, format!("{why}.")));
        let opened = if said.is_none() { self.note_of(page, wish) } else { None };
        let mut now = self.again(page, &from);
        wishing::newest(&mut now, said.clone());
        let opened = opened.map(|note| (wish.to_string(), note));
        if let Some((wish, note)) = &opened {
            working::reopen(&mut now, wish, note.clone());
        }
        Answer::taken(Seen { from, now }, said, opened)
    }
}

impl OnShow {
    /// Enter in the field of the note `note`: `typed` carried to core, as work for another thread. Which call it
    /// is, the field drawn says: the one under the offer of a follow-up is a follow-up, and any other an answer.
    /// What it gives goes to [`OnShow::answered`], as a typed wish's does. None for a note with no field on show.
    pub fn typing(&self, note: &str, typed: &str) -> Option<Wishing> {
        let (wish, more) = field_of(&self.scene, note)?;
        let (today, page, from, typed) = (self.today.clone(), self.page.clone(), self.scene.clone(), typed.to_string());
        Some(Box::new(move || match more {
            true => today.following(page.as_ref(), from, &typed, &wish),
            false => today.answering(page.as_ref(), from, &typed, &wish),
        }))
    }
}
