//! The entry: its rules with no display, where it is placed, and what the kit holds it to. The layout cases are
//! here and not in `tests/layout.rs`, which is at the size a file may be.

use sticker_kit::entry::{EntryState, MAX_ENTRY_CHARS, MAX_ENTRY_OPTIONS, Made};
use sticker_kit::{Entry, FixedAdvance, PieceKind, Placed, Rect, Scene, Theme, Viewport, layout, layout_entry};

const SIZES: [Viewport; 2] = [Viewport { w: 1440.0, h: 900.0 }, Viewport { w: 960.0, h: 640.0 }];
const MEASURE: FixedAdvance = FixedAdvance(0.56);

fn typed(text: &str) -> EntryState {
    let mut entry = EntryState::new(3, 80);
    entry.replace(None, text);
    entry
}

fn made(text: &str, option: Option<usize>) -> Option<Made> {
    Some(Made { text: text.into(), option })
}

#[test]
fn enter_gives_the_text_and_the_chosen_option_and_the_entry_is_empty_once_they_are_kept() {
    let mut entry = typed("  water the basil ");
    entry.next_option();
    assert_eq!(entry.enter(), made("water the basil", Some(1)));
    assert_eq!(entry.text(), "  water the basil ", "the words stay until they are kept");
    entry.answer(true);
    assert_eq!((entry.text(), entry.caret(), entry.chosen()), ("", 0, Some(1)), "the option stays chosen");
    assert_eq!(entry.enter(), None, "nothing typed is nothing entered");
    entry.replace(None, "   ");
    assert_eq!(entry.enter(), None, "spaces alone are nothing entered");
}

#[test]
fn enter_gives_nothing_while_a_composition_is_marked() {
    let mut entry = typed("买");
    entry.mark(None, "niu nai", None);
    assert_eq!(entry.marked(), Some(3..10));
    assert_eq!(entry.enter(), None, "Enter confirms the composition; it does not enter the text");
    assert_eq!(entry.text(), "买niu nai", "and the composition is still there");
    // The IME then confirms: the marked text is replaced by what was chosen.
    entry.replace(None, "牛奶");
    assert_eq!((entry.text(), entry.marked(), entry.caret()), ("买牛奶", None, 9));
    assert_eq!(entry.enter(), made("买牛奶", Some(0)));
}

#[test]
fn a_composition_replaces_itself_and_ends_marked_or_not() {
    let mut entry = typed("ab");
    entry.left();
    entry.mark(None, "k", None);
    entry.mark(None, "か", None);
    assert_eq!((entry.text(), entry.marked(), entry.caret()), ("aかb", Some(1..4), 4));
    // The caret an IME asks for is counted in UTF-16 from the start of the marked text.
    entry.mark(None, "漢🙂字", Some(3..3));
    assert_eq!((entry.text(), entry.marked(), entry.caret()), ("a漢🙂字b", Some(1..11), 8));
    entry.unmark();
    assert_eq!((entry.text(), entry.marked()), ("a漢🙂字b", None), "unmarking keeps the text");
    entry.mark(None, "x", None);
    entry.mark(None, "", None);
    assert_eq!((entry.text(), entry.marked()), ("a漢🙂字b", None), "an emptied composition is no composition");
}

#[test]
fn ranges_are_utf16_and_text_is_never_cut_inside_a_character() {
    let mut entry = typed("a🙂漢b");
    assert_eq!(entry.len_utf16(), 5);
    assert_eq!(entry.from_utf16(1..3), 1..5, "the emoji is two UTF-16 units and four bytes");
    assert_eq!(entry.to_utf16(1..5), 1..3);
    assert_eq!(entry.to_utf16(entry.caret()..entry.caret()), 5..5);
    assert_eq!(entry.from_utf16(2..2), 5..5, "an offset inside a surrogate pair is after its character");
    assert_eq!(entry.from_utf16(4..99), 8..9, "past the end is the end");
    let (from, to) = (9, 3);
    assert_eq!(entry.from_utf16(from..to), 5..9, "a range given backwards is the same range");
    entry.replace(Some(3..4), "字");
    assert_eq!((entry.text(), entry.caret()), ("a🙂字b", 8));
    entry.replace(Some(1..3), "");
    assert_eq!((entry.text(), entry.caret()), ("a字b", 1));
    entry.replace(Some(7..9), "!");
    assert_eq!(entry.text(), "a字b!");
}

#[test]
fn the_cap_counts_characters_for_typed_pasted_and_composed_text() {
    let mut entry = EntryState::new(0, 5);
    entry.replace(None, "漢字漢字漢字漢字");
    assert_eq!(entry.text(), "漢字漢字漢", "five characters, fifteen bytes");
    entry.replace(None, "x");
    assert_eq!(entry.text(), "漢字漢字漢", "nothing more goes in");
    entry.replace(Some(0..2), "🙂🙂🙂");
    assert_eq!(entry.text(), "🙂🙂漢字漢", "what is replaced makes room, and no more than that");
    entry.replace(Some(0..4), "");
    entry.end();
    entry.mark(None, "pin yin", None);
    assert_eq!((entry.text(), entry.marked()), ("漢字漢pi", Some(9..11)), "a composition is held to the cap too");

    let mut entry = EntryState::new(0, 3);
    entry.replace(None, "ab👨‍👩‍👧");
    assert_eq!(entry.text(), "ab", "a joined emoji that does not fit whole is left out whole");
    entry.replace(None, "e\u{301}x");
    assert_eq!(entry.text(), "ab", "nor is a letter put in without its accent");
}

#[test]
fn what_goes_in_is_one_line_of_characters_a_person_can_see() {
    let entry = typed("a\nb\r\tc\u{7}\u{200B}d\u{202E}e");
    assert_eq!(entry.text(), "a b  cde");
}

#[test]
fn backspace_and_delete_take_a_whole_character() {
    let mut entry = typed("ae\u{301}👍🏽漢");
    entry.backspace();
    assert_eq!(entry.text(), "ae\u{301}👍🏽");
    entry.backspace();
    assert_eq!(entry.text(), "ae\u{301}", "the thumb goes with its skin tone");
    entry.backspace();
    assert_eq!((entry.text(), entry.caret()), ("a", 1), "the letter goes with its accent");
    entry.delete();
    assert_eq!(entry.text(), "a", "nothing is after the caret");
    entry.home();
    entry.backspace();
    assert_eq!(entry.text(), "a", "nothing is before it");
    entry.replace(None, "🇯🇵");
    entry.home();
    entry.delete();
    assert_eq!((entry.text(), entry.caret()), ("a", 0), "a flag is one character");
}

#[test]
fn the_caret_moves_by_whole_characters_and_text_goes_in_at_it() {
    let mut entry = typed("漢e\u{301}🙂");
    entry.left();
    assert_eq!(entry.caret(), 6);
    entry.left();
    assert_eq!(entry.caret(), 3);
    entry.replace(None, "x");
    assert_eq!((entry.text(), entry.caret()), ("漢xe\u{301}🙂", 4));
    entry.right();
    assert_eq!(entry.caret(), 7);
    entry.home();
    entry.left();
    assert_eq!(entry.caret(), 0);
    entry.end();
    entry.right();
    assert_eq!(entry.caret(), 11);
    entry.set_caret(5);
    assert_eq!(entry.caret(), 4, "a place inside a character is the character's start");
}

#[test]
fn the_editing_keys_leave_a_composition_alone() {
    let mut entry = typed("ab");
    entry.mark(None, "ni", None);
    entry.mark(None, "ni", Some(1..1));
    let before = entry.clone();
    let keys: [fn(&mut EntryState); 7] = [
        EntryState::backspace,
        EntryState::delete,
        EntryState::left,
        EntryState::right,
        EntryState::home,
        EntryState::end,
        |entry| entry.set_caret(0),
    ];
    for (nth, key) in keys.into_iter().enumerate() {
        key(&mut entry);
        assert_eq!(entry, before, "key {nth}");
    }
}

#[test]
fn one_option_is_chosen_and_the_choice_goes_round() {
    let mut entry = EntryState::new(3, 80);
    assert_eq!(entry.chosen(), Some(0));
    entry.previous_option();
    assert_eq!(entry.chosen(), Some(2));
    entry.next_option();
    assert_eq!(entry.chosen(), Some(0));
    entry.choose(1);
    entry.choose(3);
    assert_eq!(entry.chosen(), Some(1), "an option that is not there is not chosen");

    let mut none = EntryState::new(0, 80);
    none.next_option();
    none.previous_option();
    none.replace(None, "a");
    assert_eq!((none.chosen(), none.enter()), (None, made("a", None)));
}

fn entry(options: usize, most: usize) -> Entry {
    Entry {
        id: "wish".into(),
        hint: "wish for something".into(),
        options: (0..options).map(|i| format!("project-{i}")).collect(),
        most,
    }
}

#[test]
fn a_scene_s_entry_is_held_to_a_count_of_options_and_a_cap() {
    let mut state = EntryState::of(&entry(20, 100_000));
    assert_eq!((MAX_ENTRY_OPTIONS, MAX_ENTRY_CHARS), (6, 400));
    state.previous_option();
    assert_eq!(state.chosen(), Some(5), "the first six options are the options");
    state.replace(None, &"字".repeat(1000));
    assert_eq!(state.text().chars().count(), 400);
    assert_eq!(EntryState::of(&entry(0, 0)).most(), 400, "no cap given is the kit's cap");
    assert_eq!(EntryState::of(&entry(0, 12)).most(), 12);

    // A scene that changes under the entry holds what is already typed to its new bounds.
    state.fit(&entry(2, 3));
    assert_eq!((state.text(), state.caret(), state.chosen()), ("字字字", 9, Some(1)));
    let mut composing = typed("ab");
    composing.mark(None, "cd", None);
    composing.fit(&entry(3, 3));
    assert_eq!((composing.text(), composing.marked(), composing.caret()), ("abc", None, 3));
}

#[test]
fn the_chosen_option_is_kept_by_its_name_when_the_options_change() {
    let names = |names: &[&str]| names.iter().map(|name| name.to_string()).collect::<Vec<_>>();
    let named =
        |options: &[String]| Entry { id: "wish".into(), hint: String::new(), options: options.to_vec(), most: 80 };
    let was = names(&["garden", "kitchen", "letters"]);
    let mut state = EntryState::of(&named(&was));
    state.choose(1);
    let now = names(&["letters", "garden", "kitchen"]);
    state.keep_option(&was, &now);
    state.fit(&named(&now));
    assert_eq!(state.chosen(), Some(2), "kitchen is still chosen, where it now stands");
    let fewer = names(&["letters", "garden"]);
    state.keep_option(&now, &fewer);
    state.fit(&named(&fewer));
    assert_eq!(state.chosen(), Some(1), "a name that is gone leaves the place, held to the options there are");
    // A name past the six that are offered is not offered, so it is not chosen.
    let many = names(&["a", "b", "c", "d", "e", "f", "garden"]);
    state.keep_option(&fewer, &many);
    state.fit(&named(&many));
    assert_eq!(state.chosen(), Some(1));
}

fn of(placed: &[Placed], kind: PieceKind) -> Vec<&Placed> {
    placed.iter().filter(|p| p.piece == kind).collect()
}

fn outline(piece: &Placed) -> Rect {
    piece.rect.turned_bounds(piece.tilt_degrees)
}

fn scenes() -> Vec<Scene> {
    let long = Scene { title: "W".repeat(400), date_label: "W".repeat(400), ..Scene::sample() };
    let bare = Scene { notes: Vec::new(), ..Scene::sample() };
    vec![Scene::sample(), long, bare, Scene::crowd(50)]
}

#[test]
fn the_entry_has_its_own_place_and_covers_no_piece_of_any_page() {
    for theme in [Theme::sticker(), Theme::plain()] {
        for viewport in SIZES {
            // Six options with long names: the chips fill their row to its end.
            let mut wide = entry(6, 80);
            wide.options.iter_mut().for_each(|option| *option = option.repeat(20));
            let ours = layout_entry(&wide, &theme, viewport, &MEASURE);
            let at = format!("{} at {viewport:?}", theme.name);
            let kinds: Vec<_> = ours.iter().map(|p| (p.piece, p.id.as_str(), p.index)).collect();
            let chip = |index| (PieceKind::Chip, "wish", index);
            assert_eq!(
                kinds,
                [(PieceKind::Entry, "wish", 0), chip(0), chip(1), chip(2), chip(3), chip(4), chip(5)],
                "{at}"
            );
            for mine in &ours {
                assert!(mine.rect.w >= 24.0 && mine.rect.h >= 14.0, "{at}: {mine:?} is too small to use");
                assert!(mine.text.iter().all(|run| run.rect.inside(&mine.rect.grown(0.5))), "{at}: {mine:?}");
                assert!(
                    ours.iter().all(|p: &Placed| std::ptr::eq(p, mine) || !p.rect.overlaps(&mine.rect)),
                    "{at}: {mine:?}"
                );
            }
            for scene in scenes() {
                let page = layout(&scene, &theme, viewport, &MEASURE);
                let paper = of(&page, PieceKind::Paper)[0].rect;
                for mine in &ours {
                    assert!(mine.rect.inside(&paper), "{at}: {mine:?} leaves the paper");
                    for other in page.iter().filter(|p| p.piece != PieceKind::Paper) {
                        assert!(!mine.rect.overlaps(&outline(other)), "{at}: {mine:?} covers {other:?}");
                    }
                }
            }
        }
    }
}

#[test]
fn the_hint_and_the_options_are_cut_to_their_room_and_the_options_counted() {
    for viewport in SIZES {
        let mut entry = entry(20, 80);
        entry.hint = "what do you wish for ".repeat(60);
        entry.options[1] = "a-project-with-a-very-long-name-indeed".repeat(20);
        let placed = layout_entry(&entry, &Theme::sticker(), viewport, &MEASURE);
        let field = of(&placed, PieceKind::Entry)[0];
        let chips = of(&placed, PieceKind::Chip);
        assert_eq!(chips.len(), MAX_ENTRY_OPTIONS, "the first six options are drawn, each on a chip");
        assert_eq!(field.text.len(), 1);
        assert!(field.text[0].text.ends_with('…') && field.text[0].rect.inside(&field.rect), "{field:?}");
        assert!(chips[1].text[0].text.ends_with('…'), "{:?}", chips[1]);
        for (i, chip) in chips.iter().enumerate() {
            assert!(chip.text[0].rect.inside(&chip.rect.grown(0.5)), "{chip:?}");
            assert!(chip.rect.right() <= field.rect.right() + 0.5, "{chip:?} ends where the field ends");
            assert!(chips[i + 1..].iter().all(|next| chip.rect.right() <= next.rect.x), "chips stand in a row");
        }
    }
    let bare = Entry { id: "wish".into(), ..Entry::default() };
    let placed = layout_entry(&bare, &Theme::plain(), SIZES[1], &MEASURE);
    assert_eq!(placed.len(), 1, "no hint and no option is a field alone");
    assert_eq!((placed[0].piece, placed[0].text.len()), (PieceKind::Entry, 0));

    let mut refused = Theme::sticker();
    refused.metrics.columns = 0;
    assert_eq!(layout_entry(&entry(2, 80), &refused, SIZES[0], &MEASURE), [], "a refused theme places nothing");
}

// A theme whose notes begin early leaves the entry a narrow row. A chip is then held to its share of it, under the
// width a chip would otherwise have at least, so that six of them still end where the field ends.
#[test]
fn a_chip_is_never_wider_than_its_share_of_a_narrow_row() {
    let mut narrow = Theme::sticker();
    narrow.metrics.note_left = narrow.metrics.content_left + 254.0 + 24.0 + 170.0;
    assert_eq!(narrow.check(), Ok(()));
    for viewport in SIZES {
        let placed = layout_entry(&entry(6, 80), &narrow, viewport, &MEASURE);
        let field = of(&placed, PieceKind::Entry)[0].rect;
        let chips = of(&placed, PieceKind::Chip);
        assert_eq!(chips.len(), 6, "{viewport:?}");
        assert!(chips[5].rect.right() <= field.right() + 0.5, "{viewport:?}: {:?} ends past {field:?}", chips[5]);
        assert!(chips.iter().all(|chip| chip.rect.w < 1.5 * chip.rect.h), "the row was to be narrow: {chips:?}");
    }
}

#[test]
fn both_themes_colour_the_entry_and_a_theme_file_without_its_keys_falls_back() {
    for theme in [Theme::sticker(), Theme::plain()] {
        let p = &theme.palette;
        assert_ne!(p.chip, p.chip_chosen, "{}: the chosen chip is told apart", theme.name);
        assert_ne!(p.entry, p.caret, "{}: the caret shows on the entry", theme.name);
        assert_eq!(theme.check(), Ok(()));
    }
    let old = Theme::from_toml("[palette]\nink = \"#102030\"\n").expect("a theme from before the entry loads");
    let sticker = Theme::sticker().palette;
    let got = &old.theme.palette;
    assert!(old.warnings.is_empty(), "{:?}", old.warnings);
    assert_eq!((got.entry, got.entry_edge, got.caret), (sticker.entry, sticker.entry_edge, sticker.caret));
    assert_eq!((got.chip, got.chip_chosen), (sticker.chip, sticker.chip_chosen));
    let new = Theme::from_toml("[palette]\nchip_chosen = \"#102030\"\n").expect("the new keys are known");
    assert!(new.warnings.is_empty(), "{:?}", new.warnings);
    assert_eq!(new.theme.palette.chip_chosen, sticker_kit::Color::rgb(0x102030));
}
