//! A told note never changes the note it is over (F86's gate): over a sweep of scenes, the work's note with a
//! result on the page has exactly the options, the field and the lines it has with no result. Layout only.

use sticker_kit::{Field, FixedAdvance, Mark, Note, PieceKind, Placed, Scene, Theme, ToldRow, Viewport, layout};

const WORK: &str = "work-1";

/// A work's note of `lines` body lines, its three choices (the last long enough to wrap when `long`), and the
/// follow-up field the app puts under every work that waits.
fn work(lines: usize, field: bool, long: bool) -> Note {
    let toss = if long {
        "Toss it, and every file it made with it, and the branch, and say nothing more of it"
    } else {
        "Toss it"
    };
    let more = Field { offer: "More like this".into(), hint: "What else".into(), most: 200, key: "more-1".into() };
    Note::Text {
        id: WORK.into(),
        title: "Work waits for this wish.".into(),
        body: (0..lines).map(|n| format!("line {n}: a.txt, n.txt and b.txt")).collect(),
        options: ["Show me", "Keep it", toss].map(String::from).to_vec(),
        quoted: String::new(),
        field: field.then_some(more),
    }
}

fn row(label: &str, quotes: &[&str]) -> ToldRow {
    ToldRow {
        label: label.into(),
        quotes: quotes.iter().map(|quote| quote.to_string()).collect(),
        ..ToldRow::default()
    }
}

/// A typical result (a did of two lines, one change, a decided), or a long one.
fn told(long: bool) -> Note {
    let mut change = row("Changed in", &["if total >= 100 { 0 } else { 8 }", "if total >= 50 { 0 } else { 8 }"]);
    (change.name, change.marks) = ("src/shipping.rs".into(), vec!["before".into(), "after".into()]);
    let mut rows = vec![row("", &["Shipping is now free from 50, not 100. The test tries 49 and 50."]), change];
    rows.push(row("Decided", &["An order of exactly 50 ships free."]));
    if long {
        let text = "a long sentence the run said about what it did and why, ".repeat(4);
        rows.extend((0..10).map(|_| row("Not sure about", &[text.as_str()])));
    }
    let end = vec!["Nothing was run to try this. A run reads and edits files, and no more.".to_string()];
    Note::Told { id: "told-1".into(), title: "What it says it did".into(), rows, end }
}

/// The work's note's pieces, each measured from the note's top, to a hundredth; a sheet's and a tape's index is
/// the note's place in the scene, which a told note before it moves on.
fn pieces(placed: &[Placed]) -> Vec<String> {
    let top = placed.iter().find(|p| p.id == WORK && p.piece == PieceKind::Note).map_or(0.0, |p| p.rect.y);
    let round = |v: f32| (v * 100.0).round() / 100.0;
    let at = |y: f32| round(y - top);
    let of = |p: &Placed| {
        let index = if matches!(p.piece, PieceKind::Note | PieceKind::Tape) { None } else { Some(p.index) };
        let runs: Vec<_> =
            p.text.iter().map(|r| (r.text.clone(), r.role, round(r.rect.x), at(r.rect.y), round(r.rect.w))).collect();
        let (x, y, w, h) = (round(p.rect.x), at(p.rect.y), round(p.rect.w), round(p.rect.h));
        format!("{:?} {index:?} {x} {y} {w} {h} {} {runs:?}", p.piece, p.tilt_degrees)
    };
    placed.iter().filter(|p| p.id == WORK).map(of).collect()
}

fn sizes() -> Vec<Viewport> {
    let mut sizes = vec![Viewport { w: 960.0, h: 640.0 }, Viewport { w: 1440.0, h: 900.0 }];
    sizes.extend((705..=848).step_by(13).chain([800]).map(|h| Viewport { w: 960.0, h: h as f32 }));
    sizes
}

#[test]
fn a_result_never_changes_what_the_work_s_note_shows() {
    let (mut scenes, mut kept) = (0, 0);
    for theme in [Theme::sticker(), Theme::plain()] {
        for viewport in sizes() {
            for (lines, field, whole, long) in (1..=24).flat_map(|n| {
                [false, true].into_iter().flat_map(move |f| {
                    [false, true].into_iter().flat_map(move |w| [false, true].map(move |l| (n, f, w, l)))
                })
            }) {
                let place = |told: Option<Note>| {
                    let list =
                        Note::Whole { title: "wished here today".into(), rows: vec![(Mark::Star, "a wish".into())] };
                    let notes = told.into_iter().chain([work(lines, field, long)]).chain(whole.then_some(list));
                    layout(&Scene { notes: notes.collect(), ..Scene::sample() }, &theme, viewport, &FixedAdvance(0.56))
                };
                let alone = pieces(&place(None));
                for result in [false, true] {
                    let placed = place(Some(told(result)));
                    let what =
                        format!("{viewport:?} lines {lines} field {field} whole {whole} long {long} result {result}");
                    assert_eq!(pieces(&placed), alone, "{what}");
                    scenes += 1;
                    kept += usize::from(placed.iter().any(|p| p.id == "told-1"));
                }
            }
        }
    }
    // The sweep is not empty of told notes.
    println!("{kept} of {scenes} scenes drew the told note");
    assert!(kept > 0, "{kept} of {scenes}");
}
