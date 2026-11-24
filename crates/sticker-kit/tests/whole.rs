//! A whole choice, and a whole list: every word of its options, or of its rows, is drawn. Never a cut one.

use sticker_kit::{FixedAdvance, Mark, Measure, Note, PieceKind, Placed, Rect, Scene, Theme, Viewport, layout};

const DEFAULT: Viewport = Viewport { w: 1440.0, h: 900.0 };
const SMALL: Viewport = Viewport { w: 960.0, h: 640.0 };

fn place(scene: &Scene, theme: &Theme, viewport: Viewport) -> Vec<Placed> {
    layout(scene, theme, viewport, &FixedAdvance(0.56))
}

fn of(placed: &[Placed], kind: PieceKind) -> Vec<&Placed> {
    placed.iter().filter(|p| p.piece == kind).collect()
}

/// The notes placed, by id.
fn notes(placed: &[Placed]) -> Vec<String> {
    of(placed, PieceKind::Note).iter().map(|note| note.id.clone()).collect()
}

// A whole choice is where what is read is what is acted on: no word of an option may be hidden.
#[test]
fn a_whole_choice_shows_every_word_of_its_options_however_many_stand_on_the_page() {
    let long: String = (0..40).map(|i| format!("w{i:03}")).collect::<Vec<_>>().join(" ");
    assert_eq!(long.chars().count(), 199);
    let choice = |id: &str| Note::Choice {
        id: id.into(),
        title: "pick one".into(),
        options: vec![long.clone(), long.replace('w', "v")],
        footer: "a footer".into(),
        struck_first: true,
        whole: true,
    };
    let said = |option: &Placed| option.text.iter().map(|run| run.text.as_str()).collect::<Vec<_>>().join(" ");
    let mut scene = Scene::sample();
    for theme in [Theme::sticker(), Theme::plain()] {
        for viewport in [DEFAULT, SMALL] {
            scene.notes = vec![choice("a")];
            let placed = place(&scene, &theme, viewport);
            let options: Vec<String> = of(&placed, PieceKind::Option).iter().map(|option| said(option)).collect();
            assert_eq!(options, [long.clone(), long.replace('w', "v")], "every word, and no ellipsis");
            let inside = |piece: &Placed| piece.rect.x >= 0.0 && piece.rect.x + piece.rect.w <= viewport.w;
            assert!(placed.iter().filter(|piece| piece.piece == PieceKind::Option).all(inside));
            // Six of them are six cards, each with every word of both its options.
            scene.notes = (0..6).map(|i| choice(&format!("c{i}"))).collect();
            let placed = place(&scene, &theme, viewport);
            assert_eq!(notes(&placed), ["c0", "c1", "c2", "c3", "c4", "c5"]);
            let options: Vec<String> = of(&placed, PieceKind::Option).iter().map(|option| said(option)).collect();
            assert_eq!(options.len(), 12);
            assert!(options.chunks(2).all(|pair| pair == [long.clone(), long.replace('w', "v")]), "{options:?}");
        }
    }
}

// The longest options a caller is known to send, 200 characters each, in the widest characters there are: the choice
// is drawn in full under a list and a plain note, at both sizes, at an option's own size.
#[test]
fn two_options_of_200_wide_characters_are_drawn_in_full_under_other_notes() {
    let options = vec!["请".repeat(200), "W".repeat(200)];
    let rows = (0..9).map(|i| (format!("row {i}"), "of nine".to_string())).collect();
    let (title, footer) = ("pick one".to_string(), "a footer".to_string());
    let choice =
        Note::Choice { id: "a".into(), title, options: options.clone(), footer, struck_first: false, whole: true };
    let mut scene = Scene::sample();
    scene.notes = vec![
        Note::List { title: "a list".into(), rows },
        Note::Plain { title: "a plain note".into(), body: "with a line of its own".into() },
        choice,
    ];
    for theme in [Theme::sticker(), Theme::plain()] {
        for viewport in [DEFAULT, SMALL] {
            let placed = place(&scene, &theme, viewport);
            assert_eq!(notes(&placed), ["note-0", "note-1", "a"]);
            let drawn: Vec<String> = of(&placed, PieceKind::Option)
                .iter()
                .map(|option| option.text.iter().map(|run| run.text.as_str()).collect())
                .collect();
            assert_eq!(drawn, options, "every character, in order");
            let sizes: Vec<f32> = of(&placed, PieceKind::Option).iter().flat_map(|o| &o.text).map(|r| r.size).collect();
            assert!(sizes.iter().all(|size| *size == sizes[0]), "none is written smaller for room: {sizes:?}");
        }
    }
}

// The bubble of an option is drawn from the option's own box, and its words from their runs: at every size the
// layout writes them in, each line measured at its run's own size ends inside the bubble's padding, and the bubble
// lies inside its sheet. Open and settled, both themes, both sizes.
#[test]
fn every_line_of_a_whole_option_lies_inside_its_bubble_and_the_bubble_inside_its_sheet() {
    let measure = FixedAdvance(0.56);
    let words = "please add a retry to the uploader so that a file which fails is sent again ";
    let options = vec![words.repeat(3).chars().take(200).collect::<String>(), "请".repeat(200)];
    let rows = (0..3).map(|i| (format!("row {i}"), "of three".to_string())).collect();
    let mut scene = Scene::sample();
    for (theme, viewport, struck_first) in [Theme::sticker(), Theme::plain()]
        .into_iter()
        .flat_map(|theme| [DEFAULT, SMALL].map(|viewport| (theme.clone(), viewport)))
        .flat_map(|(theme, viewport)| [false, true].map(|struck| (theme.clone(), viewport, struck)))
    {
        let (title, footer, options) = ("pick one".to_string(), "a footer".to_string(), options.clone());
        scene.notes = vec![
            Note::List { title: "a list".into(), rows: Vec::clone(&rows) },
            Note::Plain { title: "a plain note".into(), body: "with a line of its own".into() },
            Note::Choice { id: "a".into(), title, options, footer, struck_first, whole: true },
        ];
        let placed = place(&scene, &theme, viewport);
        let at = format!("{}x{}, struck {struck_first}", viewport.w, viewport.h);
        let sheet = placed.iter().find(|piece| piece.piece == PieceKind::Note && piece.id == "a").expect(&at);
        let pad = 14.0 * (viewport.w / 1440.0).min(viewport.h / 900.0);
        let bubbles = of(&placed, PieceKind::Option);
        assert_eq!(bubbles.len(), 2, "{at}");
        for bubble in bubbles {
            let (left, right) = (bubble.rect.x, bubble.rect.x + bubble.rect.w);
            assert!(left >= sheet.rect.x && right <= sheet.rect.x + sheet.rect.w, "{at}: the bubble is in the sheet");
            for run in &bubble.text {
                let end = run.rect.x + measure.width(&run.text, run.role, run.size);
                assert!(
                    run.rect.x >= left + pad - 0.01 && end <= right - pad + 0.01,
                    "{at}: {:?} ends at {end}",
                    run.text
                );
                assert!((run.rect.w - (end - run.rect.x)).abs() < 0.01, "{at}: the run is as wide as its words");
            }
        }
    }
}

// A whole list is the same promise for a row: drawn after its mark to its last word. It stands where the scene has
// it, as high as its rows; the rows past the twelfth are counted in its own last line, so a list of any length
// costs the same.
#[test]
fn a_whole_list_draws_each_row_to_its_last_word_where_the_scene_has_it_and_counts_the_rows_past_the_twelfth() {
    let long: String = (0..40).map(|i| format!("w{i:03}")).collect::<Vec<_>>().join(" ");
    let row = |i: usize| if i.is_multiple_of(2) { long.replace('w', &i.to_string()) } else { format!("row {i}") };
    let mut scene = Scene::sample();
    for (theme, viewport, n) in [Theme::sticker(), Theme::plain()]
        .into_iter()
        .flat_map(|theme| [DEFAULT, SMALL].map(|viewport| (theme.clone(), viewport)))
        .flat_map(|(theme, viewport)| [1, 2, 8, 12, 40].map(|n| (theme.clone(), viewport, n)))
    {
        let rows: Vec<(Mark, String)> = (0..n).map(|i| (Mark::Busy, row(i))).collect();
        scene.notes = vec![
            Note::List { title: "a list".into(), rows: vec![("a row".into(), String::new())] },
            Note::Whole { title: "a whole list".into(), rows: rows.clone() },
            Note::Plain { title: "the note after".into(), body: "is under it".into() },
        ];
        let placed = place(&scene, &theme, viewport);
        let at = format!("{n} rows at {}x{}", viewport.w, viewport.h);
        assert_eq!(notes(&placed), ["note-0", "note-1", "note-2"], "{at}");
        let sheet = placed.iter().find(|piece| piece.piece == PieceKind::Note && piece.id == "note-1").expect(&at);
        let marks: Vec<&Placed> = of(&placed, PieceKind::Mark);
        assert_eq!(marks.iter().map(|mark| mark.index).collect::<Vec<_>>(), (0..n.min(12)).collect::<Vec<_>>(), "{at}");
        let lines: Vec<&str> = sheet.text[1..].iter().map(|run| run.text.as_str()).collect();
        let drawn: Vec<String> = rows.iter().take(12).map(|row| row.1.clone()).collect();
        let more = (n > 12).then(|| format!("+{} more", n - 12));
        assert_eq!(lines.join(" "), drawn.iter().chain(&more).cloned().collect::<Vec<_>>().join(" "), "{at}");
        // Every row is written at a row's own size.
        let own = placed.iter().find(|piece| piece.id == "note-0").map(|list| list.text[2].size);
        let rows_only = &sheet.text[1..sheet.text.len() - usize::from(n > 12)];
        assert!(rows_only.iter().all(|run| Some(run.size) == own), "{at}: {own:?}");
        let inside = |r: Rect| {
            let s = sheet.rect;
            r.x >= s.x && r.x + r.w <= s.x + s.w && r.y >= s.y && r.y + r.h <= s.y + s.h
        };
        for piece in marks.iter().copied().chain([sheet]) {
            assert!(inside(piece.rect) && piece.text.iter().all(|run| inside(run.rect)), "{at}: {piece:?}");
        }
        let after = placed.iter().find(|piece| piece.piece == PieceKind::Note && piece.id == "note-2").expect(&at);
        assert!(after.rect.y >= sheet.rect.y + sheet.rect.h, "{at}: the note after is under it");
    }
}
