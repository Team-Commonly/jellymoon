//! What is typed under a made wish (F54): an answer to the question its run asks, and a follow-up to work that
//! waits or was kept, or to a run that changed nothing. The rules and every sentence are core's. Whether a wish
//! asks, and what, is the `question` core keeps with it; whether a follow-up may be made of it, and of its work or
//! of a run with none, is `jellymoon_core::followable`; an answer is
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
/// The start of the key of a follow-up's field.
const MORE_KEY: &str = "more-";

/// The title of the note for an answer core did not take. Its body is core's reason.
pub const UNANSWERED: &str = "That answer was not taken.";

/// The title of the note for an answer core handed to the run, when something after that failed. Its body is
/// core's sentence for what failed.
pub const UNSETTLED: &str = "The answer was given, but what came after it failed.";

/// `note` with `field` under it. A drawn note takes the field and no quoted words: it is never a note that asks.
fn fielded(note: Note, quoted: &str, field: Field) -> Note {
    match note {
        Note::Text { id, title, body, options, .. } => {
            Note::Text { id, title, body, options, quoted: quoted.to_string(), field: Some(field) }
        }
        Note::Drawn { id, picture, words, open, options, .. } => {
            Note::Drawn { id, picture, words, open, options, field: Some(field) }
        }
        other => other,
    }
}

/// The note of the wish `wish` with the question its run asks, when core keeps one with the wish, and a field to
/// answer in. The kit holds the field to its own most characters, which is under core's bound on an answer.
/// The field's key is the wish's and the question's: what is typed in it is the kit's to keep while the note is
/// not on show, and is there when the wish is opened again; another question is another field, with no words.
pub(crate) fn with_question(note: Note, wish: &str, question: Option<&str>) -> Note {
    let Some(question) = question else { return note };
    let key = format!("answer-{wish}-{question}");
    fielded(note, question, Field { offer: String::new(), hint: ANSWER_HINT.to_string(), most: 0, key })
}

/// Whether a note has a field to answer in: a field under no offer (`with_question`) that is not a follow-up's
/// by its key. The field under the offer of a follow-up is not one, nor the one under none (`with_go_on`).
pub(crate) fn asks(note: &Note) -> bool {
    matches!(note, Note::Text { field: Some(field), .. } if field.offer.is_empty() && !field.key.starts_with(MORE_KEY))
}

/// A note of the wish `wish` with the offer of a follow-up and its field, when core says one may be made of the
/// wish. The field's key is the wish's, not the note's: what is typed in it is the kit's to keep while another
/// note of the wish stands in this one's place, and is there under the next note that offers the same.
pub(crate) fn with_more(note: Note, wish: &str, follows: bool) -> Note {
    // Core keeps a question with a wish only while its run asks, and lets a follow-up be made only of a run that
    // finished, so it never gives one wish both. If it did, this field would take the question's place.
    if follows { fielded(note, "", more(wish, MORE)) } else { note }
}

/// A note of the made wish `wish` with the field of a follow-up under no offer, when core says one may be made
/// of the wish's run, which changed nothing: there is no work to make more of. The field is `More like this`'s.
pub(crate) fn with_go_on(note: Note, wish: &str, goes_on: bool) -> Note {
    if goes_on { fielded(note, "", more(wish, "")) } else { note }
}

fn more(wish: &str, offer: &str) -> Field {
    let (offer, hint, key) = (offer.to_string(), MORE_HINT.to_string(), format!("{MORE_KEY}{wish}"));
    Field { offer, hint, most: MADE_SAID_MAX, key }
}

/// The field of the note `note` on a scene: where the note is, whose wish it is under, and whether the field
/// is a follow-up's. One rule says which: an answer is sent only from a field to answer in ([`asks`]).
fn field_of(scene: &Scene, note: &str) -> Option<(usize, String, bool)> {
    let more = |one: &Note| match one {
        Note::Text { id, field: Some(_), .. } if id == note => Some(!asks(one)),
        Note::Drawn { id, field: Some(_), .. } if id == note => Some(true),
        _ => None,
    };
    let (at, more) = scene.notes.iter().enumerate().find_map(|(at, one)| Some((at, more(one)?)))?;
    Some((at, working::typed_under(note)?.to_string(), more))
}

/// The made wish the field of a follow-up is drawn under, of all whose notes carry one on `scene`: the wish
/// the person tapped last, while a follow-up may be made of it, and else the newest made wish of which one may.
/// One offer on a page: a field under every wish would not say which wish the next words are for.
pub(crate) fn offered(scene: &Scene, tapped: Option<&str>) -> Option<String> {
    let may: Vec<&str> = scene.notes.iter().filter_map(more_of).collect();
    let newest = || crate::placing::made(scene).iter().rev().map(|one| one.id.as_str()).find(|id| may.contains(id));
    tapped.filter(|id| may.contains(id)).or_else(newest).map(str::to_string)
}

/// The wish whose follow-up's field a note carries.
fn more_of(note: &Note) -> Option<&str> {
    match note {
        Note::Text { field: Some(field), .. } | Note::Drawn { field: Some(field), .. } => {
            field.key.strip_prefix(MORE_KEY)
        }
        _ => None,
    }
}

/// `scene` as it is drawn: the field of a follow-up under the notes of one wish ([`offered`]) and of no other.
/// A field to answer a question in is never taken away. A scene on which no made wish has such a field is as
/// it was.
pub(crate) fn one_offer(scene: &mut Scene, tapped: Option<&str>) {
    let Some(wish) = offered(scene, tapped) else { return };
    for note in &mut scene.notes {
        if more_of(note).is_some_and(|of| of != wish)
            && let Note::Text { field, .. } | Note::Drawn { field, .. } = note
        {
            *field = None;
        }
    }
}

impl Today {
    /// A follow-up to the made wish `wish`, by core's `wish_after`, and the asking for its run, as a typed
    /// wish's. The note it was typed under stays, with its field: the wish keeps its place (ADR 0030), and the
    /// follow-up is a made wish of its own, with its own track under its own typed words.
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
    /// while its run asks it. None when that is not known: the wish is not among the day's made wishes, which
    /// is also what core gives of a day it cannot read.
    fn waits_on(&self, wish: &str, question: &str) -> Option<bool> {
        let made = jellymoon_core::made(&self.home, &self.date);
        let found = made.iter().find(|one| crate::scene::drawn_id(&one.id) == wish)?;
        Some(found.question.as_deref() == Some(question))
    }

    /// An answer to the run of the made wish `wish`, by core's `answer_run`, which waits for the run to end.
    /// `asked` is the note the answer was typed under. An answer that was given leaves the wish's note as core
    /// says the wish stands then. Core says of every failure only why, so whether the answer was given is
    /// looked at: the run waited on the note's question before the call and does not after it. Then it was
    /// handed over, and what core says failed came after. A look that cannot tell is no proof, and the answer
    /// is then held not to have been given. An answer that was not given leaves `asked` as it was, with its
    /// field, under core's reason ([`OnShow::answered`] puts it there when the wish's note went meanwhile). But
    /// where the run is known to wait on that question no more, the wish's note is as core says the wish
    /// stands, since `asked` would show a question nobody asks.
    fn answering(&self, page: Option<&Page>, from: Scene, typed: &str, (wish, asked): (&str, Note)) -> Answer {
        let question = match &asked {
            Note::Text { quoted, .. } => quoted.clone(),
            _ => String::new(),
        };
        let waited = self.waits_on(wish, &question);
        let taken = jellymoon_core::answer_run(&self.home, &self.run_of(wish), typed.as_bytes());
        let waits = self.waits_on(wish, &question);
        let given = taken.is_ok() || waited == Some(true) && waits == Some(false);
        let said = taken.err().map(|why| (if given { UNSETTLED } else { UNANSWERED }, format!("{why}.")));
        let note = if given || waits == Some(false) { self.note_of(page, wish) } else { Some(asked) };
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
    /// is, the field drawn says: one to answer in, under no offer, is an answer, and any other a follow-up.
    /// What it gives goes to [`OnShow::answered`], as a typed wish's does. None for a note with no field on show.
    ///
    /// An answer changes the scene on show at once, and that scene is given with the work: the wish's note says
    /// what core says of a wish at work, with no question and no field, as a tap on a choice of work puts a
    /// note that says what is being done. So the note is true while the run goes on, however long that is.
    pub fn typing(&mut self, note: &str, typed: &str) -> Option<(Option<Scene>, Wishing)> {
        // The field under a sharper wish's choices is no made wish's: its Enter is `shaping.rs`'s.
        if note == crate::shaping::SHAPE {
            return self.answering_shape(typed);
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    // The review of the third notes on F54, note 5: Enter under a made-up third field was an answer, from a field
    // that `asks` would never put back. A follow-up's field under no offer is not one either.
    #[test]
    fn an_answer_is_sent_only_from_a_field_to_answer_in() {
        let field = |(o, k): (&str, &str)| Field { offer: o.into(), hint: "".into(), most: 0, key: k.into() };
        let note = |shape: (&str, &str)| Note::Text {
            id: "wish-m1".to_string(),
            title: String::new(),
            body: Vec::new(),
            options: Vec::new(),
            quoted: String::new(),
            field: Some(field(shape)),
        };
        let more = |one| field_of(&crate::scene::bare("2026-03-10", vec![note(one)]), "wish-m1").unwrap().2;
        let all = [("", ""), (MORE, ""), ("Another offer", ""), ("", "more-m1")].map(more);
        assert_eq!(all, [false, true, true, true]);
    }

    /// A page of the made wishes `m1`, `m2`, `m3`, each of `may` with the field of a follow-up under its note,
    /// and `m3` with a field to answer in.
    fn page(may: &[&str]) -> Scene {
        let who = |id: &str| sticker_kit::Character { id: id.into(), ..Default::default() };
        let text = |id: String| working::text(id, "", Vec::new(), Vec::new());
        let note = |wish: &&str| with_more(text(format!("wish-{wish}")), wish, true);
        let asks = with_question(text("wish-m3".into()), "m3", Some("which?"));
        let rows = vec![(sticker_kit::Mark::Star, String::new()); 3];
        let made = Note::Whole { title: crate::scene::MADE.into(), rows };
        let notes = may.iter().map(note).chain([asks, made]).collect();
        Scene { characters: ["w1", "m1", "m2", "m3"].map(who).to_vec(), notes, ..Scene::default() }
    }

    #[test]
    fn the_offer_is_under_the_wish_tapped_last_and_with_no_tap_under_the_newest_that_may_be_followed() {
        let both = page(&["m2", "m1"]);
        assert_eq!(offered(&both, None).as_deref(), Some("m2"), "the newest made wish, whatever the notes' order");
        assert_eq!(offered(&both, Some("m1")).as_deref(), Some("m1"), "the wish tapped last");
        // A tap on a wish no follow-up may be made of, or on no made wish, leaves the offer with the newest.
        assert_eq!(offered(&both, Some("m3")).as_deref(), Some("m2"));
        assert_eq!(offered(&both, Some("w1")).as_deref(), Some("m2"));
        assert_eq!((offered(&page(&["m1"]), None).as_deref(), offered(&page(&[]), Some("m1"))), (Some("m1"), None));
    }

    #[test]
    fn one_wish_has_the_field_of_a_follow_up_and_a_question_keeps_its_field_always() {
        let fields = |tapped: Option<&str>| {
            let mut scene = page(&["m1", "m2"]);
            one_offer(&mut scene, tapped);
            let key = |note: &Note| match note {
                Note::Text { field: Some(field), .. } => Some(field.key.clone()),
                _ => None,
            };
            scene.notes.iter().filter_map(key).collect::<Vec<_>>()
        };
        assert_eq!(fields(None), ["more-m2", "answer-m3-which?"]);
        assert_eq!(fields(Some("m1")), ["more-m1", "answer-m3-which?"]);
    }
}
