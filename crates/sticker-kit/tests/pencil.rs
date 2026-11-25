//! The pencil and what it draws: strokes that follow from a note's id and nothing else, that draw themselves in
//! their turn, and the three pictures of a drawn note, read from the placed pieces and their print. No window.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use sticker_kit::pencil::{DRAWN_IN, REACH};
use sticker_kit::{
    Bar, BundledMeasure, Field, Kept, Note, Pencil, Picture, PieceKind, Placed, Rect, Scene, Sketch, Stage, Stroke,
    Theme, Viewport, layout, print,
};

const SIZE: Viewport = Viewport { w: 1440.0, h: 900.0 };

fn drawn(id: &str, picture: Picture) -> Note {
    Note::Drawn { id: id.into(), picture, words: Vec::new(), open: false, options: Vec::new(), field: None }
}

fn place(notes: Vec<Note>) -> Vec<Placed> {
    let theme = Theme::sticker();
    layout(&Scene { notes, ..Scene::default() }, &theme, SIZE, &BundledMeasure::new(&theme))
}

fn piece(placed: &[Placed], kind: PieceKind, id: &str) -> Option<Placed> {
    placed.iter().find(|piece| piece.piece == kind && piece.id == id).cloned()
}

/// The picture of one drawn note, laid out alone: the piece, its sketch and the captions it draws.
fn picture(picture: Picture) -> (Placed, Sketch, Vec<String>) {
    let piece = piece(&place(vec![drawn("p", picture)]), PieceKind::Picture, "p").expect("a picture is placed");
    let (sketch, captions) = (piece.sketch.clone().expect("it has strokes"), texts(&piece));
    (piece, sketch, captions)
}

fn texts(piece: &Placed) -> Vec<String> {
    piece.text.iter().map(|run| run.text.clone()).collect()
}

fn track(steps: usize, stage: Stage, file: &str) -> Picture {
    Picture::Track { steps, stage, file: file.into() }
}

fn sheets(made: usize, changed: usize, removed: usize, state: Kept) -> Picture {
    Picture::Sheets { made, changed, removed, state }
}

fn bar(name: &str, added: usize, taken: usize) -> Bar {
    Bar { name: name.into(), added, taken }
}

fn every_picture() -> Vec<Picture> {
    let rows = vec![bar("src/hello.py", 30, 10), bar("README.md", 10, 0), bar("old.txt", 0, 0)];
    vec![
        track(0, Stage::Thinking, ""),
        track(7, Stage::Looking, "hello.py"),
        track(26, Stage::Making, "hello.py"),
        sheets(1, 0, 0, Kept::Waiting),
        sheets(3, 2, 2, Kept::In),
        sheets(1, 1, 1, Kept::Tossed),
        sheets(0, 0, 0, Kept::Waiting),
        Picture::Bars { rows, more: 2 },
    ]
}

fn parts(sketch: &Sketch, names: &[&str]) -> Vec<usize> {
    names.iter().map(|name| sketch.count(name)).collect()
}

#[test]
fn the_same_id_and_place_give_the_same_strokes_and_another_id_or_place_gives_others() {
    let strokes = |id: &str| {
        let mut pencil = Pencil::new(id, 1.0);
        pencil.line((10.0, 10.0), (90.0, 10.0));
        pencil.line((10.0, 10.0), (90.0, 10.0));
        pencil.circle((50.0, 50.0), 20.0);
        pencil.finish()
    };
    assert_eq!(strokes("work-1"), strokes("work-1"));
    let (one, other) = (strokes("work-1"), strokes("work-2"));
    assert!(one.iter().zip(&other).all(|(a, b)| a.line != b.line), "another note is another hand");
    // The same line asked for twice in one picture is two strokes, not one stroke twice.
    assert_ne!(one[0].line, one[1].line);
    // The wide pass under a stroke is not the stroke again.
    assert!(one.iter().all(|stroke| stroke.soft != stroke.line && stroke.soft.len() == stroke.line.len()));
    // A straight stroke is no ruled line: it ends past both its ends, and leaves the line between them.
    let line = &one[0].line;
    assert!(line[0].0 < 10.0 && line[line.len() - 1].0 > 90.0, "{line:?}");
    assert!(line.iter().any(|point| (point.1 - 10.0).abs() > 0.01));
    // A whole scene laid out twice is drawn alike.
    let scene = || place(every_picture().into_iter().enumerate().map(|(n, p)| drawn(&format!("p{n}"), p)).collect());
    assert_eq!(scene(), scene());
}

#[test]
fn no_stroke_leaves_its_picture_by_more_than_the_pencil_s_reach() {
    for (n, what) in every_picture().into_iter().enumerate() {
        let (piece, sketch, _) = picture(what);
        let room = piece.rect.grown(REACH);
        assert!(!sketch.strokes.is_empty(), "picture {n} draws nothing");
        for stroke in &sketch.strokes {
            for point in stroke.line.iter().chain(&stroke.soft) {
                let inside = Rect::new(point.0, point.1, 0.0, 0.0).inside(&room);
                assert!(inside, "picture {n} ({}): {point:?} is out of {room:?}", sketch.what);
            }
        }
        for part in &sketch.parts {
            assert!(part.rect.inside(&room), "picture {n}: {part:?} is out of {room:?}");
        }
        for run in &piece.text {
            assert!(run.rect.inside(&room), "picture {n}: {:?} is out of {room:?}", run.text);
        }
    }
}

fn long(points: &[(f32, f32)]) -> f32 {
    points.windows(2).map(|pair| (pair[1].0 - pair[0].0).hypot(pair[1].1 - pair[0].1)).sum()
}

#[test]
fn a_picture_draws_itself_stroke_after_stroke_and_is_whole_after_its_time_or_with_no_time() {
    for what in every_picture() {
        let (_, sketch, _) = picture(what);
        let seconds = sketch.seconds();
        assert!(seconds > 0.0 && seconds <= DRAWN_IN + 0.001, "{}: {seconds}", sketch.what);
        let drawn = |t: Option<f32>| sketch.strokes.iter().map(|stroke| stroke.part(t)).collect::<Vec<f32>>();
        assert!(drawn(Some(0.0)).iter().all(|part| *part == 0.0), "{}: ink at no time", sketch.what);
        assert!(sketch.strokes.iter().all(|stroke| stroke.passes(Some(0.0)).is_none()));
        // The strokes follow one another: each begins where the one before it ended.
        let turns = sketch.strokes.windows(2).all(|pair| (pair[0].start + pair[0].length - pair[1].start).abs() < 1e-3);
        assert!(turns, "{}", sketch.what);
        let mid = sketch.strokes[sketch.strokes.len() / 2].clone();
        let half = drawn(Some(mid.start + mid.length / 2.0));
        let partly: Vec<&f32> = half.iter().filter(|part| **part > 0.0 && **part < 1.0).collect();
        assert_eq!(partly.len(), 1, "{}: one stroke is under the pencil", sketch.what);
        assert_eq!(
            half.iter().filter(|part| **part == 0.0).count(),
            sketch.strokes.len() - sketch.strokes.len() / 2 - 1
        );
        let [_, line] = mid.passes(Some(mid.start + mid.length / 2.0)).unwrap();
        let share = long(&line) / long(&mid.line);
        assert!((share - 0.5).abs() < 0.02 && line[0] == mid.line[0], "half a stroke is half its length: {share}");
        for whole in [Some(seconds + 0.01), Some(60.0), None] {
            assert!(drawn(whole).iter().all(|part| *part == 1.0), "{} at {whole:?}", sketch.what);
        }
        let whole = |stroke: &Stroke| stroke.passes(None).unwrap() == [stroke.soft.clone(), stroke.line.clone()];
        assert!(sketch.strokes.iter().all(whole));
    }
}

#[test]
fn a_track_counts_its_steps_in_tallies_of_five_and_ends_them_with_a_plus_past_twenty_five() {
    let names = ["tally", "across", "more"];
    for (steps, counts) in [(0, [0, 0, 0]), (4, [4, 0, 0]), (5, [4, 1, 0]), (25, [20, 5, 0]), (26, [20, 5, 1])] {
        let (_, sketch, _) = picture(track(steps, Stage::Making, ""));
        assert_eq!(parts(&sketch, &names), counts, "{steps} steps");
        assert_eq!(sketch.what, format!("track steps={steps} stage=making file="));
        assert_eq!(parts(&sketch, &["dashed", "place"]), [1, 1], "the place of the result is there, and empty");
    }
}

#[test]
fn a_track_says_its_stage_in_two_words_and_draws_a_file_s_name_only_by_the_name_rule() {
    let tools = ["dot", "sheet", "magnifier", "pencil"];
    let stages = [
        (Stage::Thinking, "thinking", [3, 0, 0, 0]),
        (Stage::Looking, "looking around", [0, 1, 1, 0]),
        (Stage::Making, "making the change", [0, 1, 0, 1]),
    ];
    for (stage, caption, counts) in stages {
        let (_, sketch, captions) = picture(track(3, stage, "hello.py"));
        assert_eq!(captions, [caption, "hello.py"]);
        assert_eq!(parts(&sketch, &tools), counts, "{caption}");
        let moving = sketch.strokes.iter().filter(|stroke| stroke.rocks).count();
        assert_eq!(moving > 0, stage != Stage::Thinking, "only the tool at work rocks");
    }
    let long = "a/".repeat(400);
    for name in ["", "two words.py", "hid\u{200B}den.py", "bell\u{7}.py", long.as_str()] {
        let (piece, sketch, captions) = picture(track(3, Stage::Making, name));
        assert_eq!(captions, ["making the change"], "{name:?} is no name to draw");
        assert_eq!(sketch.what, "track steps=3 stage=making file=");
        assert!(print(&[piece]).contains("[track steps=3 stage=making file=] (tally×3, sheet×1, pencil×1,"));
    }
    let (piece, ..) = picture(track(7, Stage::Making, "hello.py"));
    let line = print(&[piece]);
    assert!(line.starts_with("picture p 0 [track steps=7 stage=making file=hello.py] (tally×6, across×1,"), "{line}");
    assert!(line.ends_with("\"making the change\" \"hello.py\"\n"), "{line}");
}

#[test]
fn sheets_draw_six_files_and_count_the_rest_and_each_state_has_its_own_arrow_folder_and_caption() {
    let kinds = ["sheet made", "sheet changed", "sheet removed", "sheet empty", "stack", "cross"];
    let (_, seven, captions) = picture(sheets(3, 2, 2, Kept::Waiting));
    assert_eq!(parts(&seven, &kinds), [3, 2, 1, 0, 1, 1], "six sheets, the seventh in the stack");
    assert_eq!(captions, ["+1", "7 files", "not in your folder yet"]);
    let (_, six, captions) = picture(sheets(3, 2, 1, Kept::Waiting));
    assert_eq!((parts(&six, &kinds), captions[0].as_str()), (vec![3, 2, 1, 0, 0, 1], "6 files"));

    let ways = ["arrow dashed", "arrow solid", "folder dashed", "folder solid", "tick"];
    let states = [
        (Kept::Waiting, [1, 0, 1, 0, 0], "not in your folder yet"),
        (Kept::In, [0, 1, 0, 1, 1], "in your folder"),
        (Kept::Tossed, [0, 0, 0, 0, 0], "thrown away"),
        (Kept::Unfit, [1, 0, 1, 0, 0], "does not fit your folder"),
    ];
    for (state, counts, caption) in states {
        let (_, sketch, captions) = picture(sheets(1, 1, 0, state));
        assert_eq!(parts(&sketch, &ways), counts, "{state:?}");
        assert_eq!(captions, ["2 files", caption]);
        // Work thrown away is crossed out, sheet by sheet; kept or waiting, only a removed file's sheet is.
        assert_eq!(sketch.count("cross"), if state == Kept::Tossed { 2 } else { 0 }, "{state:?}");
        assert_eq!(sketch.count("barred"), usize::from(state == Kept::Unfit), "{state:?}");
    }
    let (_, removed, captions) = picture(sheets(0, 0, 1, Kept::In));
    assert_eq!((parts(&removed, &kinds), captions[0].as_str()), (vec![0, 0, 1, 0, 0, 1], "1 file"));
    for state in [Kept::Waiting, Kept::In] {
        let (_, none, captions) = picture(sheets(0, 0, 0, state));
        assert_eq!(parts(&none, &kinds), [0, 0, 0, 1, 0, 0]);
        assert_eq!(captions, ["nothing changed"]);
    }
}

#[test]
fn bars_are_drawn_to_one_scale_with_a_ruler_and_a_row_of_nothing_has_no_box() {
    let rows = vec![bar("src/hello.py", 30, 10), bar("README.md", 10, 0), bar("two words", 0, 0)];
    let (_, sketch, captions) = picture(Picture::Bars { rows, more: 0 });
    let boxes = |name: &str| -> Vec<Rect> {
        sketch.parts.iter().filter(|part| part.name == name).map(|part| part.rect).collect()
    };
    let (added, taken, ruler) = (boxes("bar added"), boxes("bar taken"), boxes("ruler")[0]);
    assert_eq!((added.len(), taken.len()), (2, 1), "a row of 0 and 0 draws no box");
    let near = |a: f32, b: f32| (a - b).abs() < 0.01;
    assert!(ruler.w > 200.0 && near(added[0].w + taken[0].w, ruler.w), "the longest row is the ruler's length");
    assert!(near(added[0].w, ruler.w * 0.75) && near(taken[0].w, ruler.w * 0.25) && near(added[1].w, ruler.w * 0.25));
    assert!(near(added[0].x, ruler.x) && taken[0].x > added[0].right() && added[1].y > added[0].bottom());
    assert_eq!(sketch.count("tick"), 3);
    let expected = ["src/hello.py", "+30 −10", "README.md", "+10 −0", "a file", "+0 −0", "0", "20", "40 lines"];
    assert_eq!(captions, expected);
    assert_eq!(sketch.what, "bars rows=3 more=0 most=40");

    let many = (0..8).map(|n| bar(&format!("f{n}.rs"), 1, 0)).collect();
    let (_, sketch, captions) = picture(Picture::Bars { rows: many, more: 2 });
    assert_eq!(sketch.count("bar added"), 6);
    assert!(captions.contains(&"+4 more".to_string()) && !captions.contains(&"f6.rs".to_string()), "{captions:?}");
    assert!(captions.ends_with(&["0".to_string(), "0.5".to_string(), "1 line".to_string()]), "{captions:?}");
    let (_, none, captions) = picture(Picture::Bars { rows: vec![bar("a.rs", 0, 0)], more: 0 });
    assert_eq!((none.count("ruler"), captions), (0, vec!["a.rs".to_string(), "+0 −0".to_string()]));
}

fn worded(open: bool, words: &[&str]) -> Vec<Placed> {
    let words = words.iter().map(|word| word.to_string()).collect();
    let options = vec!["Keep it".to_string(), "Toss it".to_string()];
    let picture = sheets(1, 0, 0, Kept::Waiting);
    place(vec![Note::Drawn { id: "work".into(), picture, words, open, options, field: None }])
}

#[test]
fn a_drawn_note_s_words_are_drawn_only_when_it_is_open_and_then_to_their_last_word() {
    let long = (0..90).map(|n| format!("word{n}")).collect::<Vec<_>>().join(" ");
    let words = ["Your folder is not changed yet.", long.as_str()];
    let closed = worded(false, &words);
    let all = |placed: &[Placed]| placed.iter().flat_map(texts).collect::<Vec<_>>().join(" ");
    assert!(!all(&closed).contains("folder is not") && !all(&closed).contains("word0"), "{}", all(&closed));
    assert!(!print(&closed).contains("word0"));
    let ask = piece(&closed, PieceKind::Ask, "work").expect("words that are not drawn are asked for with a mark");
    let shown = piece(&closed, PieceKind::Picture, "work").unwrap();
    assert_eq!(texts(&ask), ["?"]);
    assert!(ask.rect.inside(&shown.rect) && ask.rect.right() == shown.rect.right() && ask.rect.y == shown.rect.y);
    assert!(ask.sketch.is_some_and(|ring| !ring.strokes.is_empty()), "the mark is the pencil's too");

    let open = worded(true, &words);
    let card = piece(&open, PieceKind::Note, "work").unwrap();
    let lines = texts(&card);
    assert_eq!(lines[0], words[0], "each of the words begins a line of its own");
    assert_eq!(lines[1..].join(" "), long, "and is drawn to its last word");
    assert!(card.text.iter().all(|run| run.rect.y >= shown.rect.bottom()), "the words are under the picture");
    assert!(print(&open).contains("word89") && piece(&open, PieceKind::Ask, "work").is_some());
    let taller = card.rect.h > piece(&closed, PieceKind::Note, "work").unwrap().rect.h;
    assert!(taller, "an open note is as high as its picture and its words");

    // No words, no mark: open or closed.
    for open in [false, true] {
        assert!(piece(&worded(open, &[]), PieceKind::Ask, "work").is_none());
        assert!(piece(&worded(open, &[" ", "\u{200B}"]), PieceKind::Ask, "work").is_none());
    }
    // The options are a text note's: the note's id and the option's place, under the picture and the words.
    for placed in [closed, open] {
        let options: Vec<&Placed> = placed.iter().filter(|piece| piece.piece == PieceKind::Option).collect();
        let named: Vec<(&str, usize, String)> =
            options.iter().map(|option| (option.id.as_str(), option.index, texts(option).join(" "))).collect();
        assert_eq!(named, [("work", 0, "Keep it".to_string()), ("work", 1, "Toss it".to_string())]);
        let under = placed.iter().filter(|piece| piece.piece != PieceKind::Option && piece.piece != PieceKind::Tape);
        let lowest = under.flat_map(|piece| piece.text.iter().map(|run| run.rect.bottom())).fold(0.0, f32::max);
        assert!(options.iter().all(|option| option.rect.y >= lowest && option.rect.y >= shown.rect.bottom()));
    }
}

#[test]
fn a_drawn_note_is_a_card_of_the_column_under_the_piece_before_it_and_ten_of_them_are_all_placed() {
    let text = Note::Text {
        id: "before".into(),
        title: "A card of words".into(),
        body: vec!["What came of it.".into()],
        options: Vec::new(),
        quoted: String::new(),
        field: None,
    };
    let pictures = every_picture().into_iter().chain([track(12, Stage::Looking, "a.rs"), sheets(9, 9, 9, Kept::In)]);
    let notes: Vec<Note> = pictures.enumerate().map(|(n, picture)| drawn(&format!("d{n}"), picture)).collect();
    assert_eq!(notes.len(), 10);
    let placed = place([vec![text], notes].concat());
    let cards: Vec<&Placed> = placed.iter().filter(|piece| piece.piece == PieceKind::Note).collect();
    let pictures: Vec<&Placed> = placed.iter().filter(|piece| piece.piece == PieceKind::Picture).collect();
    assert_eq!((cards.len(), pictures.len()), (11, 10), "every drawn note is placed, with its picture");
    for (n, pair) in cards.windows(2).enumerate() {
        let (above, card) = (pair[0], pair[1]);
        assert_eq!((card.rect.x, card.rect.w), (cards[0].rect.x, cards[0].rect.w), "card {n} has the column's width");
        assert!(card.rect.y > above.rect.bottom(), "card {n} lies under the piece before it");
        assert_eq!(card.tilt_degrees, 0.0, "a drawn card is level, as its strokes are");
        let picture = pictures[n];
        assert!(picture.rect.inside(&card.rect) && picture.rect.h > 40.0, "{:?} in {:?}", picture.rect, card.rect);
    }
    for (n, a) in pictures.iter().enumerate() {
        assert!(pictures[n + 1..].iter().all(|b| !a.rect.overlaps(&b.rect)), "picture {n} lies over another");
    }
}

// A drawn note's field is a text note's: its offer and then the field itself, under the options, open or closed.
#[test]
fn a_drawn_note_s_field_is_placed_under_its_options_as_a_text_note_s_is() {
    let (offer, hint) = ("More like this".into(), "Type here".into());
    // A drawn note's field is never first: it stands under the options whatever it says.
    let field = Field { offer, hint, most: 40, key: "more-m1".into(), wants_keys: false, first: true };
    let options = vec!["Keep it".to_string(), "Toss it".to_string()];
    for open in [false, true] {
        let (picture, words) = (sheets(1, 0, 0, Kept::Waiting), vec!["Your folder is not changed yet.".to_string()]);
        let (id, field) = ("work".to_string(), Some(field.clone()));
        let placed = place(vec![Note::Drawn { id, picture, words, open, options: options.clone(), field }]);
        let (offer, typed) = (piece(&placed, PieceKind::Offer, "work"), piece(&placed, PieceKind::Field, "work"));
        let (offer, typed) = (offer.expect("the offer is placed"), typed.expect("the field is placed"));
        assert_eq!((texts(&offer), texts(&typed)), (vec!["More like this".to_string()], vec!["Type here".to_string()]));
        let card = piece(&placed, PieceKind::Note, "work").unwrap();
        let lowest = placed.iter().filter(|piece| piece.piece == PieceKind::Option).map(|piece| piece.rect.bottom());
        let lowest = lowest.fold(0.0, f32::max);
        assert!(offer.rect.y >= lowest && typed.rect.y >= offer.rect.bottom(), "open {open}: under the options");
        assert!(typed.rect.inside(&card.rect) && typed.rect.x == offer.rect.x, "open {open}: {:?}", typed.rect);
    }
    assert!(piece(&place(vec![drawn("work", sheets(1, 0, 0, Kept::Waiting))]), PieceKind::Field, "work").is_none());
}
