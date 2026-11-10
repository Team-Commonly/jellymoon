//! The layout rules, with a fixed-advance measurer and no GPUI.

use sticker_kit::{
    Character, FixedAdvance, Mood, Note, PieceKind, Placed, Rect, Scene, State, Theme, Viewport, layout,
};

const DEFAULT: Viewport = Viewport { w: 1440.0, h: 900.0 };
const SMALL: Viewport = Viewport { w: 960.0, h: 640.0 };
const MEASURE: FixedAdvance = FixedAdvance(0.56);

fn character(id: usize, label: &str, stars: u8, state: State) -> Character {
    Character { id: format!("c{id}"), label: label.into(), hue: (id % 5) as u8, stars, state, mood: Mood::Happy }
}

fn scene_with(characters: Vec<Character>) -> Scene {
    Scene { characters, ..Scene::sample() }
}

fn place(scene: &Scene, theme: &Theme, viewport: Viewport) -> Vec<Placed> {
    layout(scene, theme, viewport, &MEASURE)
}

fn of(placed: &[Placed], kind: PieceKind) -> Vec<&Placed> {
    placed.iter().filter(|p| p.piece == kind).collect()
}

fn full(viewport: Viewport) -> Rect {
    Rect::new(0.0, 0.0, viewport.w, viewport.h)
}

fn assert_apart(pieces: &[&Placed], what: &str) {
    for (i, a) in pieces.iter().enumerate() {
        for b in &pieces[i + 1..] {
            assert!(!a.rect.overlaps(&b.rect), "{what} overlap: {} {:?} and {} {:?}", a.id, a.rect, b.id, b.rect);
        }
    }
}

/// The checks every layout must pass, whatever the scene.
fn assert_sound(scene: &Scene, theme: &Theme, viewport: Viewport) {
    let placed = place(scene, theme, viewport);
    let labels = of(&placed, PieceKind::Label);
    let characters = of(&placed, PieceKind::Character);
    assert_eq!(characters.len(), scene.characters.len());
    assert_apart(&labels, "labels");
    assert_apart(&characters, "characters");
    for label in &labels {
        assert!(label.rect.inside(&full(viewport)), "label leaves the viewport: {label:?}");
        for run in &label.text {
            assert!(run.rect.inside(&label.rect.grown(0.5)), "text leaves its tag: {label:?}");
        }
        for other in characters.iter().filter(|c| c.id != label.id) {
            assert!(!label.rect.overlaps(&other.rect), "label {} covers {}", label.id, other.id);
        }
    }
    for who in &characters {
        assert!(who.rect.inside(&full(viewport)), "character leaves the viewport: {who:?}");
    }
    for note in of(&placed, PieceKind::Note) {
        let outline = note.rect.turned_bounds(note.tilt_degrees);
        assert!(outline.inside(&full(viewport)), "note leaves the viewport");
        for piece in placed.iter().filter(|p| is_character_art(p.piece)) {
            assert!(!outline.overlaps(&piece.rect), "note covers {:?} {}", piece.piece, piece.id);
        }
    }
}

fn is_character_art(kind: PieceKind) -> bool {
    use PieceKind::*;
    matches!(kind, Character | Star | Bubble | Badge | Label)
}

#[test]
fn one_character_piece_per_character() {
    let placed = place(&Scene::sample(), &Theme::sticker(), DEFAULT);
    let ids: Vec<&str> = of(&placed, PieceKind::Character).iter().map(|p| p.id.as_str()).collect();
    let expected: Vec<String> = {
        let scene = Scene::sample();
        let (last, first): (Vec<_>, Vec<_>) = scene.characters.into_iter().partition(|c| c.state == State::Dimmed);
        first.into_iter().chain(last).map(|c| c.id).collect()
    };
    assert_eq!(ids, expected);
    assert_eq!(of(&placed, PieceKind::Hero).len(), 1);
}

#[test]
fn one_star_piece_per_star_capped_at_four() {
    let scene = scene_with(vec![
        character(0, "none", 0, State::Idle),
        character(1, "three", 3, State::Idle),
        character(2, "many", 200, State::Idle),
    ]);
    let placed = place(&scene, &Theme::sticker(), DEFAULT);
    let stars = |id: &str| of(&placed, PieceKind::Star).iter().filter(|p| p.id == id).count();
    assert_eq!((stars("c0"), stars("c1"), stars("c2")), (0, 3, 4));
}

#[test]
fn bubble_for_busy_and_badge_for_needs_you() {
    let scene = scene_with(vec![
        character(0, "idle", 0, State::Idle),
        character(1, "busy", 0, State::Busy),
        character(2, "ask", 0, State::NeedsYou),
        character(3, "dim", 0, State::Dimmed),
    ]);
    let placed = place(&scene, &Theme::sticker(), DEFAULT);
    let ids = |kind| of(&placed, kind).iter().map(|p| p.id.clone()).collect::<Vec<_>>();
    assert_eq!(ids(PieceKind::Bubble), ["c1"]);
    assert_eq!(ids(PieceKind::Badge), ["c2"]);
}

#[test]
fn dimmed_characters_come_last() {
    let scene = scene_with(vec![
        character(0, "dim", 1, State::Dimmed),
        character(1, "a", 1, State::Idle),
        character(2, "dim too", 1, State::Dimmed),
        character(3, "b", 1, State::Busy),
    ]);
    let placed = place(&scene, &Theme::sticker(), DEFAULT);
    let ids: Vec<&str> = of(&placed, PieceKind::Character).iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, ["c1", "c3", "c0", "c2"]);
}

#[test]
fn eight_longest_labels_fit_at_default_and_small_sizes() {
    for theme in [Theme::sticker(), Theme::plain()] {
        let longest = "W".repeat(theme.labels.max_chars);
        let scene = scene_with((0..8).map(|i| character(i, &longest, 4, State::NeedsYou)).collect());
        for viewport in [DEFAULT, SMALL] {
            assert_sound(&scene, &theme, viewport);
            let placed = place(&scene, &theme, viewport);
            assert_eq!(of(&placed, PieceKind::Label).len(), 8);
        }
    }
}

#[test]
fn notes_stack_on_the_right_clear_of_the_characters() {
    let mut scene = Scene::sample();
    scene.notes.push(Note::Plain { title: "a third note".into(), body: "something that fits no other shape".into() });
    for viewport in [DEFAULT, SMALL] {
        let placed = place(&scene, &Theme::sticker(), viewport);
        let notes = of(&placed, PieceKind::Note);
        assert!(notes.len() >= 2, "at least the first two notes are placed");
        for pair in notes.windows(2) {
            let (above, below) =
                (pair[0].rect.turned_bounds(pair[0].tilt_degrees), pair[1].rect.turned_bounds(pair[1].tilt_degrees));
            assert!(above.bottom() <= below.y, "notes stack downward without overlap");
        }
        let rightmost = of(&placed, PieceKind::Character).iter().map(|p| p.rect.right()).fold(0.0, f32::max);
        assert!(notes.iter().all(|n| n.rect.x > rightmost));
        assert_sound(&scene, &Theme::sticker(), viewport);
    }
}

fn plain(title: &str) -> Note {
    Note::Plain { title: title.into(), body: "a line of body text".into() }
}

#[test]
fn a_long_list_gives_up_rows_to_the_notes_after_it_and_says_how_many() {
    let rows: Vec<(String, String)> = (0..40).map(|i| (format!("row {i}"), "of forty".to_string())).collect();
    let mut scene = Scene::sample();
    scene.notes = vec![Note::List { title: "a long list".into(), rows }, plain("second"), plain("third")];
    for viewport in [DEFAULT, SMALL] {
        let placed = place(&scene, &Theme::sticker(), viewport);
        let notes = of(&placed, PieceKind::Note);
        assert_eq!(notes.len(), 3, "the list did not push the later notes off the paper");
        let shown = notes[0].text.iter().filter(|run| run.text.starts_with("row ")).count();
        assert!((1..40).contains(&shown), "{shown} rows shown");
        let last = notes[0].text.last().map(|run| run.text.as_str());
        assert_eq!(last, Some(format!("+{} more", 40 - shown).as_str()));
        assert_sound(&scene, &Theme::sticker(), viewport);
    }
}

#[test]
fn a_note_too_tall_for_the_room_left_is_left_out_and_a_smaller_one_after_it_is_placed() {
    let tall = Note::Choice {
        id: "tall".into(),
        title: "four long options".into(),
        options: vec!["an option that runs on for three whole lines of text, and then some more words".repeat(2); 4],
        footer: "a footer".into(),
        struck_first: false,
    };
    let mut scene = Scene::sample();
    scene.notes = vec![tall.clone(), tall, plain("last")];
    let placed = place(&scene, &Theme::sticker(), DEFAULT);
    let notes: Vec<(&str, usize)> =
        of(&placed, PieceKind::Note).iter().map(|note| (note.id.as_str(), note.index)).collect();
    assert_eq!(notes, [("tall", 0), ("note-2", 2)]);
    assert_sound(&scene, &Theme::sticker(), DEFAULT);
}

#[test]
fn more_characters_than_slots_flow_to_more_rows() {
    let theme = Theme::sticker();
    for count in [9, 12, 30, 50] {
        let scene = scene_with((0..count).map(|i| character(i, "overflow", 2, State::Busy)).collect());
        for viewport in [DEFAULT, SMALL] {
            assert_sound(&scene, &theme, viewport);
            let placed = place(&scene, &theme, viewport);
            let mut rows: Vec<i32> =
                of(&placed, PieceKind::Character).iter().map(|p| p.rect.y.round() as i32).collect();
            rows.sort_unstable();
            rows.dedup();
            assert!(rows.len() > 2, "{count} characters use more than one staggered row");
        }
    }
}

#[test]
fn both_themes_place_the_same_pieces_in_the_same_order() {
    let scene = Scene::sample();
    for viewport in [DEFAULT, SMALL] {
        let sticker = place(&scene, &Theme::sticker(), viewport);
        let plain = place(&scene, &Theme::plain(), viewport);
        let keys = |placed: &[Placed]| placed.iter().map(|p| (p.piece, p.id.clone(), p.index)).collect::<Vec<_>>();
        assert_eq!(keys(&sticker), keys(&plain));
        assert!(sticker.iter().any(|p| p.tilt_degrees != 0.0));
        assert!(plain.iter().all(|p| p.tilt_degrees == 0.0));
    }
}

#[test]
fn scene_text_is_carried_as_text_only() {
    let nasty =
        format!("<script>alert(1)</script> & <b>bold</b>\n\tline two\u{200F}\u{202E}\u{0007}{}", "x".repeat(5000));
    let mut scene = Scene::sample();
    let calm = place(&scene, &Theme::sticker(), DEFAULT);
    scene.title = nasty.clone();
    scene.date_label = nasty.clone();
    scene.footer = nasty.clone();
    scene.characters[0].label = nasty.clone();
    let wild = place(&scene, &Theme::sticker(), DEFAULT);

    let keys = |placed: &[Placed]| placed.iter().map(|p| (p.piece, p.index)).collect::<Vec<_>>();
    assert_eq!(keys(&calm), keys(&wild), "odd text adds or removes no piece");
    for (before, after) in calm.iter().zip(&wild) {
        if !matches!(before.piece, PieceKind::Label | PieceKind::Title | PieceKind::Footer) {
            assert_eq!(before.rect, after.rect, "{:?} moved", before.piece);
        }
    }
    let max_chars = Theme::sticker().labels.max_chars;
    for run in wild.iter().flat_map(|p| &p.text) {
        assert!(run.text.chars().all(|c| !c.is_control()), "control character kept");
        assert!(!run.text.contains(['\u{200F}', '\u{202E}']), "direction mark kept");
        assert!(run.text.chars().count() <= 401, "very long text is cut");
    }
    let label = &of(&wild, PieceKind::Label)[0].text[0].text;
    assert!(label.starts_with("<script>alert(1)"), "markup is shown, not read: {label}");
    assert!(label.chars().count() <= max_chars + 1);
    assert_sound(&scene, &Theme::sticker(), DEFAULT);
}

/// A small deterministic generator (xorshift), so the test needs no crate.
struct Dice(u64);

impl Dice {
    fn roll(&mut self, below: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % below.max(1) as u64) as usize
    }

    fn label(&mut self) -> String {
        const ALPHABET: [&str; 12] = ["a", "W", " ", "i", "手", "帐", "日", "🌙", "⭐", "👩‍🚀", "é", "-"];
        let length = self.roll(61);
        (0..length).map(|_| ALPHABET[self.roll(ALPHABET.len())]).collect()
    }
}

#[test]
fn generated_scenes_never_overlap() {
    let mut dice = Dice(0x9E37_79B9_7F4A_7C15);
    let states = [State::Idle, State::Busy, State::NeedsYou, State::Dimmed];
    let themes = [Theme::sticker(), Theme::plain()];
    let viewports = [DEFAULT, SMALL, Viewport { w: 1200.0, h: 760.0 }];
    for round in 0..300 {
        let count = dice.roll(13);
        let characters = (0..count)
            .map(|i| {
                let label = dice.label();
                character(i, &label, dice.roll(5) as u8, states[dice.roll(4)])
            })
            .collect();
        let mut scene = scene_with(characters);
        if dice.roll(4) == 0 {
            scene.notes.clear();
        }
        if dice.roll(4) == 0 {
            scene.hero = None;
        }
        assert_sound(&scene, &themes[round % 2], viewports[round % 3]);
    }
}
