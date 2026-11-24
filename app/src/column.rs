//! A scene in the order its page is read: one column, from its top. First the day's own notes, the ones of no made
//! wish. Then each made wish, the oldest first: what the person typed for it, and under that the wish's notes, in
//! one fixed order of what they are ([`ORDER`]), whatever order they were put on the scene in. A pure function:
//! the scene's notes are moved and none is changed, added to or taken away, but the list of the made wishes
//! (`scene::MADE`), whose rows are the words each `Said` now carries.
//!
//! It is applied where a scene goes to the kit to be drawn or placed (`window.rs`, `lines.rs`) and nowhere else:
//! every scene the app makes, keeps and compares is as it was.

use sticker_kit::{Note, Scene};

use crate::scene::MADE;

/// What comes before a wish's id in the id of what the person typed for it. `said-<wish>` is the answer to a tap
/// on the wish's work, so the person's own words are `typed-<wish>`.
pub const TYPED: &str = "typed-";

/// The notes of one wish, by what their ids begin with, in the order they stand under what the person typed: how
/// the wish is being shaped, what its run is doing and the choice to stop it, what the run says it did, the work
/// that waits and its files, a choice to be sure of, what is being done with the work, the wish's own note, and
/// last the answer.
pub const ORDER: [&str; 11] =
    ["shape-", "shaping", "steps-", "stop-", "told-", "work-", "file-", "sure-", "busy-", "wish-", "said-"];

/// The scene in the order of its column.
pub fn column(scene: &Scene) -> Scene {
    let made = |note: &Note| matches!(note, Note::Whole { title, .. } if title == MADE);
    let Some(Note::Whole { rows, .. }) = scene.notes.iter().find(|note| made(note)) else {
        return scene.clone();
    };
    // The made wishes are the scene's last characters, one for each row and in the rows' order (`scene::ruled`).
    let first = scene.characters.len().saturating_sub(rows.len());
    let mut left: Vec<Option<&Note>> = scene.notes.iter().filter(|note| !made(note)).map(Some).collect();
    let mut under = Vec::new();
    for (who, (_, said)) in scene.characters[first..].iter().zip(rows) {
        under.push(Note::Said { id: format!("{TYPED}{}", who.id), text: said.clone() });
        for prefix in ORDER {
            for note in &mut left {
                under.extend(note.take_if(|note| of(note, prefix, &who.id)).cloned());
            }
        }
    }
    let own = left.into_iter().flatten().cloned();
    Scene { notes: own.chain(under).collect(), ..scene.clone() }
}

/// Whether `note` is a note of the wish `wish` of the kind `prefix`: its id is the prefix and then the wish's id,
/// with or without something of the note's own between them and a `-` (`file-3-m1`, `shaping-m1`).
fn of(note: &Note, prefix: &str, wish: &str) -> bool {
    let (Note::Text { id, .. } | Note::Choice { id, .. } | Note::Told { id, .. }) = note else { return false };
    let Some(rest) = id.strip_prefix(prefix) else { return false };
    rest == wish || rest.strip_suffix(wish).is_some_and(|between| between.ends_with('-'))
}

#[cfg(test)]
mod tests {
    use sticker_kit::{Character, Mark, Note, Scene};

    use super::{ORDER, column};
    use crate::scene::MADE;

    fn text(id: &str) -> Note {
        Note::Text {
            id: id.into(),
            title: id.into(),
            body: Vec::new(),
            options: Vec::new(),
            quoted: String::new(),
            field: None,
        }
    }

    fn ids(scene: &Scene) -> Vec<String> {
        let id = |note: &Note| match note {
            Note::Text { id, .. }
            | Note::Choice { id, .. }
            | Note::Told { id, .. }
            | Note::Said { id, .. }
            | Note::Drawn { id, .. } => id.clone(),
            Note::Plain { title, .. } | Note::List { title, .. } | Note::Whole { title, .. } => title.clone(),
        };
        scene.notes.iter().map(id).collect()
    }

    fn day(notes: Vec<Note>) -> Scene {
        let who = |id: &str| Character { id: id.into(), ..Character::default() };
        let rows = vec![(Mark::Star, "first words".to_string()), (Mark::Busy, "second words".to_string())];
        let made = Note::Whole { title: MADE.into(), rows };
        Scene {
            characters: ["w1", "m1", "m2"].map(who).to_vec(),
            notes: notes.into_iter().chain([made]).collect(),
            ..Scene::default()
        }
    }

    #[test]
    fn each_made_wish_has_its_typed_words_then_its_notes_in_the_one_order_under_the_day_s_own() {
        let own = Note::Plain { title: "own".into(), body: String::new() };
        // Every kind of each wish, put on the scene in the reverse of the order and the newer wish first.
        let mut notes: Vec<Note> = ["m2", "m1"]
            .iter()
            .flat_map(|wish| {
                ORDER.iter().rev().map(move |prefix| match *prefix {
                    "file-" => text(&format!("file-3-{wish}")),
                    "shaping" => text(&format!("shaping-{wish}")),
                    prefix => text(&format!("{prefix}{wish}")),
                })
            })
            .collect();
        notes.extend([text("wish-w1"), own, text("work-m11")]);
        let ordered = ids(&column(&day(notes)));
        let of = |wish: &str| {
            let kinds = [
                "shape-", "shaping-", "steps-", "stop-", "told-", "work-", "file-3-", "sure-", "busy-", "wish-",
                "said-",
            ];
            [vec![format!("typed-{wish}")], kinds.iter().map(|kind| format!("{kind}{wish}")).collect()].concat()
        };
        let own = ["wish-w1", "own", "work-m11"].map(String::from).to_vec();
        assert_eq!(ordered, [own, of("m1"), of("m2")].concat(), "the day's own, then the older wish, then the newer");
    }

    #[test]
    fn the_typed_words_are_the_person_s_and_a_scene_with_no_made_wish_is_as_it_was() {
        let ordered = column(&day(Vec::new()));
        let said = |id: &str, text: &str| Note::Said { id: id.into(), text: text.into() };
        assert_eq!(ordered.notes, [said("typed-m1", "first words"), said("typed-m2", "second words")]);
        let none = Scene { notes: vec![text("work-m1")], ..Scene::default() };
        assert_eq!(column(&none), none);
    }
}
