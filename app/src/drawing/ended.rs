//! A tucked wish says how it ended (ADR 0030, decision 11; `docs/development/app.md`, "Every wish keeps its
//! place"): under the bubble of a made wish that is tucked away stands one small picture, the kit's small form of
//! a note's picture (`Note::Said`'s `small`). It is made of the scene the window holds and of nothing else: no
//! call to core, nothing written, and no word of the person's or of a run's. This file passes the kit a picture
//! and never a word, so every word of the mark is the kit's own caption of that picture.
//!
//! - The change is in the folder: the wish's `kept-<wish>` note is drawn with its sheets in the folder
//!   (`landing::kept_note`), and the mark is that very picture, so its count is the card's.
//! - The wish was let go: core says so (its little one is drawn let go) and a note of the wish is on the scene.
//!   The mark is work thrown away, with no count: the scene holds none for it.
//! - The run finished and changed nothing: the note of what it says it did begins with the app's own sentence
//!   that nothing was changed (`told::UNCHANGED`). The mark is no file at all.
//!
//! Of any other end there is no mark: a change kept by a tap, a kept change the folder no longer shows, a wish
//! with no note on the scene. A picture of the folder is drawn only from a note that draws the folder.

use std::collections::BTreeSet;

use sticker_kit::{Kept, Note, Picture, Scene, State};

use super::tucked::settled;
use crate::column::TYPED;
use crate::landing::KEPT;
use crate::placing::{any_of, made};
use crate::told::{TOLD, UNCHANGED};
use crate::working::id_of;

/// How the made wish `wish` of `scene` ended, as a picture of finished work, or None where the scene does not
/// say it in one of the three ways above.
fn ended(scene: &Scene, wish: &str) -> Option<Picture> {
    let state = made(scene).iter().find(|one| one.id == wish)?.state;
    let named =
        |start: &str| scene.notes.iter().find(|note| id_of(note).and_then(|id| id.strip_prefix(start)) == Some(wish));
    let nothing = |state| Picture::Sheets { made: 0, changed: 0, removed: 0, state };
    match (state, named(KEPT), named(TOLD)) {
        (State::Idle, Some(Note::Drawn { picture, .. }), _) => {
            matches!(picture, Picture::Sheets { state: Kept::In, .. }).then(|| picture.clone())
        }
        (State::Dimmed, None, _) => scene.notes.iter().any(|note| any_of(note, wish)).then(|| nothing(Kept::Tossed)),
        (State::Idle, None, Some(Note::Told { end, .. })) => {
            (end.first().map(String::as_str) == Some(UNCHANGED)).then(|| nothing(Kept::Waiting))
        }
        _ => None,
    }
}

/// `column`, the column of `scene`, with the mark under the bubble of each wish of `away` that is settled, and
/// under no other bubble: a wish whose notes are drawn has none.
pub(super) fn marked(mut column: Scene, scene: &Scene, away: &BTreeSet<String>) -> Scene {
    for note in &mut column.notes {
        let Note::Said { id, small, .. } = note else { continue };
        let tucked = id.strip_prefix(TYPED).filter(|wish| away.contains(*wish) && settled(scene, wish));
        *small = tucked.and_then(|wish| ended(scene, wish));
    }
    column
}

#[cfg(test)]
mod tests {
    use sticker_kit::{Character, Mark, ToldRow};

    use super::*;
    use crate::drawing::Open;
    use crate::working::text;

    fn sheets(made: usize, changed: usize, state: Kept) -> Picture {
        Picture::Sheets { made, changed, removed: 0, state }
    }

    fn kept(wish: &str, picture: Picture) -> Note {
        let (id, words) = (format!("{KEPT}{wish}"), vec!["Your folder has this change now.".to_string()]);
        Note::Drawn { id, picture, words, open: false, options: Vec::new(), field: None }
    }

    /// What a run says it did: `said` is the run's own sentence, and `end` the app's sentences after it.
    fn told(wish: &str, said: &str, end: &[&str]) -> Note {
        let row = ToldRow { quotes: vec![said.to_string()], ..ToldRow::default() };
        let end = end.iter().map(|one| one.to_string()).collect();
        Note::Told { id: format!("{TOLD}{wish}"), title: String::new(), rows: vec![row], end }
    }

    /// A day of four made wishes with `notes`: the second was let go, and the others are done.
    fn day(notes: Vec<Note>) -> Scene {
        let who = |(id, state): (&str, State)| Character { id: id.into(), state, ..Character::default() };
        let all = [("m1", State::Idle), ("m2", State::Dimmed), ("m3", State::Idle), ("m4", State::Idle)];
        let rows = all.map(|(id, _)| (Mark::Star, format!("put it in your folder, {id}"))).to_vec();
        let made = Note::Whole { title: crate::scene::MADE.into(), rows };
        Scene {
            characters: all.map(who).to_vec(),
            notes: notes.into_iter().chain([made]).collect(),
            ..Scene::default()
        }
    }

    /// The mark under each bubble of the column `open` draws of `scene`.
    fn marks(open: &Open, scene: &Scene) -> Vec<Option<Picture>> {
        let small = |note: &Note| match note {
            Note::Said { small, .. } => Some(small.clone()),
            _ => None,
        };
        open.column(scene).notes.iter().filter_map(small).collect()
    }

    fn ended_so() -> Vec<Note> {
        vec![
            kept("m1", sheets(2, 1, Kept::In)),
            text("wish-m2".into(), "This wish was let go.", Vec::new(), Vec::new()),
            told("m3", "I looked and left it.", &[UNCHANGED, "Nothing was run to try this."]),
            text("wish-m3".into(), "This wish is done.", Vec::new(), Vec::new()),
            kept("m4", sheets(5, 0, Kept::In)),
        ]
    }

    #[test]
    fn each_tucked_wish_has_the_mark_of_its_own_end_and_a_wish_that_is_not_tucked_has_none() {
        let scene = day(ended_so());
        let open = Open::opening(&scene);
        let (folder, gone, none) = (sheets(2, 1, Kept::In), sheets(0, 0, Kept::Tossed), sheets(0, 0, Kept::Waiting));
        // The newest wish's notes are drawn: no mark. The first one's count is its own, not the newest's five.
        assert_eq!(marks(&open, &scene), [Some(folder.clone()), Some(gone.clone()), Some(none), None]);
        // A tap brings the first wish back, and its mark goes in the same drawing.
        assert!(open.poked(&scene, "m1"));
        assert_eq!(marks(&open, &scene)[..2], [None, Some(gone)]);
        // With no window's tuck there is no mark at all.
        assert_eq!(marks(&Open::default(), &scene), [None, None, None, None]);
        // Work that waits is never tucked away, whatever the window holds: no mark, and none of the folder.
        open.made(&scene, None, 4);
        let waits =
            day(ended_so().into_iter().chain([text("work-m1".into(), "", Vec::new(), vec!["Keep it"])]).collect());
        assert_eq!(marks(&open, &waits)[0], None);
        let held = marked(crate::column::column(&waits), &waits, &BTreeSet::from(["m1".to_string()]));
        let has = |note: &Note| matches!(note, Note::Said { small: Some(_), .. });
        assert!(!held.notes.iter().any(has), "a wish that is not settled has no mark, whatever is held of it");
        assert_eq!(marks(&open, &scene)[0], None, "and it stays shown when it has settled");
    }

    #[test]
    fn a_mark_is_made_of_a_drawing_or_of_the_app_s_own_sentence_and_of_no_other_word() {
        let mark = |notes: Vec<Note>, wish: &str| ended(&day(notes), wish);
        // No note of the wish: nothing is said of it, though core says it is done or was let go.
        assert_eq!((mark(Vec::new(), "m1"), mark(Vec::new(), "m2")), (None, None));
        // A kept change the folder no longer shows is sentences, and a change kept by a tap is core's answer.
        let earlier = text("kept-m1".into(), "This change was put in your folder earlier.", Vec::new(), Vec::new());
        let by_tap =
            text("said-m1".into(), "The change was kept.", vec!["It is in your folder now.".into()], Vec::new());
        assert_eq!((mark(vec![earlier], "m1"), mark(vec![by_tap], "m1")), (None, None));
        // Only sheets in the folder are a mark of the folder: not sheets that wait, and not another wish's.
        assert_eq!(mark(vec![kept("m1", sheets(1, 0, Kept::Waiting))], "m1"), None);
        assert_eq!(mark(vec![kept("m11", sheets(1, 0, Kept::In))], "m1"), None);
        // What a run says is never read: its sentence that nothing changed, or that all is in the folder, is no
        // mark. Nor are the person's typed words, which say "in your folder" on every row of this day.
        let said = told("m3", UNCHANGED, &["Nothing was run to try this."]);
        let boast = told("m1", "3 files are in your folder", &["Nothing was run to try this."]);
        assert_eq!((mark(vec![said], "m3"), mark(vec![boast], "m1")), (None, None));
        // A wish let go is said so whatever its notes say, and never as in the folder.
        let gone = mark(vec![text("said-m2".into(), "The change was thrown away.", Vec::new(), Vec::new())], "m2");
        assert_eq!(gone, Some(sheets(0, 0, Kept::Tossed)));
        assert_eq!(mark(vec![kept("m2", sheets(1, 0, Kept::In))], "m2"), None, "a let-go wish with a kept note");
    }
}
