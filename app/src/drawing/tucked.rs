//! Finished wishes tuck away (ADR 0030, decision 11; `docs/development/app.md`, "Every wish keeps its place"). A
//! made wish that is settled has none of its notes on the page once the person has made the next wish: its typed
//! words and its little one stay, and the little one says how it ended. Which wishes are tucked away is the
//! window's own state ([`Tucked`], held by `drawing::Open`): no call to core, and nothing written. It is applied
//! where a scene is made ready to be drawn (`drawing::passed`), so the scene the app keeps holds every note, and
//! `layout`, which has no tap, prints them all.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use sticker_kit::{Character, Note, Scene, State};

use crate::placing::{any_of, in_hand, made};
use crate::working::{SAID, WORK, id_of};

/// Whether the made wish `wish` of `scene` is settled: nothing of it waits for the person. Core says it is done
/// or was let go (its little one is neither busy nor needing the person: its run is not going and asks
/// nothing); no work of it waits (`work-<wish>`); no note of it is in the person's hand (`placing::in_hand`);
/// core's answer under it (`said-<wish>`) offers no choice; and no note of it has a field to answer in.
pub(crate) fn settled(scene: &Scene, wish: &str) -> bool {
    let ended = |one: &Character| one.id == wish && matches!(one.state, State::Idle | State::Dimmed);
    let named = |note: &Note, start: &str| id_of(note).and_then(|id| id.strip_prefix(start)) == Some(wish);
    let offers = |note: &Note| matches!(note, Note::Text { options, .. } if !options.is_empty());
    let asks = |note: &Note| any_of(note, wish) && crate::answering::asks(note);
    let waits = |note: &Note| named(note, WORK) || named(note, SAID) && offers(note) || asks(note);
    made(scene).iter().any(ended) && !in_hand(scene, wish) && !scene.notes.iter().any(waits)
}

/// The notes of `scene` that are drawn while the wishes of `away` are tucked away: every note but those of a
/// wish of `away` that is settled. No note of a wish that is not settled is left out, whatever `away` holds.
pub(super) fn shown<'a>(scene: &'a Scene, away: &'a BTreeSet<String>) -> impl Iterator<Item = &'a Note> {
    let hidden: Vec<&String> = away.iter().filter(|wish| settled(scene, wish)).collect();
    scene.notes.iter().filter(move |note| !hidden.iter().any(|wish| any_of(note, wish)))
}

/// The made wishes tucked away in one window, by their ids.
#[derive(Clone, Default)]
pub(crate) struct Tucked(Rc<RefCell<BTreeSet<String>>>);

impl Tucked {
    /// Every made wish of `scene` that is settled is tucked away, but the wish `but`. It is asked when a new made
    /// wish is kept, and when the window opens, with the newest made wish as `but`. A wish that settles between
    /// those moments stays on the page: a person sees the result of what they asked for until they ask again.
    /// With `drawn`, as when a wish is kept, so does a wish that ended since the last look and has no note on
    /// `scene` yet: its result was never drawn. A window that opens has drawn nothing, and asks without it.
    pub(crate) fn tuck(&self, scene: &Scene, but: Option<&str>, drawn: bool) {
        let ids = made(scene).iter().map(|one| &one.id);
        let seen = |id: &&String| !drawn || scene.notes.iter().any(|note| any_of(note, id));
        let away = ids.filter(|id| Some(id.as_str()) != but && settled(scene, id)).filter(seen);
        self.0.borrow_mut().extend(away.cloned());
    }

    /// The wishes tucked away while `scene` is drawn. One that is on `scene` and settled no more (its work waits
    /// again, core has something to say of it) is tucked away no longer: it is shown again by itself, and stays
    /// shown when it settles.
    pub(crate) fn away(&self, scene: &Scene) -> BTreeSet<String> {
        let on = |id: &String| made(scene).iter().any(|one| one.id == *id);
        let mut away = self.0.borrow_mut();
        away.retain(|id| !on(id) || settled(scene, id));
        away.clone()
    }

    /// A tap on the made wish `wish`: whether it was tucked away. It is no longer.
    pub(crate) fn back(&self, wish: &str) -> bool {
        self.0.borrow_mut().remove(wish)
    }
}

#[cfg(test)]
mod tests {
    use sticker_kit::Mark;

    use super::*;
    use crate::drawing::{Open, pass, passed};
    use crate::working::text;

    fn note(id: &str, options: &[&str]) -> Note {
        text(id.to_string(), "", Vec::new(), options.to_vec())
    }

    /// A day of three made wishes: `m1` kept in the folder, `m2` let go, and `m3` whose work waits; then `more`.
    fn day(more: Vec<Note>) -> Scene {
        let who = |(id, state): (&str, State)| Character { id: id.into(), state, ..Character::default() };
        let rows = vec![(Mark::Star, String::new()); 3];
        let notes = ["told-m1", "kept-m1", "wish-m2", "said-m2", "told-m3", "work-m3"].map(|id| note(id, &[]));
        let made = Note::Whole { title: crate::scene::MADE.into(), rows };
        Scene {
            characters: [("m1", State::Idle), ("m2", State::Dimmed), ("m3", State::Idle)].map(who).to_vec(),
            notes: notes.into_iter().chain(more).chain([made]).collect(),
            ..Scene::default()
        }
    }

    fn set(ids: &[&str]) -> BTreeSet<String> {
        ids.iter().map(|id| id.to_string()).collect()
    }

    /// The ids of the notes of `scene` as its column is drawn with `away` tucked away.
    fn drawn(scene: &Scene, away: &[&str]) -> Vec<String> {
        let column = crate::column::column(&passed(scene, &BTreeSet::new(), None, &set(away)));
        let id = |note: &Note| match note {
            Note::Said { id, .. } => Some(id.clone()),
            other => id_of(other).map(String::from),
        };
        column.notes.iter().filter_map(id).collect()
    }

    #[test]
    fn a_tucked_wish_has_its_bubble_and_none_of_its_notes_and_a_wish_whose_work_waits_is_never_tucked() {
        let all =
            ["typed-m1", "told-m1", "kept-m1", "typed-m2", "wish-m2", "said-m2", "typed-m3", "told-m3", "work-m3"];
        assert_eq!(drawn(&day(Vec::new()), &[]), all, "with none tucked away every wish has its notes");
        assert_eq!(pass(&day(Vec::new()), &BTreeSet::new()).notes.len(), 7, "`layout` has no tap: every note");
        let rest = ["typed-m1", "typed-m2", "typed-m3", "told-m3", "work-m3"];
        assert_eq!(drawn(&day(Vec::new()), &["m1", "m2"]), rest, "three bubbles, and the third wish's notes");
        assert_eq!(drawn(&day(Vec::new()), &["m1", "m2", "m3"]), rest, "work that waits is never tucked away");
    }

    #[test]
    fn a_tucked_wish_with_a_note_in_the_person_s_hand_or_a_choice_under_core_s_answer_is_drawn() {
        let again = note("said-m1", &["Make it again"]);
        for held in [note("busy-m1", &[]), note("sure-m1", &["Keep it"]), note("file-2-m1", &[]), again] {
            let scene = day(vec![held.clone()]);
            assert!(!settled(&scene, "m1") && settled(&scene, "m2"), "{held:?}");
            let on = drawn(&scene, &["m1", "m2"]);
            assert!(on.contains(&"kept-m1".to_string()) && !on.contains(&"wish-m2".to_string()), "{held:?}: {on:?}");
        }
        // Core's answer with no choice under it asks nothing: the wish is settled. A question does ask.
        assert!(settled(&day(vec![note("said-m1", &[])]), "m1"));
        let asks = crate::answering::with_question(note("wish-m1", &[]), "m1", Some("which?"));
        assert!(!settled(&day(vec![asks]), "m1"));
        // A wish core says is at work or needs the person is not settled, and neither is one that is no made wish.
        for state in [State::Busy, State::NeedsYou] {
            let mut scene = day(Vec::new());
            scene.characters[1].state = state;
            assert!(!settled(&scene, "m2"), "{state:?}");
        }
        assert!(!settled(&day(Vec::new()), "m9"));
    }

    #[test]
    fn settled_wishes_tuck_away_when_asked_and_one_that_is_settled_no_more_comes_back_and_stays() {
        let (scene, tucked) = (day(Vec::new()), Tucked::default());
        assert_eq!(tucked.away(&scene), set(&[]), "a wish that is settled stays until a wish is made");
        tucked.tuck(&scene, Some("m2"), true);
        assert_eq!(tucked.away(&scene), set(&["m1"]), "every settled wish but the one named, and no unsettled one");
        tucked.tuck(&scene, None, true);
        assert_eq!(tucked.away(&scene), set(&["m1", "m2"]));
        // The first wish's work waits again: it is shown by itself, and stays shown when it has settled.
        let waits = day(vec![note("work-m1", &["Keep it"])]);
        assert_eq!((tucked.away(&waits), tucked.away(&scene)), (set(&["m2"]), set(&["m2"])));
        // A scene that does not hold a wish says nothing of it.
        assert_eq!(tucked.away(&Scene::default()), set(&["m2"]));
        assert!(tucked.back("m2") && !tucked.back("m2") && tucked.away(&scene).is_empty());
    }

    #[test]
    fn a_wish_that_ended_with_no_note_on_the_scene_yet_is_not_tucked_away() {
        let (mut scene, tucked) = (day(Vec::new()), Tucked::default());
        scene.notes.retain(|note| !crate::working::of(note, "m1"));
        tucked.tuck(&scene, None, true);
        assert_eq!(tucked.away(&scene), set(&["m2"]), "the first wish's result was never on the page");
        tucked.tuck(&scene, None, false);
        assert_eq!(tucked.away(&scene), set(&["m1", "m2"]), "a window that opens has drawn nothing yet");
    }

    #[test]
    fn a_window_opens_with_every_settled_wish_tucked_away_but_the_newest_and_a_tap_brings_one_back() {
        let mut scene = day(Vec::new());
        scene.notes.retain(|note| !crate::working::of(note, "m3"));
        let open = Open::opening(&scene);
        let ids = |open: &Open| -> Vec<String> {
            let column = open.column(&scene);
            column.notes.iter().filter_map(|note| id_of(note).map(String::from)).collect()
        };
        assert_eq!(ids(&open), [""; 0], "the newest wish has no note here, and the older two are tucked away");
        assert!(open.poked(&scene, "m1"), "the tap brought a tucked wish back");
        assert_eq!(ids(&open), ["told-m1", "kept-m1"]);
        assert!(!open.poked(&scene, "m1"), "a second tap is the tap it was");
        // The next made wish tucks it away again; the wish an answer is of stays.
        open.made(&scene, Some("m2"));
        assert_eq!(ids(&open), [""; 0]);
        open.poked(&scene, "m2");
        open.made(&scene, Some("m2"));
        assert_eq!(ids(&open), ["wish-m2", "said-m2"]);
    }
}
