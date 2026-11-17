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
//! to answer, `answer_run`, which waits for the run to end. From Enter until then the wish's note says what core
//! says of a wish at work and has no field, so no key is held for it. Closing the page's window ends nothing:
//! the run goes on, and the note is as core says when the page is shown again. Quit cuts the run off, which the
//! next look ends as core ends any run that was cut off.

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

/// The title of the note for an answer core handed to the run, when something after that failed. Its body is
/// core's sentence for what failed.
pub const UNSETTLED: &str = "The answer was given, but what came after it failed.";

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
pub(crate) fn with_question(note: Note, question: Option<&str>) -> Note {
    let field = Field { offer: String::new(), hint: ANSWER_HINT.to_string(), most: 0, key: String::new() };
    match question {
        Some(question) => fielded(note, question, field),
        None => note,
    }
}

/// A note of the wish `wish` with the offer of a follow-up and its field, when core says one may be made of the
/// wish. The field's key is the wish's, not the note's: what is typed in it is the kit's to keep while another
/// note of the wish stands in this one's place, and is there under the next note that offers the same.
pub(crate) fn with_more(note: Note, wish: &str, follows: bool) -> Note {
    // Core keeps a question with a wish only while its run asks, and lets a follow-up be made only of work that
    // waits or was kept, so it never gives one wish both. If it did, this field would take the question's place.
    let field =
        Field { offer: MORE.to_string(), hint: MORE_HINT.to_string(), most: MADE_SAID_MAX, key: more_key(wish) };
    if follows { fielded(note, "", field) } else { note }
}

fn more_key(wish: &str) -> String {
    format!("more-{wish}")
}

/// The field of the note `note` on a scene: where the note is, whose wish it is under, and whether the field
/// is a follow-up's.
fn field_of(scene: &Scene, note: &str) -> Option<(usize, String, bool)> {
    let more = |one: &Note| match one {
        Note::Text { id, field: Some(field), .. } if id == note => Some(field.offer == MORE),
        _ => None,
    };
    let (at, more) = scene.notes.iter().enumerate().find_map(|(at, one)| Some((at, more(one)?)))?;
    Some((at, working::typed_under(note)?.to_string(), more))
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

    /// Whether the run of the made wish `wish` waits on `question` now: core keeps a question with a wish only
    /// while its run asks it.
    fn waits_on(&self, wish: &str, question: &str) -> bool {
        let made = jellymoon_core::made(&self.home, &self.date);
        made.iter().any(|one| crate::scene::drawn_id(&one.id) == wish && one.question.as_deref() == Some(question))
    }

    /// An answer to the run of the made wish `wish`, by core's `answer_run`, which waits for the run to end.
    /// `asked` is the note the answer was typed under. An answer that was given leaves the wish's note as core
    /// says the wish stands then. Core says of every failure only why, so whether the answer was given is
    /// looked at: the run waited on the note's question before the call and does not after it. Then it was
    /// handed over, and what core says failed came after. An answer that was not given leaves `asked` as it
    /// was, with its field, under core's reason.
    fn answering(&self, page: Option<&Page>, from: Scene, typed: &str, (wish, asked): (&str, Note)) -> Answer {
        let question = match &asked {
            Note::Text { quoted, .. } => quoted.clone(),
            _ => String::new(),
        };
        let waited = self.waits_on(wish, &question);
        let taken = jellymoon_core::answer_run(&self.home, &self.run_of(wish), typed.as_bytes());
        let given = taken.is_ok() || waited && !self.waits_on(wish, &question);
        let said = taken.err().map(|why| (if given { UNSETTLED } else { UNANSWERED }, format!("{why}.")));
        let note = if given { self.note_of(page, wish) } else { Some(asked) };
        let mut now = self.again(page, &from);
        wishing::newest(&mut now, said.clone());
        let opened = note.map(|note| (wish.to_string(), note));
        if let Some((wish, note)) = &opened {
            working::reopen(&mut now, wish, note.clone());
        }
        Answer::taken(Seen { from, now }, said, opened, given)
    }
}

impl OnShow {
    /// Enter in the field of the note `note`: `typed` carried to core, as work for another thread. Which call it
    /// is, the field drawn says: the one under the offer of a follow-up is a follow-up, and any other an answer.
    /// What it gives goes to [`OnShow::answered`], as a typed wish's does. None for a note with no field on show.
    ///
    /// An answer changes the scene on show at once, and that scene is given with the work: the wish's note says
    /// what core says of a wish at work, with no question and no field, as a tap on a choice of work puts a
    /// note that says what is being done. So the note is true while the run goes on, however long that is.
    pub fn typing(&mut self, note: &str, typed: &str) -> Option<(Option<Scene>, Wishing)> {
        let (at, wish, more) = field_of(&self.scene, note)?;
        let (today, page, typed) = (self.today.clone(), self.page.clone(), typed.to_string());
        if more {
            let from = self.scene.clone();
            return Some((None, Box::new(move || today.following(page.as_ref(), from, &typed, &wish))));
        }
        let at_work = working::at_work(&self.scene.notes[at]);
        let asked = std::mem::replace(&mut self.scene.notes[at], at_work);
        let from = self.scene.clone();
        let work = move || today.answering(page.as_ref(), from, &typed, (&wish, asked));
        Some((Some(self.scene.clone()), Box::new(work)))
    }
}
