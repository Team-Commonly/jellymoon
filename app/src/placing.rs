//! Where the notes of a made wish's work go on a scene that holds other notes (`working.rs` makes them).

use sticker_kit::{Note, Scene};

use crate::working::{BUSY, SAID, id_of, of, ours};

/// `notes` over the scene `from`, as the notes of an opened wish go: what core is at work on stays, and every
/// other note of an opened wish gives way to these. With `said`, what was done with work stays too: the answer
/// to a person's own tap is not taken away by a look. A wish that is being read, and a sharper wish that waits
/// for its tap, stay always: they hold the only copy of words the entry no longer has (`shaping.rs`).
pub(crate) fn over(from: &Scene, notes: Vec<Note>, said: bool) -> Scene {
    let stays = |id: &str| id.starts_with(BUSY) || said && id.starts_with(SAID) || crate::shaping::names(id);
    let mut now = from.clone();
    now.notes.retain(|one| !ours(one) || id_of(one).is_some_and(stays));
    now.notes.splice(0..0, notes);
    now
}

/// `notes` in place of every note of `wish` on the scene, where the first of those was. When they say what was
/// done with the wish's work, what was said of another wish's goes: an answer names no wish, so one stands on
/// the page, and it is of the work just kept or tossed.
pub(crate) fn put(scene: &mut Scene, wish: &str, notes: Vec<Note>) {
    let said = |note: &Note| id_of(note).is_some_and(|id| id.starts_with(SAID));
    let answer = notes.iter().any(said);
    scene.notes.retain(|note| of(note, wish) || !(answer && said(note)));
    let at = scene.notes.iter().position(|note| of(note, wish)).unwrap_or(0);
    scene.notes.retain(|note| !of(note, wish));
    let at = at.min(scene.notes.len());
    scene.notes.splice(at..at, notes);
}

#[cfg(test)]
mod tests {
    use super::*;

    // A tap on a wish's character opens its notes over the scene: a sharper wish that waits is not taken away.
    #[test]
    fn a_sharper_wish_that_waits_stays_when_another_wish_is_opened() {
        let text = |id: &str| Note::Text {
            id: id.to_string(),
            title: String::new(),
            body: Vec::new(),
            options: Vec::new(),
            quoted: String::new(),
            field: None,
        };
        let from = Scene { notes: vec![text("shape"), text("work-m1")], ..Scene::default() };
        let ids = |scene: &Scene| scene.notes.iter().filter_map(id_of).map(str::to_string).collect::<Vec<_>>();
        assert_eq!(ids(&over(&from, vec![text("work-m2")], false)), ["work-m2", "shape"]);
    }
}
