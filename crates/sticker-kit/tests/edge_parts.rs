//! What painting an edge scene rests on, with no window: the part of a text's box that is inside its bubble, the
//! ink of someone else's words and its floor on the bubble, the art of each mood, and whether a scene speaks.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashSet;

use sticker_kit::theme::QUOTED_FLOOR;
use sticker_kit::{Color, Edge, EdgeMood, EdgeText, Rect, Theme, Tone, art};

fn hex(text: &str) -> Color {
    Color::parse(text).expect("a colour")
}

#[test]
fn the_part_of_a_box_that_is_inside_another_is_never_past_it_and_never_less_than_nothing() {
    let within = Rect::new(10.0, 20.0, 100.0, 50.0);
    assert_eq!(Rect::new(20.0, 30.0, 10.0, 10.0).clipped(&within), Rect::new(20.0, 30.0, 10.0, 10.0));
    assert_eq!(Rect::new(0.0, 0.0, 500.0, 500.0).clipped(&within), within);
    assert_eq!(Rect::new(90.0, 60.0, 40.0, 40.0).clipped(&within), Rect::new(90.0, 60.0, 20.0, 10.0));
    assert_eq!(Rect::new(0.0, 25.0, 30.0, 10.0).clipped(&within), Rect::new(10.0, 25.0, 20.0, 10.0));
    // Apart, or touching at an edge: nothing, and no size below nothing.
    for apart in
        [Rect::new(200.0, 30.0, 10.0, 10.0), Rect::new(20.0, 70.0, 10.0, 10.0), Rect::new(0.0, 0.0, 10.0, 99.0)]
    {
        let part = apart.clipped(&within);
        assert_eq!((part.w, part.h), (0.0, 0.0), "{apart:?}");
    }
    // A box that is no numbers is nothing.
    let none = Rect::new(f32::NAN, 30.0, f32::INFINITY, 10.0).clipped(&within);
    assert_eq!((none.w, none.h), (0.0, 0.0));
    assert_eq!(Rect::new(20.0, 30.0, -5.0, 10.0).clipped(&within).w, 0.0);
}

#[test]
fn someone_else_s_words_have_an_ink_of_their_own_that_reads_on_the_bubble() {
    for theme in [Theme::sticker(), Theme::plain()] {
        let p = &theme.palette;
        let on_bubble = p.quoted.contrast(p.bubble.over(hex("#FFFFFF")));
        assert!(on_bubble >= QUOTED_FLOOR, "{}: {on_bubble}", theme.name);
        assert!(p.quoted != p.ink && p.quoted != p.soft && p.quoted != p.quiet, "{}: not the scene's own", theme.name);
    }
    assert_eq!(QUOTED_FLOOR, 4.5);
    let palette = |file: &str| {
        let loaded = Theme::from_toml(file).expect("the file loads");
        (loaded.theme.palette, loaded.warnings)
    };
    let reads = |p: &sticker_kit::theme::Palette| p.quoted.contrast(p.bubble.over(hex("#FFFFFF"))) >= QUOTED_FLOOR;
    // A file that sets no `quoted` takes its own `accent_dark` when that reads on its bubble.
    let (p, warnings) = palette("[palette]\naccent_dark = \"#004080\"\n");
    assert_eq!((p.quoted, warnings), (hex("#004080"), Vec::new()));
    // One that does not read is not taken, and nothing is said of a key the file did not set.
    let (p, warnings) = palette("[palette]\naccent_dark = \"#FFE0E0\"\n");
    assert_eq!((p.quoted, warnings), (Theme::sticker().palette.quoted, Vec::new()));
    // A dark bubble: the sticker theme's ink for it cannot be read there, so another is used, with no word.
    let (p, warnings) = palette("[palette]\nbubble = \"#202020\"\n");
    assert!(reads(&p) && p.quoted != Theme::sticker().palette.quoted && warnings.is_empty(), "{:?}", p.quoted);
    // The file's own `quoted` is the file's, while it reads; one that does not is replaced, and said.
    let (p, warnings) = palette("[palette]\nquoted = \"#123456\"\n");
    assert_eq!((p.quoted, warnings), (hex("#123456"), Vec::new()));
    let (p, warnings) = palette("[palette]\nquoted = \"#F0F0F0\"\n");
    assert!(reads(&p) && p.quoted != hex("#F0F0F0"));
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].starts_with("palette.quoted: #F0F0F0 on palette.bubble #FFFFFF is under 4.5 to 1; "));
}

#[test]
fn each_mood_is_drawn_as_art_of_its_own_at_one_size() {
    let theme = Theme::sticker();
    let moods = [
        EdgeMood::Floats,
        EdgeMood::LeansIn,
        EdgeMood::HoldsUp,
        EdgeMood::Swims,
        EdgeMood::Waves,
        EdgeMood::Glows,
        EdgeMood::Droops,
    ];
    let drawn: Vec<String> =
        moods.iter().map(|mood| art::edge_character(&theme.palette, theme.hue(0), *mood, 100.0, 6.0)).collect();
    assert_eq!(drawn.iter().collect::<HashSet<_>>().len(), moods.len(), "two moods are drawn alike");
    let (w, h) = art::character_size(100.0, 6.0);
    for (art, mood) in drawn.iter().zip(moods) {
        assert!(art.starts_with("<svg") && art.trim_end().ends_with("</svg>"), "{mood:?}");
        assert!(art.contains(&format!("width=\"{w:.2}\" height=\"{h:.2}\"")), "{mood:?}: {art}");
        // Art is shapes: no text, no link and no script is in it.
        assert!(!art.contains("<text") && !art.contains("href") && !art.contains("<script"), "{mood:?}");
    }
}

#[test]
fn a_scene_speaks_only_while_its_bubble_holds_something_a_person_sees() {
    let said =
        |text: &str| Edge { bubble: vec![EdgeText { text: text.to_string(), tone: Tone::Plain }], ..Edge::default() };
    assert!(!Edge::default().speaks() && !Edge { ring: Some(1.0), count: 3, ..Edge::default() }.speaks());
    for unseen in ["", " ", "\u{200b}", "\u{2800}", "\u{200d}", "\n\t"] {
        assert!(!said(unseen).speaks(), "{}", unseen.escape_unicode());
    }
    assert!(said("a").speaks() && said("\u{200b}a").speaks());
    let second =
        Edge { bubble: vec![EdgeText::default(), EdgeText { text: "b".into(), tone: Tone::Small }], ..Edge::default() };
    assert!(second.speaks());
}
