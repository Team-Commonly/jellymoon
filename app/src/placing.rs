//! Every made wish keeps its place (ADR 0030): where the notes of a made wish go on a scene that holds other
//! wishes' notes (`working.rs` makes them), and when they are made again. A made wish that has ended has the notes
//! an opening gives, with no tap, and they stay while the day's page is on show; a note of one wish takes no note
//! of another away. A wish of the reading is as it was: its note comes with a tap, one at a time.

use sticker_kit::{Character, Note, Scene, State};

use crate::read::Page;
use crate::today::Today;
use crate::told::rows_shown;
use crate::wishing::Seen;
use crate::working::{BUSY, FILE, SAID, SURE, WISH, at_work, id_of, of};

/// The characters of the day's made wishes: the last of the page's, one for each row of the note of them.
pub(crate) fn made(scene: &Scene) -> &[Character] {
    let rows = |note: &Note| match note {
        Note::Whole { title, rows } if title == crate::scene::MADE => Some(rows.len()),
        _ => None,
    };
    let count = scene.notes.iter().find_map(rows).unwrap_or(0);
    &scene.characters[scene.characters.len().saturating_sub(count)..]
}

/// The made wish of `scene` the note `id` is of: a tap on that note is a tap on that wish.
pub(crate) fn whose(scene: &Scene, id: &str) -> Option<String> {
    let named = |wish: &&String| id.strip_suffix(wish.as_str()).is_some_and(|start| start.ends_with('-'));
    made(scene).iter().map(|one| &one.id).find(named).cloned()
}

/// The wish a note that says how a wish stands is of.
fn stands_of(note: &Note) -> Option<&str> {
    id_of(note).and_then(|id| id.strip_prefix(WISH))
}

/// Whether a note is one of `wish`, the one that says how it stands among them.
fn any_of(note: &Note, wish: &str) -> bool {
    of(note, wish) || stands_of(note) == Some(wish)
}

/// Whether a note is the opened note of a wish of the reading: of a wish that is no made wish of `scene`.
fn read(note: &Note, scene: &Scene) -> bool {
    stands_of(note).is_some_and(|wish| !made(scene).iter().any(|one| one.id == wish))
}

/// Whether the made wish `wish` has a note in the person's hand, which no look and no tap on its little one
/// makes again: what core is doing with its work, the keep asked once more, what `Show me` brought (a file, or
/// the rows of the changes), or the note of a wish whose answer is with its run. Core's answer comes in its place.
pub(crate) fn in_hand(scene: &Scene, wish: &str) -> bool {
    let held = |note: &&Note| {
        let starts = |start: &str| id_of(note).is_some_and(|id| id.starts_with(start));
        starts(BUSY) || starts(SURE) || starts(FILE) || rows_shown(note) || starts(WISH) && at_work(note) == **note
    };
    scene.notes.iter().filter(|note| any_of(note, wish)).any(|note| held(&note))
}

/// `notes` in place of every note of `wish` on the scene, where the first of those was, or first when it had
/// none. No note of another wish is moved or taken away.
pub(crate) fn put(scene: &mut Scene, wish: &str, notes: Vec<Note>) {
    let at = scene.notes.iter().position(|note| any_of(note, wish)).unwrap_or(0);
    scene.notes.retain(|note| !any_of(note, wish));
    let at = at.min(scene.notes.len());
    scene.notes.splice(at..at, notes);
}

impl Today {
    /// The notes of the made wish `id` made again on `scene`, as core says the wish stands now: the notes an
    /// opening gives ([`Today::notes_on_open`]), in their place, over core's answer to the last choice on its
    /// work (`said-<wish>`), which stays. Nothing is changed while a note of the wish is in the person's hand.
    pub(crate) fn remade(&self, page: Option<&Page>, scene: &mut Scene, id: &str) {
        if in_hand(scene, id) {
            return;
        }
        let Some(mut notes) = self.notes_on_open(page, id) else { return };
        let said = |note: &&Note| id_of(note).and_then(|one| one.strip_prefix(SAID)) == Some(id);
        notes.extend(scene.notes.iter().find(said).cloned());
        put(scene, id, notes);
    }

    /// One look at the day made of the scene `from`: what core keeps now ([`Today::again`]), with the notes of
    /// each made wish of `whom` that has ended made again. When the look finds that a made wish has ended since
    /// `from` was drawn (its run ended, or it was made and ended elsewhere), the notes of every ended wish are
    /// made again, but of those in `held`: what landed may have changed what core says of an older wish. None
    /// when what is shown is what is kept.
    pub(crate) fn looked(&self, page: Option<&Page>, from: Scene, whom: &[String], held: &[String]) -> Option<Seen> {
        let mut now = self.again(page, &from);
        let ended = |scene: &Scene, id: &str| made(scene).iter().any(|one| one.id == id && one.state != State::Busy);
        let all: Vec<String> = made(&now).iter().map(|one| one.id.clone()).filter(|id| ended(&now, id)).collect();
        let fresh = all.iter().any(|id| !ended(&from, id));
        let again = |id: &&String| if fresh { !held.contains(id) } else { whom.contains(id) };
        for id in all.iter().filter(again) {
            self.remade(page, &mut now, id);
        }
        (now != from).then_some(Seen { from, now })
    }

    /// A tap on the character `id` that stops no run, over the scene `from`. Of a made wish: its notes made
    /// again, so core is asked of the wish tapped; while its work is a drawn note the scene is given even when
    /// nothing changed, as the tap also opens or closes that note (`drawing::Open`). Of a wish of the reading: its note
    /// ([`Today::note_of`]), first on the page, one at a time; None when that note is on show. A tap on either
    /// takes the opened note of another wish of the reading away, and no note of a made wish. None for a
    /// character that is no wish.
    pub(crate) fn opened(&self, page: Option<&Page>, from: Scene, id: &str) -> Option<Seen> {
        let mut now = from.clone();
        now.notes.retain(|note| !read(note, &from));
        if made(&from).iter().any(|one| one.id == id) {
            let busy = |note: &Note| id_of(note).and_then(|one| one.strip_prefix(BUSY)) == Some(id);
            if from.notes.iter().any(busy) {
                return None;
            }
            if let Some(notes) = self.notes_on_open(page, id) {
                put(&mut now, id, notes);
            }
            let drawn = now.notes.iter().any(|note| matches!(note, Note::Drawn { .. }) && of(note, id));
            return (drawn || now != from).then_some(Seen { from, now });
        }
        now.notes.insert(0, self.note_of(page, id)?);
        (now != from).then_some(Seen { from, now })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(id: &str) -> Note {
        crate::working::text(id.to_string(), "", Vec::new(), Vec::new())
    }

    fn ids(scene: &Scene) -> Vec<&str> {
        scene.notes.iter().filter_map(id_of).collect()
    }

    /// A page of the made wishes `m1` and `m11` and the reading's `w1`, under `notes`.
    fn page(notes: &[&str]) -> Scene {
        let who = |id: &str| Character { id: id.into(), ..Character::default() };
        let rows = vec![(sticker_kit::Mark::Star, String::new()); 2];
        let made = Note::Whole { title: crate::scene::MADE.into(), rows };
        let notes = notes.iter().map(|id| text(id)).chain([made]).collect();
        Scene { characters: ["w1", "m1", "m11"].map(who).to_vec(), notes, ..Scene::default() }
    }

    // A wish's notes go where its notes were, and a sharper wish that waits, another wish's notes and another
    // wish's answer are not taken away: one wish is not open in the place of another.
    #[test]
    fn a_wish_s_notes_take_the_place_of_its_own_and_of_no_other_wish_s() {
        let mut scene = page(&["shape", "said-m11", "told-m1", "work-m1", "file-2-m1", "wish-m11", "said-m1"]);
        put(&mut scene, "m1", vec![text("wish-m1"), text("said-m1")]);
        assert_eq!(ids(&scene), ["shape", "said-m11", "wish-m1", "said-m1", "wish-m11"]);
        // A wish with no note yet has its notes first.
        put(&mut scene, "m2", vec![text("work-m2")]);
        assert_eq!(ids(&scene)[..2], ["work-m2", "shape"]);
    }

    #[test]
    fn a_note_is_of_the_made_wish_its_id_ends_with_and_of_no_wish_whose_id_only_ends_the_same() {
        let scene = page(&[]);
        let whose = |id: &str| whose(&scene, id);
        assert_eq!(
            (whose("work-m1"), whose("file-3-m11"), whose("kept-m11")),
            (Some("m1".into()), Some("m11".into()), Some("m11".into()))
        );
        assert_eq!((whose("wish-w1"), whose("clash-1"), whose("m1")), (None, None, None));
    }

    #[test]
    fn a_wish_is_in_hand_while_core_is_at_work_for_it_or_a_file_of_it_is_shown_and_no_other_wish_is() {
        for held in ["busy-m1", "sure-m1", "file-1-m1"] {
            let scene = page(&["work-m11", held]);
            assert!(in_hand(&scene, "m1") && !in_hand(&scene, "m11"), "{held}");
        }
        assert!(!in_hand(&page(&["told-m1", "work-m1", "said-m1", "wish-m1"]), "m1"));
        // The opened note of a wish of the reading is no made wish's.
        let scene = page(&["wish-w1", "wish-m1"]);
        assert!(read(&scene.notes[0], &scene) && !read(&scene.notes[1], &scene));
    }
}
