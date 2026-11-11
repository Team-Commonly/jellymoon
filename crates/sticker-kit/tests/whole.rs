//! A whole choice: every word of its options is drawn, or the note is left out and counted. Never a cut option.

use sticker_kit::{FixedAdvance, MORE_NOTES_ID, Measure, Note, PieceKind, Placed, Scene, Theme, Viewport, layout};

const DEFAULT: Viewport = Viewport { w: 1440.0, h: 900.0 };
const SMALL: Viewport = Viewport { w: 960.0, h: 640.0 };

fn place(scene: &Scene, theme: &Theme, viewport: Viewport) -> Vec<Placed> {
    layout(scene, theme, viewport, &FixedAdvance(0.56))
}

fn of(placed: &[Placed], kind: PieceKind) -> Vec<&Placed> {
    placed.iter().filter(|p| p.piece == kind).collect()
}

/// The notes placed, by id, and what the last sheet says when notes were left out.
fn notes_and_count(placed: &[Placed]) -> (Vec<String>, Option<String>) {
    let notes = of(placed, PieceKind::Note);
    let more = notes.iter().find(|note| note.id == MORE_NOTES_ID).map(|note| note.text[0].text.clone());
    (notes.iter().filter(|note| note.id != MORE_NOTES_ID).map(|note| note.id.clone()).collect(), more)
}

// A whole choice is where what is read is what is acted on: no word of an option may be hidden.
#[test]
fn a_whole_choice_shows_every_word_of_its_options_or_is_left_out_and_counted() {
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
            // With no room for all of them, one is left out whole: no option of it is placed, and it is counted.
            scene.notes = (0..6).map(|i| choice(&format!("c{i}"))).collect();
            let placed = place(&scene, &theme, viewport);
            let (shown, count) = notes_and_count(&placed);
            assert!(shown.len() < 6 && count.is_some(), "{shown:?}");
            for option in of(&placed, PieceKind::Option) {
                assert!(shown.contains(&option.id) && !said(option).contains('…'), "{}: {}", option.id, said(option));
            }
        }
    }
}

// The longest options a caller is known to send, 200 characters each, in the widest characters there are: the choice
// is drawn in full beside a list and a plain note, at both sizes, by writing its options smaller.
#[test]
fn two_options_of_200_wide_characters_are_drawn_in_full_beside_other_notes() {
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
            assert_eq!(notes_and_count(&placed), (vec!["note-0".into(), "note-1".into(), "a".into()], None));
            let drawn: Vec<String> = of(&placed, PieceKind::Option)
                .iter()
                .map(|option| option.text.iter().map(|run| run.text.as_str()).collect())
                .collect();
            assert_eq!(drawn, options, "every character, in order");
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
