//! Where the entry is placed: the field under the title, the chips under the field, and what each is cut to.

use sticker_kit::entry::MAX_ENTRY_OPTIONS;
use sticker_kit::{Entry, FixedAdvance, PieceKind, Placed, Rect, Scene, Theme, Viewport, layout, layout_entry};

const SIZES: [Viewport; 2] = [Viewport { w: 1440.0, h: 900.0 }, Viewport { w: 960.0, h: 640.0 }];
const MEASURE: FixedAdvance = FixedAdvance(0.56);

fn entry(options: usize, most: usize) -> Entry {
    Entry {
        id: "wish".into(),
        hint: "wish for something".into(),
        options: (0..options).map(|i| format!("project-{i}")).collect(),
        most,
        action: String::new(),
    }
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

// The entry stands where wishes are listed: in the left column, under the title, the chips under the field.
#[test]
fn the_entry_stands_under_the_title_in_the_left_column_and_covers_no_piece_of_any_page() {
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
                let (paper, title) = (of(&page, PieceKind::Paper)[0].rect, of(&page, PieceKind::Title)[0].rect);
                let (field, k) = (ours[0].rect, paper.w / 1344.0);
                assert!((field.x - title.x).abs() < 0.5, "{at}: {field:?} begins where {title:?} does");
                let under = field.y - title.bottom();
                assert!((0.0..=12.0 * k).contains(&under), "{at}: {field:?} is {under} under {title:?}");
                for chip in &ours[1..] {
                    let under = chip.rect.y - field.bottom();
                    assert!((0.0..=44.0 * k).contains(&under), "{at}: {chip:?} is {under} under {field:?}");
                }
                for note in of(&page, PieceKind::Note) {
                    assert!(field.right() < outline(note).x, "{at}: {field:?} is in the column of {note:?}");
                }
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
        assert!(chips[1].text[0].text.contains('…'), "{:?}", chips[1]);
        for (i, chip) in chips.iter().enumerate() {
            assert!(chip.text[0].rect.inside(&chip.rect.grown(0.5)), "{chip:?}");
            assert!(chip.rect.right() <= field.rect.right() + 0.5, "{chip:?} ends where the field ends");
            let row = if i < 3 { 0 } else { 3 };
            assert_eq!(chip.rect.y, chips[row].rect.y, "names too long for one row stand in two rows of three");
            assert!(chips[i + 1..row + 3].iter().all(|next| chip.rect.right() <= next.rect.x), "{chip:?}");
        }
        assert!(chips[2].rect.bottom() <= chips[3].rect.y && chips[3].rect.x == field.rect.x);
        // The count of the options past the sixth stands in the second row: the first keeps the field's width.
        assert!(chips[2].rect.right() >= field.rect.right() - 0.5, "{:?} ends short of {field:?}", chips[2]);
        // Short names stand in one row, each as wide as its name asks.
        let short = layout_entry(&self::entry(3, 80), &Theme::sticker(), viewport, &MEASURE);
        let chips = of(&short, PieceKind::Chip);
        assert!(chips.iter().all(|chip| chip.rect.y == chips[0].rect.y && !chip.text[0].text.ends_with('…')));
    }
    let bare = Entry { id: "wish".into(), ..Entry::default() };
    let placed = layout_entry(&bare, &Theme::plain(), SIZES[1], &MEASURE);
    assert_eq!(placed.len(), 1, "no hint and no option is a field alone");
    assert_eq!((placed[0].piece, placed[0].text.len()), (PieceKind::Entry, 0));

    let mut refused = Theme::sticker();
    refused.metrics.columns = 0;
    assert_eq!(layout_entry(&entry(2, 80), &refused, SIZES[0], &MEASURE), [], "a refused theme places nothing");
}

// A theme whose big character stands far left leaves the entry a narrow row. A chip is then held to its share of
// its row, so that six of them, in two rows, still end where the field ends. With no room at all there is no entry:
// half a field is nothing to type in.
#[test]
fn a_chip_is_never_wider_than_its_share_of_a_narrow_row() {
    let mut narrow = Theme::sticker();
    narrow.metrics.hero_x = narrow.metrics.content_left + 170.0 + 24.0 + narrow.metrics.hero_size / 2.0;
    assert_eq!(narrow.check(), Ok(()));
    for viewport in SIZES {
        let placed = layout_entry(&entry(6, 80), &narrow, viewport, &MEASURE);
        let field = of(&placed, PieceKind::Entry)[0].rect;
        let chips = of(&placed, PieceKind::Chip);
        assert_eq!(chips.len(), 6, "{viewport:?}");
        assert!(chips[5].rect.right() <= field.right() + 0.5, "{viewport:?}: {:?} ends past {field:?}", chips[5]);
        assert!(chips[2].rect.right() <= field.right() + 0.5, "{viewport:?}: {:?} ends past {field:?}", chips[2]);
        assert!(chips.iter().all(|chip| chip.rect.w <= field.w / 3.0), "each has a third of its row: {chips:?}");
        assert!(chips[3].rect.y > chips[2].rect.y, "the row was to be narrow: {chips:?}");
        let page = layout(&Scene::sample(), &narrow, viewport, &MEASURE);
        let hero = of(&page, PieceKind::Hero)[0].rect;
        assert!(field.right() < hero.x, "{viewport:?}: {field:?} ends short of {hero:?}");
    }
    narrow.metrics.hero_x -= 20.0;
    assert_eq!((narrow.check(), layout_entry(&entry(6, 80), &narrow, SIZES[0], &MEASURE)), (Ok(()), Vec::new()));
}

// The options past the sixth are counted, as the rows of a list past its room are: `+N more`, in words, after the
// last chip. It is no option, and it never costs a chip its place.
#[test]
fn the_options_past_the_sixth_are_counted_in_words_after_the_last_chip() {
    for viewport in SIZES {
        for (options, said) in [(7, "+1 more"), (20, "+14 more"), (100, "+94 more")] {
            let mut entry = entry(options, 80);
            entry.options[4] = "a-project-with-a-very-long-name-indeed".repeat(20);
            let placed = layout_entry(&entry, &Theme::sticker(), viewport, &MEASURE);
            let (field, chips) = (of(&placed, PieceKind::Entry)[0].rect, of(&placed, PieceKind::Chip));
            let [more] = of(&placed, PieceKind::More)[..] else { panic!("one count: {placed:?}") };
            assert_eq!((more.text[0].text.as_str(), more.index, more.id.as_str()), (said, options - 6, "wish"));
            assert_eq!(placed.last().map(|last| last.piece), Some(PieceKind::More), "it is placed after the chips");
            assert!(more.text[0].rect.inside(&more.rect.grown(0.5)), "{more:?}");
            assert!(more.rect.right() <= field.right() + 0.5, "{more:?} ends where the field ends");
            assert!(more.rect.y == chips[5].rect.y && chips[5].rect.right() <= more.rect.x, "{more:?}");
            assert_eq!(chips.len(), MAX_ENTRY_OPTIONS);
            assert!(chips[4].text[0].text.contains('…') && chips[3].rect.y > chips[2].rect.y, "{chips:?}");
            assert!(chips.iter().all(|chip| chip.rect.w >= chip.rect.h * 1.5 - 0.01), "{chips:?}");
        }
        // Names and a count that fit one row stand in one row, and no name is cut for the count.
        let short =
            layout_entry(&named(&["a", "b", "c", "d", "e", "f", "g", "h"]), &Theme::plain(), viewport, &MEASURE);
        let (chips, more) = (of(&short, PieceKind::Chip), of(&short, PieceKind::More));
        assert_eq!(more[0].text[0].text, "+2 more");
        assert!(chips.iter().all(|chip| chip.rect.y == more[0].rect.y && chip.text[0].text.chars().count() == 1));
        // Six options are six chips, and nothing is counted.
        let six = layout_entry(&entry(6, 80), &Theme::sticker(), viewport, &MEASURE);
        assert!(of(&six, PieceKind::More).is_empty());
    }
    // The count is beside the second row only, so it needs room for that row's three chips at their least width,
    // not for all six: a field of 240 design pixels holds three and the count, and would not hold six.
    let mut between = Theme::sticker();
    between.metrics.hero_x = between.metrics.content_left + 240.0 + 24.0 + between.metrics.hero_size / 2.0;
    assert_eq!(between.check(), Ok(()));
    for viewport in SIZES {
        let placed = layout_entry(&entry(9, 80), &between, viewport, &MEASURE);
        let (field, chips) = (of(&placed, PieceKind::Entry)[0].rect, of(&placed, PieceKind::Chip));
        let [more] = of(&placed, PieceKind::More)[..] else { panic!("{viewport:?}: one count: {placed:?}") };
        assert_eq!((chips.len(), more.text[0].text.as_str()), (6, "+3 more"), "{viewport:?}");
        // 1.5 is the layout's private `LEAST` (`src/layout/entry.rs`), a chip's least width in chip heights: a
        // change to that constant changes this number with it.
        let (least, aside) = (chips[0].rect.h * 1.5, more.rect.w);
        assert!(6.0 * least + aside > field.w, "{viewport:?}: the field was to be too narrow for six: {field:?}");
        assert!(more.rect.right() <= field.right() + 0.5 && chips[5].rect.right() <= more.rect.x, "{more:?}");
        assert!(chips.iter().all(|chip| chip.rect.w >= least - 0.01), "{viewport:?}: {chips:?}");
    }
    // Where the last row could not hold its chips at their least width beside the count, there is no count:
    // the six options stay there to be chosen.
    let mut narrow = Theme::sticker();
    narrow.metrics.hero_x = narrow.metrics.content_left + 170.0 + 24.0 + narrow.metrics.hero_size / 2.0;
    for viewport in SIZES {
        let placed = layout_entry(&entry(9, 80), &narrow, viewport, &MEASURE);
        assert_eq!((of(&placed, PieceKind::Chip).len(), of(&placed, PieceKind::More).len()), (6, 0), "{viewport:?}");
    }
}

// A name is drawn as text: what would be markup or a link elsewhere is the chip's words, and a character that
// cannot be seen is not drawn.
#[test]
fn an_option_is_words_and_nothing_else() {
    let odd = named(&["<b>bold</b>", "[x](javascript:y)", "bell\u{7}rung", "line\nbreak"]);
    let placed = layout_entry(&odd, &Theme::sticker(), SIZES[0], &MEASURE);
    let said = said(&placed);
    assert_eq!(said[..2], ["<b>bold</b>", "[x](javascript:y)"]);
    assert!(said.iter().all(|name| !name.chars().any(char::is_control)), "{said:?}");
    assert!(said[2].starts_with("bell") && said[2].ends_with("rung") && said[3].starts_with("line"), "{said:?}");
    assert_eq!(placed.len(), 5, "the field and four chips: {placed:?}");
}

fn named(names: &[&str]) -> Entry {
    let options = names.iter().map(|name| name.to_string()).collect();
    Entry { id: "wish".into(), hint: String::new(), options, most: 80, action: String::new() }
}

/// The chips, row by row, and the field they stand under.
fn rows(placed: &[Placed]) -> (Vec<Vec<&Placed>>, Rect) {
    let mut rows: Vec<Vec<&Placed>> = Vec::new();
    for chip in of(placed, PieceKind::Chip) {
        match rows.last_mut().filter(|row| row[0].rect.y == chip.rect.y) {
            Some(row) => row.push(chip),
            None => rows.push(vec![chip]),
        }
    }
    (rows, of(placed, PieceKind::Entry)[0].rect)
}

fn said(placed: &[Placed]) -> Vec<&str> {
    of(placed, PieceKind::Chip).iter().map(|chip| chip.text[0].text.as_str()).collect()
}

// A row's spare width goes to its long chips: a chip is as wide as its name while its row has the room, and names
// are cut only in a row that is filled to its end.
#[test]
fn no_name_is_cut_while_its_row_has_room() {
    let three = ["kitchen-sink-app-frontend", "a", "b"];
    let six = ["alpha", "beta", "garden-letters", "photo-sorter", "parser", "notebook"];
    let long = ["alpha", "garden-letters", "kitchen-sink-app-frontend", "notes-private-journal", "beta", "jellymoon"];
    for (theme, viewport) in [Theme::sticker(), Theme::plain()].iter().flat_map(|theme| SIZES.map(|size| (theme, size)))
    {
        let at = format!("{} at {viewport:?}", theme.name);
        let placed = layout_entry(&named(&three), theme, viewport, &MEASURE);
        assert_eq!((said(&placed), rows(&placed).0.len()), (three.to_vec(), 1), "{at}: whole, in one row");
        let placed = layout_entry(&named(&six), theme, viewport, &MEASURE);
        assert_eq!((said(&placed), rows(&placed).0.len()), (six.to_vec(), 2), "{at}: whole, in two rows");

        let placed = layout_entry(&named(&long), theme, viewport, &MEASURE);
        let (rows, field) = rows(&placed);
        assert_eq!(rows.iter().map(Vec::len).collect::<Vec<_>>(), [3, 3], "{at}");
        for row in &rows {
            let spare = field.right() - row[2].rect.right();
            let cut: Vec<_> = row.iter().filter(|chip| chip.text[0].text.contains('…')).collect();
            assert!(spare >= -0.5 && (cut.is_empty() || spare < 0.5), "{at}: {cut:?} cut with {spare} to spare");
            // The short names of a row are whole, and what is cut is cut to one width.
            let widest = row.iter().map(|chip| chip.rect.w).fold(0.0, f32::max);
            assert!(cut.iter().all(|chip| (chip.rect.w - widest).abs() < 0.01), "{at}: {row:?}");
        }
        assert!(said(&placed).contains(&"alpha") && said(&placed).contains(&"beta"), "{at}: {:?}", said(&placed));
    }
}

// A name too long for its room is cut in its middle, so two projects that begin alike are still told apart.
#[test]
fn two_names_with_a_long_shared_start_read_differently_when_cut() {
    let ends = ["frontend", "backend", "docs", "worker", "mobile", "design"];
    let names = ends.map(|end| format!("kitchen-sink-app-{end}"));
    let entry = named(&names.each_ref().map(String::as_str));
    for (theme, viewport) in [Theme::sticker(), Theme::plain()].iter().flat_map(|theme| SIZES.map(|size| (theme, size)))
    {
        let placed = layout_entry(&entry, theme, viewport, &MEASURE);
        let said = said(&placed);
        for (n, cut) in said.iter().enumerate() {
            let (start, end) = cut.split_once('…').unwrap_or_else(|| panic!("{cut} was to be cut"));
            assert!(start.len() >= 3 && end.len() >= 4, "{cut} keeps its start and its end");
            assert!(names[n].starts_with(start) && names[n].ends_with(end), "{cut} is of {}", names[n]);
            assert!(said[..n].iter().all(|other| other != cut), "{cut} is drawn twice: {said:?}");
        }
    }
}

// Two numbers of the entry, stated: the least a chip is wide, and what the field keeps from the big character.
#[test]
fn a_chip_is_never_narrower_than_one_and_a_half_times_its_height_and_the_field_ends_24_short_of_the_big_character() {
    for (theme, viewport) in [Theme::sticker(), Theme::plain()].iter().flat_map(|theme| SIZES.map(|size| (theme, size)))
    {
        let at = format!("{} at {viewport:?}", theme.name);
        let placed = layout_entry(&named(&["a", "b", "garden-letters"]), theme, viewport, &MEASURE);
        let page = layout(&Scene::sample(), theme, viewport, &MEASURE);
        let k = of(&page, PieceKind::Paper)[0].rect.w / 1344.0;
        let (rows, field) = rows(&placed);
        let [short, _, long] = rows[0][..] else { panic!("{at}: three chips in one row") };
        assert!((short.rect.w - 1.5 * short.rect.h).abs() < 0.01, "{at}: {short:?}");
        assert!((long.rect.w - (long.text[0].rect.w + 20.0 * k)).abs() < 0.01, "{at}: {long:?} is as wide as its name");
        assert!(long.rect.w > 1.5 * long.rect.h, "{at}");
        let hero = of(&page, PieceKind::Hero)[0].rect;
        assert!((hero.x - field.right() - 24.0 * k).abs() < 0.01, "{at}: {field:?} ends 24 short of {hero:?}");
    }
}
