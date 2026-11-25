//! A scene in the order its page is read: one column, from its top. First the day's own notes, the ones of no made
//! wish. Then each made wish, the oldest first: what the person typed for it, and under that the wish's notes, in
//! one fixed order of what they are ([`ORDER`]), whatever order they were put on the scene in. Last the notes of a
//! wish that is not made yet (`shaping.rs`): the newest thing on the page, over the entry. And under those, the
//! very last, the note that a newer build is installed (`newer.rs`), when the scene has it. A pure function:
//! the scene's notes are moved and none is changed, added to or taken away, but the list of the made wishes
//! (`scene::MADE`), whose rows are the words each `Said` now carries: the typed words and, after a line break,
//! the answer the wish went on with (`scene::typed`).
//!
//! It is applied where a scene goes to the kit to be drawn or placed, by `drawing::column` and nowhere else,
//! after the pass that opens and closes the drawn notes: every scene the app makes, keeps and compares is as it
//! was.

use sticker_kit::{Note, Scene};

use crate::scene::MADE;

/// What comes before a wish's id in the id of what the person typed for it. `said-<wish>` is the answer to a tap
/// on the wish's work, so the person's own words are `typed-<wish>`.
pub const TYPED: &str = "typed-";

/// The notes of one wish, by what their ids begin with, in the order they stand under what the person typed: what
/// its run is doing and the choice to stop it, what the run says it did, the work that waits or the change that
/// was kept by itself, the work's files, a
/// choice to be sure of, what is being done with the work, the wish's own note, and last the answer.
pub const ORDER: [&str; 10] =
    ["steps-", "stop-", "told-", "work-", "kept-", "file-", "sure-", "busy-", "wish-", "said-"];

/// The scene in the order of its column.
pub fn column(scene: &Scene) -> Scene {
    let made = |note: &Note| matches!(note, Note::Whole { title, .. } if title == MADE);
    let rows = match scene.notes.iter().find(|note| made(note)) {
        Some(Note::Whole { rows, .. }) => rows.as_slice(),
        _ => &[],
    };
    // The made wishes are the scene's last characters, one for each row and in the rows' order (`scene::ruled`).
    let first = scene.characters.len().saturating_sub(rows.len());
    let build = scene.notes.iter().filter(|note| crate::newer::names(note));
    let rest = scene.notes.iter().filter(|note| !made(note) && !crate::newer::names(note));
    let mut left: Vec<Option<&Note>> = rest.map(Some).collect();
    let mut under = Vec::new();
    for (who, (_, said)) in scene.characters[first..].iter().zip(rows) {
        let (text, answer) = said.split_once('\n').unwrap_or((said, ""));
        let (id, small) = (format!("{TYPED}{}", who.id), None);
        under.push(Note::Said { id, text: text.into(), answer: answer.into(), small });
        for prefix in ORDER {
            for note in &mut left {
                under.extend(note.take_if(|note| of(note, prefix, &who.id)).cloned());
            }
        }
    }
    let newest = |note: &&Note| crate::working::id_of(note).is_some_and(crate::shaping::names);
    let (last, own): (Vec<&Note>, Vec<&Note>) = left.into_iter().flatten().partition(newest);
    let notes = own.into_iter().cloned().chain(under).chain(last.into_iter().cloned()).chain(build.cloned());
    Scene { notes: notes.collect(), ..scene.clone() }
}

/// Whether `note` is a note of the wish `wish` of the kind `prefix`: its id is the prefix and then the wish's id,
/// with or without something of the note's own between them and a `-` (`file-3-m1`).
fn of(note: &Note, prefix: &str, wish: &str) -> bool {
    let (Note::Text { id, .. } | Note::Choice { id, .. } | Note::Told { id, .. } | Note::Drawn { id, .. }) = note
    else {
        return false;
    };
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
                    prefix => text(&format!("{prefix}{wish}")),
                })
            })
            .collect();
        notes.extend([text("wish-w1"), own, text("work-m11")]);
        let ordered = ids(&column(&day(notes)));
        let of = |wish: &str| {
            let kinds = ["steps-", "stop-", "told-", "work-", "kept-", "file-3-", "sure-", "busy-", "wish-", "said-"];
            [vec![format!("typed-{wish}")], kinds.iter().map(|kind| format!("{kind}{wish}")).collect()].concat()
        };
        let own = ["wish-w1", "own", "work-m11"].map(String::from).to_vec();
        assert_eq!(ordered, [own, of("m1"), of("m2")].concat(), "the day's own, then the older wish, then the newer");
    }

    // Every wish keeps its place (ADR 0030): a page holds the notes of all its made wishes at once, two at work and
    // two done, put on the scene interleaved. Each is under its own typed words, and one wish's id that ends
    // another's (`m1`, `m11`) takes none of the other's notes.
    #[test]
    fn the_notes_of_four_wishes_at_once_stand_each_under_its_own_typed_words() {
        let who = |id: &str| Character { id: id.into(), ..Character::default() };
        let rows = ["m1", "m2", "m3", "m11"].map(|wish| (Mark::Star, format!("words of {wish}"))).to_vec();
        let on =
            ["said-m3", "steps-m11", "kept-m2", "file-2-m1", "steps-m1", "wish-m3", "told-m2", "work-m1", "told-m1"];
        let notes = on.iter().map(|id| text(id)).chain([Note::Whole { title: MADE.into(), rows }]).collect();
        let scene = Scene { characters: ["w1", "m1", "m2", "m3", "m11"].map(who).to_vec(), notes, ..Scene::default() };
        let want = [
            ["typed-m1", "steps-m1", "told-m1", "work-m1", "file-2-m1"].as_slice(),
            &["typed-m2", "told-m2", "kept-m2"],
            &["typed-m3", "wish-m3", "said-m3"],
            &["typed-m11", "steps-m11"],
        ];
        assert_eq!(ids(&column(&scene)), want.concat());
    }

    #[test]
    fn the_typed_words_are_the_person_s_and_a_scene_with_no_made_wish_is_as_it_was() {
        let ordered = column(&day(Vec::new()));
        let said = |id: &str, text: &str, answer: &str| {
            let (id, text, answer) = (id.into(), text.into(), answer.into());
            Note::Said { id, text, answer, small: None }
        };
        assert_eq!(ordered.notes, [said("typed-m1", "first words", ""), said("typed-m2", "second words", "")]);
        // A row with an answer under its typed words gives its own bubble both, and no other bubble anything.
        let mut answered = day(Vec::new());
        let Some(Note::Whole { rows, .. }) = answered.notes.last_mut() else { panic!("the made wishes") };
        rows[1].1 = "second words\nonly seven".into();
        let both = [said("typed-m1", "first words", ""), said("typed-m2", "second words", "only seven")];
        assert_eq!(column(&answered).notes, both);
        let none = Scene { notes: vec![text("work-m1")], ..Scene::default() };
        assert_eq!(column(&none), none);
    }

    // A wish that is not made yet is the newest thing on the page: it stands last, over the entry it was typed in.
    #[test]
    fn a_wish_that_is_being_read_or_stands_sharper_is_the_column_s_last() {
        let pair = [text("shape-said"), text("shape")];
        let ordered = ids(&column(&day(pair.into_iter().chain([text("wish-w1"), text("work-m2")]).collect())));
        assert_eq!(ordered, ["wish-w1", "typed-m1", "typed-m2", "work-m2", "shape-said", "shape"]);
        let reading = Scene { notes: vec![text("shaping"), text("wish-w1")], ..Scene::default() };
        assert_eq!(ids(&column(&reading)), ["wish-w1", "shaping"]);
    }
    // A newer build installed under the window (`newer.rs`) is said under everything, where the reader's eyes
    // are: under a wish that is not made yet too, wherever its note was put on the scene.
    #[test]
    fn the_note_of_a_newer_build_is_the_column_s_last_whatever_else_is_on_it() {
        let plain = |title: &str| Note::Plain { title: title.into(), body: String::new() };
        let notes = vec![plain(crate::newer::TITLE), plain("own"), text("shape"), text("work-m2"), text("wish-w1")];
        let want = ["own", "wish-w1", "typed-m1", "typed-m2", "work-m2", "shape", crate::newer::TITLE];
        assert_eq!(ids(&column(&day(notes))), want);
    }
}
