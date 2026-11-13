//! The entry's colours: what the built-in themes draw its texts in, the least contrast each is held to, and what a
//! theme file is given for a key it does not set (`tests/floors.rs` holds when it is not given it). Contrast is
//! WCAG 2's ratio of relative luminance.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use sticker_kit::theme::{CARET_FLOOR, CHIP_INK_FLOOR, CHOSEN_INK_FLOOR, HINT_FLOOR, Palette, RING_FLOOR, TYPED_FLOOR};
use sticker_kit::{Color, Theme};

fn hex(text: &str) -> Color {
    Color::parse(text).expect("a colour")
}

/// The chosen chip's name on its chip, the hint on the field, and the ring on the paper.
fn contrasts(p: &Palette) -> [f32; 3] {
    [p.chip_chosen_ink.contrast(p.chip_chosen), p.entry_hint.contrast(p.entry), p.entry_ring.contrast(p.paper)]
}

fn palette(file: &str) -> (Palette, Vec<String>) {
    let loaded = Theme::from_toml(file).expect("the file loads");
    (loaded.theme.palette, loaded.warnings)
}

#[test]
fn contrast_is_the_ratio_of_wcag_2() {
    let (black, white) = (hex("#000000"), hex("#FFFFFF"));
    let near = |got: f32, want: f32| (got - want).abs() < 0.006;
    assert!(near(black.contrast(white), 21.0) && near(white.contrast(black), 21.0));
    assert_eq!(white.contrast(white), 1.0);
    assert!(near(hex("#777777").contrast(white), 4.48) && near(hex("#767676").contrast(white), 4.54));
    // A colour one can see through is what it makes of its ground; one that is all transparent is the ground.
    assert_eq!(hex("#00000080").over(white), hex("#7F7F7F"));
    assert_eq!(hex("#00000000").contrast(white), 1.0);
    assert!(near(hex("#00000080").contrast(white), hex("#7F7F7F").contrast(white)));
}

#[test]
fn both_built_in_themes_draw_the_entry_s_texts_over_their_floors_and_through_the_door_unchanged() {
    assert_eq!((CHOSEN_INK_FLOOR, HINT_FLOOR, RING_FLOOR), (4.5, 2.5, 3.0));
    // Measured: the chosen chip's name on its chip, the hint on the field, the ring on the paper.
    let measured = [[9.99, 2.80, 5.35], [4.57, 3.06, 15.80]];
    // The field, its edge, the ring, the hint, the caret, a chip, the chosen chip and its name.
    let colours = [
        ["#FFFFFF", "#CFC8EE", "#C2371C", "#9A9AA0", "#E4573A", "#EDEBF4", "#FFD24D", "#2B2350"],
        ["#FFFFFF", "#C3C9D1", "#1F2328", "#8E949A", "#2F6FEB", "#ECEDEF", "#2F6FEB", "#FFFFFF"],
    ];
    for (n, theme) in [Theme::sticker(), Theme::plain()].into_iter().enumerate() {
        let p = &theme.palette;
        let got = [p.entry, p.entry_edge, p.entry_ring, p.entry_hint, p.caret, p.chip, p.chip_chosen];
        assert_eq!([&got[..], &[p.chip_chosen_ink]].concat(), colours[n].map(hex), "{}", theme.name);
        for ((got, want), floor) in contrasts(p).iter().zip(measured[n]).zip([4.5, 2.5, 3.0]) {
            assert!((got - want).abs() < 0.006 && *got >= floor, "{}: {got}, not {want}", theme.name);
        }
        // The three are told apart: the hint is a grey, and the ring a line darker than the chosen chip's fill.
        let grey = Color { r: p.entry_hint.g, b: p.entry_hint.g, ..p.entry_hint };
        assert!(p.entry_hint.contrast(grey) < 1.05, "{}: {:?} is a grey", theme.name, p.entry_hint);
        assert!(p.entry_ring.luminance() < p.chip_chosen.luminance(), "{}: the ring is the darker", theme.name);
        // A chip's name on its chip, what is typed on the field, and the caret on the field.
        let more = [p.soft.contrast(p.chip), p.ink.contrast(p.entry), p.caret.contrast(p.entry)];
        let floors = [CHIP_INK_FLOOR, TYPED_FLOOR, CARET_FLOOR];
        assert_eq!(floors, [4.5, 4.5, 3.0]);
        for ((got, want), floor) in more.iter().zip([[5.66, 14.39, 3.67], [5.45, 15.80, 4.57]][n]).zip(floors) {
            assert!((got - want).abs() < 0.006 && *got >= floor, "{}: {got}, not {want}", theme.name);
        }
        assert_eq!(p.chosen_edge(), p.chip_chosen, "{}: its fill tells the chosen chip, with no line", theme.name);
        assert_eq!(theme.check(), Ok(()));
        // Written to a file and read again, a theme is what it was, and nothing is said of it.
        let again = Theme::from_toml(&theme.to_toml().unwrap()).unwrap();
        assert_eq!((&again.theme.palette, again.warnings), (p, Vec::new()), "{}", theme.name);
    }
}

/// A dark theme written before the hint, the ring and the chosen chip's name had keys of their own.
const DARK: &str = r##"
[palette]
desk = "#14141C"
paper = "#1E1E2A"
ink = "#F2F2F7"
soft = "#C9C9D4"
quiet = "#8A8AA0"
option = "#2A2A3A"
entry = "#2A2A3A"
entry_edge = "#4A4A60"
caret = "#FFB347"
chip = "#33334A"
chip_chosen = "#3B3B52"
"##;

#[test]
fn a_file_without_the_three_newer_keys_draws_them_in_its_own_colours() {
    let (p, warnings) = palette(DARK);
    assert_eq!(warnings, Vec::<String>::new());
    let own = (hex("#C9C9D4"), hex("#8A8AA0"), hex("#FFB347"));
    assert_eq!((p.chip_chosen_ink, p.entry_hint, p.entry_ring), own, "its soft, its quiet and its caret");
    let [name, hint, ring] = contrasts(&p);
    assert!(name >= 6.0 && hint >= HINT_FLOOR && ring >= RING_FLOOR, "{name} {hint} {ring}");
    // Its chosen chip is a shade from its other chips: a line in the colour of the name tells it.
    assert_eq!(p.chosen_edge(), hex("#C9C9D4"));
    assert!(p.chosen_edge().contrast(p.chip) >= CHOSEN_INK_FLOOR);

    // The file of the review of PR 150: three keys, the rest the kit's. Its name is read; its amber caret is not
    // seen on the kit's white field, so the caret is the darker accent, and a line says so. The ring, which the
    // file did not set, is the caret's, and nothing is said of it.
    let three = "[palette]\nchip_chosen = \"#3B3B52\"\nsoft = \"#C9C9D4\"\ncaret = \"#FFB347\"\n";
    let (p, warnings) = palette(three);
    assert_eq!(p.chip_chosen_ink, hex("#C9C9D4"));
    assert!(contrasts(&p)[0] >= 6.0, "{:?}", contrasts(&p));
    assert_eq!(warnings, ["palette.caret: #FFB347 on palette.entry #FFFFFF is under 3 to 1; #E4573A is used"]);
    assert_eq!((p.caret, p.entry_ring), (hex("#E4573A"), hex("#E4573A")));
}

#[test]
fn a_key_a_file_does_not_set_is_its_own_colour_and_one_it_sets_is_kept() {
    // A file from before the entry sets none of its keys: each is the file's own colour, never the sticker's.
    let before = "[palette]\npaper = \"#0A0A0A\"\noption = \"#101820\"\nbubble_ring = \"#405060\"\naccent_dark = \"#F0A000\"\n\
                  star = \"#00C0A0\"\nsoft = \"#102030\"\nquiet = \"#8090A0\"\nink = \"#F0F0F0\"\n\
                  [palette.dimmed]\nlight = \"#C0D0E0\"\n";
    let (p, warnings) = palette(before);
    assert_eq!(warnings, Vec::<String>::new());
    assert_eq!((p.entry, p.entry_edge, p.caret), (hex("#101820"), hex("#405060"), hex("#F0A000")));
    assert_eq!((p.chip, p.chip_chosen, p.chip_chosen_ink), (hex("#C0D0E0"), hex("#00C0A0"), hex("#102030")));
    assert_eq!((p.entry_hint, p.entry_ring), (hex("#8090A0"), hex("#F0A000")), "the ring is the caret's");

    // A file that sets some of the keys keeps those, and the others are its own.
    let some = "[palette]\nchip_chosen_ink = \"#101010\"\nquiet = \"#707070\"\ncaret = \"#0040C0\"\n";
    let (p, warnings) = palette(some);
    assert_eq!(warnings, Vec::<String>::new());
    assert_eq!((p.chip_chosen_ink, p.entry_hint, p.entry_ring), (hex("#101010"), hex("#707070"), hex("#0040C0")));
    let ring = "[palette]\nentry_ring = \"#202020\"\ncaret = \"#0040C0\"\n";
    assert_eq!(palette(ring).0.entry_ring, hex("#202020"));

    // A file that sets neither a key nor the colour it is taken from is the sticker theme there, pair and all.
    for file in ["name = \"mine\"\n", "[palette]\nink = \"#102030\"\npaper = \"#FFFFFF\"\n"] {
        let (p, warnings) = palette(file);
        let sticker = Palette { ink: p.ink, paper: p.paper, ..Theme::sticker().palette };
        assert_eq!((p, warnings), (sticker, Vec::new()), "{file}");
    }
}

#[test]
fn text_a_file_makes_unreadable_is_replaced_and_the_file_still_loads() {
    // The name of the chosen chip in the chip's own colour, a white hint on a white field, a ring nobody sees.
    let unread = "[palette]\nchip_chosen = \"#3B3B52\"\nchip_chosen_ink = \"#3B3B52\"\nsoft = \"#C9C9D4\"\n\
                  entry_hint = \"#FFFFFF\"\nentry_ring = \"#FFFDF700\"\n";
    let (p, warnings) = palette(unread);
    assert_eq!(
        warnings,
        [
            "palette.entry_hint: #FFFFFF on palette.entry #FFFFFF is under 2.5 to 1; #9390A6 is used",
            "palette.entry_ring: #FFFDF700 on palette.paper #FFFDF7 is under 3 to 1; #E4573A is used",
            "palette.chip_chosen_ink: #3B3B52 on palette.chip_chosen #3B3B52 is under 4.5 to 1; #C9C9D4 is used",
        ]
    );
    assert!(contrasts(&p).iter().zip([4.5, 2.5, 3.0]).all(|(got, floor)| *got >= floor), "{:?}", contrasts(&p));

    // A dark chosen chip and nothing else: the sticker theme's dark name is not read on it, nor its `soft`, so
    // the name is white. The file did not set the name's key, and nothing is said.
    let (p, warnings) = palette("[palette]\nchip_chosen = \"#3B3B52\"\n");
    assert_eq!((p.chip_chosen_ink, warnings), (hex("#FFFFFF"), Vec::new()));

    // None of the file's own colours is read on a mid grey: then black is, or white.
    let grey = "[palette]\nchip_chosen = \"#808080\"\nchip_chosen_ink = \"#909090\"\nsoft = \"#707070\"\n\
                ink = \"#888888\"\n";
    let (p, warnings) = palette(grey);
    assert_eq!(p.chip_chosen_ink, hex("#000000"), "{warnings:?}");
    let white = "[palette]\nchip_chosen = \"#202020\"\nchip_chosen_ink = \"#303030\"\nsoft = \"#282828\"\n\
                 ink = \"#111111\"\n";
    assert_eq!(palette(white).0.chip_chosen_ink, hex("#FFFFFF"));
}
