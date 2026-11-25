//! The layout rules, with a fixed-advance measurer and no GPUI: the page is one column, as long as what it holds.

use sticker_kit::{
    Character, FixedAdvance, MAX_PIECES, Mood, Note, PieceKind, Placed, Role, Scene, State, Theme, Viewport, entry_top,
    layout, scroll_most,
};

const DEFAULT: Viewport = Viewport { w: 1440.0, h: 900.0 };
const SMALL: Viewport = Viewport { w: 960.0, h: 640.0 };
const MEASURE: FixedAdvance = FixedAdvance(0.56);

fn character(id: usize, label: &str, stars: u8, state: State) -> Character {
    let says = String::new();
    Character { id: format!("c{id}"), label: label.into(), hue: (id % 5) as u8, stars, state, mood: Mood::Happy, says }
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

fn assert_apart(pieces: &[&Placed], what: &str) {
    for (i, a) in pieces.iter().enumerate() {
        for b in &pieces[i + 1..] {
            assert!(!a.rect.overlaps(&b.rect), "{what} overlap: {} {:?} and {} {:?}", a.id, a.rect, b.id, b.rect);
        }
    }
}

fn said(note: &Note) -> bool {
    matches!(note, Note::Said { .. })
}

/// The checks every layout must pass, whatever the scene: one column, nothing beside or over anything else, and
/// nothing left of the window or right of it. How far down it goes is the column's own business.
fn assert_sound(scene: &Scene, theme: &Theme, viewport: Viewport) {
    let placed = place(scene, theme, viewport);
    let labels = of(&placed, PieceKind::Label);
    let characters = of(&placed, PieceKind::Character);
    assert_eq!(characters.len(), scene.characters.len());
    assert_apart(&labels, "labels");
    assert_apart(&characters, "characters");
    for piece in placed.iter().filter(|p| p.piece != PieceKind::Paper) {
        let outline = piece.rect.turned_bounds(piece.tilt_degrees);
        assert!(outline.x >= 0.0 && outline.right() <= viewport.w, "{piece:?} leaves the window's sides");
        assert!(outline.y >= 0.0, "{piece:?} is over the window's top");
        assert!(piece.text.iter().all(|run| run.rect.inside(&piece.rect.grown(0.5))), "text leaves {piece:?}");
    }
    // The cards: of one width, each under the one before, and no character on any of them.
    let bubble = |p: &Placed| p.piece == PieceKind::Quote && p.index == 0 && typed(scene, &p.id);
    let cards: Vec<&Placed> = placed.iter().filter(|p| p.piece == PieceKind::Note || bubble(p)).collect();
    for pair in cards.windows(2) {
        assert!(pair[0].rect.bottom() <= pair[1].rect.y, "{} is not under {}", pair[1].id, pair[0].id);
    }
    let notes = of(&placed, PieceKind::Note);
    assert!(notes.iter().all(|note| note.rect.x == notes[0].rect.x && note.rect.w == notes[0].rect.w), "one width");
    for art in placed.iter().filter(|p| is_character_art(p.piece)) {
        for card in &cards {
            assert!(!card.rect.overlaps(&art.rect), "{} covers {:?} {}", card.id, art.piece, art.id);
        }
    }
    assert_eq!(notes.len(), scene.notes.iter().filter(|note| !said(note)).count(), "every note is a card");
}

fn typed(scene: &Scene, id: &str) -> bool {
    scene.notes.iter().any(|note| matches!(note, Note::Said { id: said, .. } if said == id))
}

fn is_character_art(kind: PieceKind) -> bool {
    use PieceKind::*;
    matches!(kind, Character | Hero | Star | Bubble | Badge | Label | Says)
}

#[test]
fn one_character_piece_per_character_in_the_scene_s_order_each_on_a_row_of_its_own() {
    let scene = Scene::sample();
    for viewport in [DEFAULT, SMALL] {
        let placed = place(&scene, &Theme::sticker(), viewport);
        let small = of(&placed, PieceKind::Character);
        let ids: Vec<&str> = small.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, scene.characters.iter().map(|c| c.id.as_str()).collect::<Vec<_>>());
        // No one typed for these: each is one line under the header, the small one and then its label.
        let labels = of(&placed, PieceKind::Label);
        assert_eq!(labels.len(), small.len());
        let first = of(&placed, PieceKind::Note)[0].rect;
        for (who, label) in small.iter().zip(&labels) {
            assert_eq!((who.id.as_str(), who.index), (label.id.as_str(), label.index));
            assert!(who.rect.right() <= label.rect.x, "{label:?} is beside {who:?}");
            let middle = label.rect.center().1;
            assert!(middle >= who.rect.y && middle <= who.rect.bottom(), "on its line: {label:?} {who:?}");
            assert!(who.rect.bottom() <= first.y, "over the first card");
        }
        for pair in small.windows(2) {
            assert!(pair[0].rect.bottom() <= pair[1].rect.y && pair[0].rect.x == pair[1].rect.x, "{pair:?}");
        }
        assert_sound(&scene, &Theme::sticker(), viewport);
    }
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
    let struck: Vec<&str> = of(&placed, PieceKind::Label)
        .into_iter()
        .filter(|label| label.text[0].role == Role::LabelStruck)
        .map(|label| label.id.as_str())
        .collect();
    assert_eq!(struck, ["c3"], "one let go has its label struck, where it stands in the scene");
}

fn plain(title: &str) -> Note {
    Note::Plain { title: title.into(), body: "a line of body text".into() }
}

fn choice(id: &str) -> Note {
    let option = "an option that runs on for three whole lines of text, and then some more words".repeat(2);
    let (title, footer) = ("four long options".into(), "a footer".into());
    Note::Choice { id: id.into(), title, options: vec![option; 4], footer, struck_first: false, whole: false }
}

/// The page was a canvas of one window, and what had no room on it was cut, made smaller or left out and counted.
/// The column has no such room to run out of.
#[test]
fn every_note_is_a_card_of_the_column_s_width_at_its_full_height_and_none_is_left_out_for_room() {
    let rows: Vec<(String, String)> = (0..20).map(|i| (format!("row {i}"), "of twenty".to_string())).collect();
    let mut scene = Scene::sample();
    scene.notes = vec![Note::List { title: "a list".into(), rows }, choice("a"), choice("b"), plain("small")];
    scene.notes.extend((0..12).map(|n| if n % 2 == 0 { choice(&format!("c{n}")) } else { plain("more") }));
    for theme in [Theme::sticker(), Theme::plain()] {
        for viewport in [DEFAULT, SMALL] {
            let placed = place(&scene, &theme, viewport);
            let notes = of(&placed, PieceKind::Note);
            let ids: Vec<&str> = notes.iter().map(|note| note.id.as_str()).collect();
            assert_eq!(ids.len(), 16, "sixteen notes are sixteen cards: {ids:?}");
            assert_eq!(ids[..5], ["note-0", "a", "b", "note-3", "c0"], "in the scene's order");
            assert_eq!(notes.iter().map(|note| note.index).collect::<Vec<_>>(), (0..16).collect::<Vec<_>>());
            let rows = notes[0].text.iter().filter(|run| run.text.starts_with("row ")).count();
            assert_eq!(rows, 20, "every row of the list");
            let options = of(&placed, PieceKind::Option);
            assert_eq!(options.len(), 8 * 4, "every option of every choice");
            assert!(options.iter().all(|option| (1..=3).contains(&option.text.len())), "in three lines at most");
            assert!(placed.iter().flat_map(|p| &p.text).all(|run| !run.text.contains(" more of ")), "none counted");
            // Each role is written at its own size: nothing is made smaller to fit.
            let sizes = |role: Role| {
                let mut sizes: Vec<u32> =
                    placed.iter().flat_map(|p| &p.text).filter(|r| r.role == role).map(|r| r.size.to_bits()).collect();
                sizes.dedup();
                sizes.len()
            };
            assert_eq!((sizes(Role::Option), sizes(Role::Body), sizes(Role::NoteTitle)), (1, 1, 1));
            // The column is longer than the window, and that is all it costs: how far it scrolls.
            let (bottom, band) = (notes[15].rect.bottom(), entry_top(&theme, viewport));
            let most = scroll_most(&placed, &theme, viewport);
            assert!(bottom > viewport.h && most > 0.0 && bottom - most <= band, "{bottom} {most} {band}");
            assert_sound(&scene, &theme, viewport);
        }
    }
    scene.notes = vec![plain("small")];
    scene.characters.clear();
    let short = place(&scene, &Theme::sticker(), DEFAULT);
    assert_eq!(scroll_most(&short, &Theme::sticker(), DEFAULT), 0.0, "a column that fits does not scroll");
}

fn typed_for(id: &str, text: &str) -> Note {
    Note::Said { id: format!("typed-{id}"), text: text.into(), answer: String::new(), small: None }
}

#[test]
fn what_a_person_typed_is_a_quote_at_the_column_s_right_with_its_character_at_its_end() {
    let words = "please make the uploader try a file again when it fails, twice, and say so ".repeat(4);
    let mut scene = scene_with(vec![
        character(0, "nobody typed this", 1, State::Idle),
        Character { says: "reading a.txt".into(), ..character(1, "first", 2, State::Busy) },
        character(2, "second", 0, State::NeedsYou),
    ]);
    scene.notes = vec![plain("of the day"), typed_for("c1", &words), plain("came of it"), typed_for("c2", "short")];
    for theme in [Theme::sticker(), Theme::plain()] {
        for viewport in [DEFAULT, SMALL] {
            let placed = place(&scene, &theme, viewport);
            let notes = of(&placed, PieceKind::Note);
            let (left, right) = (notes[0].rect.x, notes[0].rect.right());
            let quotes = of(&placed, PieceKind::Quote);
            let [long, short] = &quotes[..] else { panic!("two bubbles: {quotes:?}") };
            assert_eq!((long.id.as_str(), short.id.as_str()), ("typed-c1", "typed-c2"));
            // Drawn by the rules for words that are not the scene's own: every run quoted, every word there.
            let drawn: Vec<&str> = long.text.iter().flat_map(|run| run.text.split_whitespace()).collect();
            assert_eq!(drawn.join(" "), words.trim(), "to its last word");
            assert!(quotes.iter().flat_map(|quote| &quote.text).all(|run| run.role == Role::Quoted));
            assert!(long.text.len() > 1 && short.text.len() == 1);
            for quote in &quotes {
                let who = placed.iter().find(|p| p.piece == PieceKind::Character && quote.id.ends_with(&p.id)).unwrap();
                assert!(quote.rect.x > left && quote.rect.right() <= who.rect.x, "ends at its character: {quote:?}");
                assert!(who.rect.right() <= right + 0.5 && who.rect.y < quote.rect.bottom(), "{who:?}");
                assert!(quote.rect.w < notes[0].rect.w * 0.75, "a bubble, not a card: {quote:?}");
                assert!(placed.iter().all(|p| p.piece != PieceKind::Label || p.id != who.id), "no label of its own");
            }
            assert!(short.rect.w < long.rect.w && short.rect.right() == long.rect.right(), "as wide as its words");
            // In the scene's order: the day's note, the first words, what came of them, the second words.
            assert!(notes[0].rect.bottom() <= long.rect.y && long.rect.bottom() <= notes[1].rect.y);
            assert!(notes[1].rect.bottom() <= short.rect.y);
            // The one nobody typed for is a row under the header; the others are at their bubbles only.
            let rows: Vec<&str> = of(&placed, PieceKind::Label).iter().map(|label| label.id.as_str()).collect();
            assert_eq!(rows, ["c0"]);
            assert_eq!(of(&placed, PieceKind::Character).len(), 3);
            // The busy one says its line under its own bubble, and the stars and states are drawn as ever.
            let says = of(&placed, PieceKind::Says);
            assert_eq!((says.len(), says[0].id.as_str(), says[0].text[0].text.as_str()), (1, "c1", "reading a.txt"));
            assert!(says[0].rect.y >= long.rect.y && says[0].rect.bottom() <= notes[1].rect.y, "{says:?}");
            assert_eq!(of(&placed, PieceKind::Star).iter().filter(|star| star.id == "c1").count(), 2);
            assert_eq!(of(&placed, PieceKind::Badge)[0].id, "c2");
            // The big one stands left of the column's first card and of the first card after typed words.
            let heroes = of(&placed, PieceKind::Hero);
            assert_eq!(heroes.iter().map(|hero| hero.index).collect::<Vec<_>>(), [0, 1]);
            for (hero, card) in heroes.iter().zip(&notes) {
                assert!(hero.rect.y == card.rect.y && hero.rect.right() < card.rect.x, "{hero:?} {card:?}");
            }
            assert_sound(&scene, &theme, viewport);
        }
    }
    // Words that clean to nothing are no bubble, and the character still stands there.
    scene.notes = vec![typed_for("c1", "\u{7}\u{200F}")];
    let placed = place(&scene, &Theme::sticker(), DEFAULT);
    assert!(of(&placed, PieceKind::Quote).is_empty() && of(&placed, PieceKind::Label).len() == 2);
}

#[test]
fn a_scene_of_any_size_is_laid_out_in_a_time_that_does_not_grow_with_its_notes() {
    let mut scene = Scene::sample();
    scene.notes = (0..200_000).map(|i| plain(&format!("note {i}"))).collect();
    let start = std::time::Instant::now();
    let placed = place(&scene, &Theme::sticker(), DEFAULT);
    assert!(start.elapsed() < std::time::Duration::from_secs(2), "{:?}", start.elapsed());
    assert_eq!(placed.len(), MAX_PIECES, "the backstop: no layout returns more pieces than this");
    let rows = (0..200_000).map(|i| (format!("row {i}"), String::new())).collect();
    scene.notes = vec![Note::List { title: "a list".into(), rows }];
    let placed = place(&scene, &Theme::sticker(), DEFAULT);
    let last = of(&placed, PieceKind::Note)[0].text.last().unwrap().text.clone();
    assert_eq!(last, "+199976 more", "a list is laid out to its twenty-fourth row, and the rest counted");

    let crowd = Scene::crowd(100_000);
    let placed = place(&crowd, &Theme::sticker(), DEFAULT);
    assert_eq!(placed.len(), MAX_PIECES);
    // However long the column, 150 holes are punched at most. A page with no end is refused.
    assert_eq!(of(&placed, PieceKind::Hole).len(), 150);
    let mut tall = Theme::sticker();
    tall.metrics.height = f32::INFINITY;
    assert!(place(&Scene::sample(), &tall, DEFAULT).is_empty());
}

#[test]
fn a_long_text_is_carried_to_400_characters_and_a_label_stays_in_the_column() {
    let narrow = FixedAdvance(0.001);
    let mut scene = Scene::sample();
    scene.notes = vec![Note::Plain { title: "t".into(), body: "b".repeat(1000) }];
    let placed = layout(&scene, &Theme::sticker(), DEFAULT, &narrow);
    let body = &of(&placed, PieceKind::Note)[0].text[1].text;
    assert_eq!(body.chars().count(), 401, "400 characters and the ellipsis");

    // A label allowed to be far wider than the column.
    let mut theme = Theme::sticker();
    theme.labels.max_width = 2000.0;
    theme.labels.max_chars = 200;
    let scene = scene_with(vec![character(0, &"W".repeat(200), 0, State::Idle)]);
    let placed = place(&scene, &theme, DEFAULT);
    let (label, card) = (of(&placed, PieceKind::Label)[0], of(&placed, PieceKind::Note)[0]);
    assert!(label.text[0].text.ends_with('…'), "the label was cut");
    assert!(label.rect.right() <= card.rect.right() + 0.5, "{:?} ends where the column does", label.rect);
}

#[test]
fn both_themes_place_the_same_pieces_in_the_same_order() {
    let scene = Scene::sample();
    for viewport in [DEFAULT, SMALL] {
        let sticker = place(&scene, &Theme::sticker(), viewport);
        let plain = place(&scene, &Theme::plain(), viewport);
        // The holes down the paper's edge are as many as the column is long, which a theme's sizes change.
        let keys = |placed: &[Placed]| {
            let kept = placed.iter().filter(|p| p.piece != PieceKind::Hole);
            kept.map(|p| (p.piece, p.id.clone(), p.index)).collect::<Vec<_>>()
        };
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
    scene.notes.push(typed_for("garden", "words"));
    let calm = place(&scene, &Theme::sticker(), DEFAULT);
    scene.title = nasty.clone();
    scene.date_label = nasty.clone();
    scene.footer = nasty.clone();
    scene.characters[1].label = nasty.clone();
    scene.notes[2] = typed_for("garden", &nasty);
    let wild = place(&scene, &Theme::sticker(), DEFAULT);

    // But for the holes down the paper's edge, which are as many as the column is long.
    let keys = |placed: &[Placed]| {
        placed.iter().filter(|p| p.piece != PieceKind::Hole).map(|p| (p.piece, p.index)).collect::<Vec<_>>()
    };
    assert_eq!(keys(&calm), keys(&wild), "odd text adds or removes no piece");
    let max_chars = Theme::sticker().labels.max_chars;
    for run in wild.iter().flat_map(|p| &p.text) {
        assert!(run.text.chars().all(|c| !c.is_control()), "control character kept");
        assert!(!run.text.contains(['\u{200F}', '\u{202E}']), "direction mark kept");
        assert!(run.text.chars().count() <= 401, "very long text is cut");
    }
    let label = &of(&wild, PieceKind::Label)[0].text[0].text;
    assert!(label.starts_with("<script>alert(1)"), "markup is shown, not read: {label}");
    assert!(label.chars().count() <= max_chars + 1);
    // What a person typed is cleaned and cut as every text is, and is still nothing but quoted text.
    let typed = of(&wild, PieceKind::Quote)[0];
    assert!(typed.text[0].text.starts_with("<script>alert(1)") && typed.text.iter().all(|r| r.role == Role::Quoted));
    let drawn = typed.text.iter().map(|run| run.text.chars().count()).sum::<usize>();
    assert!((390..=401).contains(&drawn), "400 and the ellipsis, less the blanks its lines end at: {drawn}");
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
        let characters: Vec<Character> = (0..count)
            .map(|i| {
                let label = dice.label();
                let says = if dice.roll(2) == 0 { dice.label() } else { String::new() };
                Character { says, ..character(i, &label, dice.roll(5) as u8, states[dice.roll(4)]) }
            })
            .collect();
        let mut scene = scene_with(characters);
        if dice.roll(4) == 0 {
            scene.notes.clear();
        }
        // Some of them were typed for, with a card or two under the words.
        for who in 0..count {
            if dice.roll(3) == 0 {
                scene.notes.push(typed_for(&format!("c{who}"), &dice.label()));
                scene.notes.extend((0..dice.roll(3)).map(|_| plain("came of it")));
            }
        }
        if dice.roll(4) == 0 {
            scene.hero = None;
        }
        assert_sound(&scene, &themes[round % 2], viewports[round % 3]);
    }
}
