//! A theme file never draws a pair of the entry under its floor for a key it did not set: its own colour is
//! taken for such a key only when the pair it makes meets its floor, every text of the entry has a floor on its
//! ground, and a warning names only a key the file set.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use sticker_kit::theme::{
    CARET_FLOOR, CHIP_INK_FLOOR, CHOSEN_FILL_FLOOR, CHOSEN_INK_FLOOR, HINT_FLOOR, Palette, RING_FLOOR, TYPED_FLOOR,
};
use sticker_kit::{Color, Theme};

fn hex(text: &str) -> Color {
    Color::parse(text).expect("a colour")
}

fn palette(file: &str) -> (Palette, Vec<String>) {
    let loaded = Theme::from_toml(file).expect("the file loads");
    (loaded.theme.palette, loaded.warnings)
}

/// Every text of the entry on its ground: its name here, its contrast and its floor. A colour one can see through
/// is laid on what is under it first: a chip and the field on the paper, the paper on the desk, the desk on white.
fn pairs(p: &Palette) -> [(&'static str, f32, f32); 6] {
    let paper = p.paper.over(p.desk.over(hex("#FFFFFF")));
    let on = |text: Color, fill: Color| text.contrast(fill.over(paper));
    [
        ("the chosen chip's name", on(p.chip_chosen_ink, p.chip_chosen), CHOSEN_INK_FLOOR),
        ("a chip's name", on(p.soft, p.chip), CHIP_INK_FLOOR),
        ("typed words", on(p.ink, p.entry), TYPED_FLOOR),
        ("the hint", on(p.entry_hint, p.entry), HINT_FLOOR),
        ("the caret", on(p.caret, p.entry), CARET_FLOOR),
        ("the ring", p.entry_ring.contrast(paper), RING_FLOOR),
    ]
}

fn near(got: f32, want: f32) -> bool {
    (got - want).abs() < 0.006
}

/// The light file of the review of PR 152: greys for a let-go character, and no key of the entry.
const GREYS: &str = "[palette]\npaper = \"#FAFAFA\"\nink = \"#111111\"\nsoft = \"#666666\"\nquiet = \"#999999\"\n\
                     star = \"#F5C400\"\n[palette.dimmed]\nbase = \"#B0B0B0\"\ndark = \"#8C8C8C\"\nlight = \"#9C9C9C\"\n";

#[test]
fn a_file_s_own_colour_is_taken_only_when_its_pair_meets_the_floor() {
    let sticker = Theme::sticker().palette;
    // Its `dimmed.light` under its `soft` is 2.09 to 1: the chip stays the sticker's, 4.86 as before, and its
    // `soft` on its `star` is 3.49, so the chosen chip and its name stay the sticker's two. Nothing is said.
    let (p, warnings) = palette(GREYS);
    assert!(near(p.soft.contrast(hex("#9C9C9C")), 2.09) && near(p.soft.contrast(p.star), 3.49));
    assert_eq!((p.chip, warnings), (sticker.chip, Vec::new()));
    assert!(near(p.soft.contrast(p.chip), 4.86), "{}", p.soft.contrast(p.chip));
    assert_eq!((p.chip_chosen, p.chip_chosen_ink), (sticker.chip_chosen, sticker.chip_chosen_ink), "the pair, whole");
    // With `dimmed` alone, the sticker's own `soft` would be 1.94 on the file's grey, and stays at 5.66.
    let (p, warnings) = palette("[palette.dimmed]\nlight = \"#8A8A8A\"\n");
    assert_eq!((p.chip, warnings), (sticker.chip, Vec::new()));
    assert!(near(p.soft.contrast(hex("#8A8A8A")), 1.94) && near(p.soft.contrast(p.chip), 5.66));
    // A grey the names are read on is taken; a file that sets another shade of `dimmed` gives the chip none.
    assert_eq!(palette("[palette.dimmed]\nlight = \"#F2F2F2\"\n").0.chip, hex("#F2F2F2"));
    assert_eq!(palette("[palette.dimmed]\ndark = \"#101010\"\n").0.chip, sticker.chip);

    // A blue `option` the ink is not read on is no field, and a pale `accent_dark` no caret. A dark `star` is no
    // chosen chip under the sticker's dark name, and with a pale `soft` beside it both are taken.
    for file in ["option = \"#2F6FEB\"\n", "accent_dark = \"#FFE0D0\"\n", "star = \"#102030\"\n"] {
        let (p, warnings) = palette(&format!("[palette]\n{file}"));
        let kept = Palette { option: p.option, accent_dark: p.accent_dark, star: p.star, ..sticker.clone() };
        assert_eq!((p, warnings), (kept, Vec::new()), "{file}");
    }
    let (p, warnings) = palette("[palette]\nstar = \"#102030\"\nsoft = \"#F0F0F0\"\n");
    assert_eq!((p.chip_chosen, p.chip_chosen_ink), (hex("#102030"), hex("#F0F0F0")));
    assert_eq!(warnings, Vec::<String>::new());
    // A key the file set is the file's, whatever else of its colours would read there.
    let (p, _) = palette("[palette]\nchip_chosen = \"#D0F0D0\"\nstar = \"#F5C400\"\n");
    assert_eq!(p.chip_chosen, hex("#D0F0D0"));
}

#[test]
fn a_file_that_sets_no_key_of_the_entry_draws_every_text_of_it_over_its_floor_and_nothing_is_said() {
    let papers = [
        "#FFFFFF",
        "#FFFDF7",
        "#101018",
        "#000000",
        "#808080",
        "#777777",
        "#E01020",
        "#1030D0",
        "#00C040",
        "#FFE000",
        "#FFFFFF00",
        "#00000000",
        "#20203080",
        "#FFE00040",
    ];
    let desks = ["", "desk = \"#000000\"\n", "desk = \"#FFFFFF80\"\n", "desk = \"#10101000\"\n"];
    let inks = [
        "",
        "ink = \"#F2F2F7\"\nsoft = \"#C9C9D4\"\nquiet = \"#8A8AA0\"\n",
        "ink = \"#111111\"\nsoft = \"#444444\"\nquiet = \"#777777\"\n",
        "ink = \"#808080\"\nsoft = \"#7A7690\"\nquiet = \"#808080\"\n",
        "ink = \"#F2F2F7\"\nsoft = \"#C9C9D4\"\noption = \"#2A2A3A\"\naccent_dark = \"#FFB347\"\nstar = \"#3B3B52\"\n",
    ];
    for paper in papers {
        for (desk, ink) in desks.iter().flat_map(|desk| inks.map(|ink| (desk, ink))) {
            let file = format!("[palette]\npaper = \"{paper}\"\n{desk}{ink}");
            let (p, warnings) = palette(&file);
            for (what, got, floor) in pairs(&p) {
                assert!(got >= floor, "{what} is {got}, under {floor}, in\n{file}{p:?}");
            }
            assert_eq!(warnings, Vec::<String>::new(), "{file}");
        }
    }
}

#[test]
fn a_warning_names_a_key_the_file_set_and_no_other() {
    // Each file sets one key of the entry that its text is not read on, or that is not read: one line, that key,
    // and the first of the palette's colours the pair reads with. Black comes before white where both read.
    let greys = "chip_chosen = \"#757575\"\nchip_chosen_ink = \"#707070\"\nsoft = \"#787878\"\nink = \"#727272\"\n";
    let ring = "entry_ring = \"#FFF0E0\"\ncaret = \"#0040C0\"\n";
    let said = [
        (
            "chip = \"#9C9C9C\"\nsoft = \"#666666\"\n",
            "chip: palette.soft #666666 on it, #9C9C9C, is under 4.5",
            "#EDEBF4",
        ),
        ("entry = \"#2F6FEB\"\n", "entry: palette.ink #2B2350 on it, #2F6FEB, is under 4.5", "#FFFFFF"),
        ("caret = \"#FFE0D0\"\n", "caret: #FFE0D0 on palette.entry #FFFFFF is under 3", "#E4573A"),
        ("entry_hint = \"#F0F0F0\"\n", "entry_hint: #F0F0F0 on palette.entry #FFFFFF is under 2.5", "#9390A6"),
        (ring, "entry_ring: #FFF0E0 on palette.paper #FFFDF7 is under 3", "#0040C0"),
        (
            "chip_chosen_ink = \"#FFE080\"\n",
            "chip_chosen_ink: #FFE080 on palette.chip_chosen #FFD24D is under 4.5",
            "#5E5878",
        ),
        (greys, "chip_chosen_ink: #707070 on palette.chip_chosen #757575 is under 4.5", "#000000"),
    ];
    for (file, start, used) in said {
        let (p, warnings) = palette(&format!("[palette]\n{file}"));
        assert_eq!(warnings, [format!("palette.{start} to 1; {used} is used")], "{file}");
        assert!(pairs(&p).iter().all(|(_, got, floor)| got >= floor), "{file}{:?}", pairs(&p));
    }
    // A file that sets no key of the entry is told nothing, whatever its colours make of the sticker's: a pale
    // `soft` beside the sticker's yellow, a dark chosen chip's `star`, a caret's colour nobody sees on the paper.
    let quiet = [
        "soft = \"#7A7690\"\n",
        "soft = \"#C9C9D4\"\n",
        "star = \"#3B3B52\"\n",
        "accent_dark = \"#FFFDF0\"\n",
        "paper = \"#101018\"\nink = \"#F2F2F7\"\n",
    ];
    for file in quiet {
        let (p, warnings) = palette(&format!("[palette]\n{file}"));
        assert_eq!(warnings, Vec::<String>::new(), "{file}");
        assert!(pairs(&p).iter().all(|(_, got, floor)| got >= floor), "{file}{:?}", pairs(&p));
    }
    // The file that sets only a pale `soft` ends on the sticker's chosen chip and its name, as before.
    let (p, sticker) = (palette("[palette]\nsoft = \"#7A7690\"\n").0, Theme::sticker().palette);
    assert_eq!((p.chip_chosen, p.chip_chosen_ink), (sticker.chip_chosen, sticker.chip_chosen_ink));
}

#[test]
fn a_see_through_paper_is_measured_on_the_desk_it_is_laid_on() {
    // A paper that is all see-through is the desk: a dark ring is read on the sticker's pale desk and is kept,
    // and is not read on a dark desk, whatever colour the paper names.
    let (p, warnings) = palette("[palette]\npaper = \"#00000000\"\nentry_ring = \"#111111\"\n");
    assert_eq!((p.entry_ring, warnings), (hex("#111111"), Vec::new()));
    let (p, warnings) = palette("[palette]\npaper = \"#FFFFFF00\"\ndesk = \"#101010\"\nentry_ring = \"#111111\"\n");
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].starts_with("palette.entry_ring: #111111 on palette.paper #101010 is under 3"), "{warnings:?}");
    assert!(p.entry_ring.contrast(hex("#101010")) >= RING_FLOOR);
    // A chip one half sees through is what it makes of the paper: white on a dark paper, it is a grey the pale
    // name is still read on, and all white it is not.
    let dark = "[palette]\npaper = \"#101018\"\nsoft = \"#F0F0F0\"\n";
    assert_eq!(palette(&format!("{dark}chip = \"#FFFFFF30\"\n")).1, Vec::<String>::new());
    assert_eq!(palette(&format!("{dark}chip = \"#FFFFFF\"\n")).1.len(), 1);
    // The field and a chip are laid on that paper on its desk too: nearly all see-through over a paper that is,
    // on a dark desk, each is the desk's dark, which the sticker's dark words are not read on.
    let desk = "[palette]\npaper = \"#FFFFFF00\"\ndesk = \"#101010\"\n";
    for (key, text) in [("entry", "ink #2B2350"), ("chip", "soft #5E5878")] {
        let (p, warnings) = palette(&format!("{desk}{key} = \"#FFFFFF20\"\n"));
        let start = format!("palette.{key}: palette.{text} on it, #FFFFFF20, is under 4.5 to 1; ");
        assert!(warnings.len() == 1 && warnings[0].starts_with(&start), "{warnings:?}");
        assert!(pairs(&p).iter().all(|(_, got, floor)| got >= floor), "{key}{:?}", pairs(&p));
    }
}

#[test]
fn a_text_one_sees_through_that_no_ground_is_read_under_is_replaced_and_said() {
    // The file of the review of PR 157: typed words that were barely there, and nothing was said.
    let (p, warnings) = palette("[palette]\nink = \"#2B235020\"\n");
    assert_eq!(warnings, ["palette.ink: #2B235020 on palette.entry #FFFFFF is under 4.5 to 1; #2B2350 is used"]);
    assert_eq!((p.ink, p.entry), (hex("#2B2350"), Theme::sticker().palette.entry), "itself, with nothing seen through");
    let (p, warnings) = palette("[palette]\nsoft = \"#5E587830\"\n");
    assert_eq!(warnings, ["palette.soft: #5E587830 on palette.chip #EDEBF4 is under 4.5 to 1; #5E5878 is used"]);
    assert_eq!((p.soft, p.chip_chosen_ink), (hex("#5E5878"), hex("#5E5878")), "and the chosen name is the one used");
    // Where itself is not read either, black or white is: one line, for the text, and its ground is kept.
    let (p, warnings) = palette("[palette]\nentry = \"#101010\"\nink = \"#20202040\"\n");
    assert_eq!(warnings, ["palette.ink: #20202040 on palette.entry #101010 is under 4.5 to 1; #FFFFFF is used"]);
    assert_eq!((p.ink, p.entry), (hex("#FFFFFF"), hex("#101010")));
    // A text one sees through that is read is the file's, and a ground gives way to it before it is replaced.
    let (p, warnings) = palette("[palette]\nink = \"#000000C0\"\nsoft = \"#000000D0\"\n");
    assert_eq!((p.ink, p.soft, warnings), (hex("#000000C0"), hex("#000000D0"), Vec::new()));
    let (p, warnings) = palette("[palette]\nentry = \"#404040\"\noption = \"#FFFFFF\"\nink = \"#000000C0\"\n");
    assert_eq!((p.ink, p.entry, warnings.len()), (hex("#000000C0"), hex("#FFFFFF"), 1), "{warnings:?}");
    assert!(warnings[0].starts_with("palette.entry: "), "{warnings:?}");

    // Every text of the entry is over its floor whatever a file's `ink` and `soft` let through, on any paper,
    // and only those two keys, which the file set, are named, each once.
    let texts = ["#2B235020", "#FFFFFF10", "#00000080", "#80808080", "#FFFFFFC0", "#00000000", "#FF0000A0"];
    for paper in ["#FFFDF7", "#101018", "#808080", "#FFFFFF00", "#20203080", "#FFE000"] {
        for (ink, soft) in texts.iter().flat_map(|ink| texts.map(|soft| (ink, soft))) {
            let file =
                format!("[palette]\npaper = \"{paper}\"\ndesk = \"#30303080\"\nink = \"{ink}\"\nsoft = \"{soft}\"\n");
            let (p, warnings) = palette(&file);
            for (what, got, floor) in pairs(&p) {
                assert!(got >= floor, "{what} is {got}, under {floor}, in\n{file}{p:?}");
            }
            let named =
                |key: &str| warnings.iter().filter(|line| line.starts_with(&format!("palette.{key}: "))).count();
            assert!(named("ink") <= 1 && named("soft") <= 1, "{file}{warnings:?}");
            assert_eq!(named("ink") + named("soft"), warnings.len(), "{file}{warnings:?}");
        }
    }
}

#[test]
fn a_line_tells_the_chosen_chip_when_its_fill_does_not_stand_out() {
    assert_eq!(CHOSEN_FILL_FLOOR, 1.2);
    let edge = |file: &str| {
        let p = palette(&format!("[palette]\n{file}")).0;
        (p.chosen_edge() == p.chip_chosen_ink, p.chosen_edge() == p.chip_chosen)
    };
    let (line, none) = ((true, false), (false, true));
    // One channel far apart is not a fill a person tells: 1.03 to 1, and a line. The sticker's yellow on its
    // lilac chips is 1.22 and has none; a fill at 1.16 has one.
    assert_eq!(edge("chip = \"#FFFFFF\"\nchip_chosen = \"#FFFFCF\"\n"), line);
    assert_eq!(edge(""), none);
    assert_eq!(edge("chip_chosen = \"#C8C4D8\"\n"), none, "1.44 from the chips and 1.67 from the paper");
    assert_eq!(edge("chip_chosen = \"#DDDDDD\"\n"), line, "1.16 from the chips");
    // A fill that is the paper's is told from the chips and still has no outline of its own: a line.
    assert_eq!(edge("chip = \"#A0A0B0\"\nsoft = \"#101010\"\nchip_chosen = \"#FFFDF7\"\n"), line);
    // A fill one nearly sees through is the paper under it, whatever colour it names.
    assert_eq!(edge("chip_chosen = \"#FFD24D10\"\n"), line);
    // And chips one half sees through are the grey they make of the paper, which this fill is not told from.
    assert_eq!(edge("chip = \"#00000080\"\nsoft = \"#000000\"\nchip_chosen = \"#7F7F7F\"\n"), line);
    // The chosen chip is laid on the paper too, never on the other chips: half black on the paper is their grey.
    assert_eq!(edge("chip = \"#7F7E7B\"\nsoft = \"#000000\"\nchip_chosen = \"#00000080\"\n"), line);
}
