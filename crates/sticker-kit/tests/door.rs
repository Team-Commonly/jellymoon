//! The kit's door on the layout side: the exact number of notes and options laid out, and a theme built in code.

use sticker_kit::{FixedAdvance, MAX_NOTES, Note, PieceKind, Placed, Scene, Theme, ThemeError, Viewport, layout};

const DEFAULT: Viewport = Viewport { w: 1440.0, h: 900.0 };
const MEASURE: FixedAdvance = FixedAdvance(0.56);

/// A page ten times as tall as the default, so that room is never what stops a note.
fn tall() -> (Theme, Viewport) {
    let mut theme = Theme::sticker();
    theme.metrics.height = 10_000.0;
    (theme, Viewport { w: 1440.0, h: 10_000.0 })
}

fn notes(placed: &[Placed]) -> Vec<(String, String)> {
    let of = |note: &Placed| note.text.iter().map(|run| run.text.as_str()).collect::<Vec<_>>().join(" | ");
    placed.iter().filter(|p| p.piece == PieceKind::Note).map(|note| (note.id.clone(), of(note))).collect()
}

#[test]
fn twelve_notes_are_laid_out_and_the_thirteenth_is_counted() {
    assert_eq!(MAX_NOTES, 12);
    let (theme, viewport) = tall();
    let mut scene = Scene::sample();
    let plain = |i: usize| Note::Plain { title: format!("note {i}"), body: String::new() };
    scene.notes = (0..12).map(plain).collect();
    let twelve = notes(&layout(&scene, &theme, viewport, &MEASURE));
    assert_eq!(twelve.len(), 12, "with room for all, twelve are twelve sheets");
    assert_eq!(twelve[11], ("note-11".to_string(), "note 11".to_string()));

    scene.notes.push(plain(12));
    let thirteen = notes(&layout(&scene, &theme, viewport, &MEASURE));
    assert_eq!(thirteen[..12], twelve[..], "the first twelve are the same");
    assert_eq!(thirteen[12..], [("more".to_string(), "+1 more of 13 notes".to_string())]);
}

#[test]
fn four_options_are_laid_out_and_the_fifth_is_not() {
    let (theme, viewport) = tall();
    let mut scene = Scene::sample();
    for (given, drawn) in [(3, 3), (4, 4), (5, 4), (40, 4)] {
        scene.notes = vec![Note::Choice {
            id: "pick".into(),
            title: "pick one".into(),
            options: (0..given).map(|i| format!("option {i}")).collect(),
            footer: String::new(),
            struck_first: false,
            whole: false,
        }];
        let placed = layout(&scene, &theme, viewport, &MEASURE);
        let options: Vec<&str> =
            placed.iter().filter(|p| p.piece == PieceKind::Option).map(|p| p.text[0].text.as_str()).collect();
        assert_eq!(options, ["option 0", "option 1", "option 2", "option 3"][..drawn], "{given} options");
    }
}

#[test]
fn a_theme_built_in_code_goes_through_the_same_check_and_nothing_is_laid_out_for_one_it_refuses() {
    type Change = fn(&mut Theme);
    let wrong: [(&str, Change); 10] = [
        ("metrics.width", |t| t.metrics.width = f32::NAN),
        ("metrics.width", |t| t.metrics.width = 0.0),
        ("metrics.height", |t| t.metrics.height = f32::INFINITY),
        ("metrics.columns", |t| t.metrics.columns = 0),
        ("metrics.columns", |t| t.metrics.columns = 13),
        ("metrics.margin_y", |t| t.metrics.margin_y = -5.0),
        ("metrics.row_gap", |t| t.metrics.row_gap = -1e30),
        ("metrics.max_scale", |t| t.metrics.max_scale = 1e9),
        ("paper.grid_step", |t| t.paper.grid_step = f32::NAN),
        ("labels.max_chars", |t| t.labels.max_chars = 0),
    ];
    for (name, change) in wrong {
        for mut theme in [Theme::sticker(), Theme::plain()] {
            change(&mut theme);
            match theme.check() {
                Err(ThemeError::OutOfRange { key, .. }) => assert_eq!(key, name),
                other => panic!("{name} was not refused: {other:?}"),
            }
            assert_eq!(layout(&Scene::sample(), &theme, DEFAULT, &MEASURE), [], "{name}: nothing is laid out");
        }
    }
    let mut huge = Theme::sticker();
    huge.metrics.columns = usize::MAX;
    assert!(huge.check().is_err() && layout(&Scene::sample(), &huge, DEFAULT, &MEASURE).is_empty());

    for theme in [Theme::sticker(), Theme::plain(), Theme::default(), tall().0] {
        assert_eq!(theme.check(), Ok(()));
        assert!(!layout(&Scene::sample(), &theme, DEFAULT, &MEASURE).is_empty());
    }
    let from_file = Theme::from_toml("[metrics]\ncolumns = 0\n").unwrap_err();
    let mut built = Theme::sticker();
    built.metrics.columns = 0;
    assert_eq!(built.check().unwrap_err(), from_file, "a file and a struct are refused in the same words");
}
