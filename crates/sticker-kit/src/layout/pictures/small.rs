//! A picture drawn small, on one line: what stands under the bubble of what a person typed when its note has one
//! (`Note::Said`'s `small`). Left to right: a sign the pencil draws, the picture's own captions, word for word
//! as the card of a drawn note has them (`sheets.rs`), and a chevron that says there is more behind it. It is a
//! `Small` piece as high as the kit's smallest tap target, and a tap on it is the bubble's.
//!
//! Finished work has a small form, and no other picture has one yet. In the folder: a folder with its tick, the
//! count of files and "in your folder". Waiting: the folder dashed. No file at all: an empty sheet and "nothing
//! changed". Thrown away: a sheet crossed out and "thrown away", after the count when there is one, all of it in
//! the quiet ink.

use super::super::{Ctx, PieceKind, Placed, line_height, piece, run};
use super::sheets::{TOSSED, count, place};
use crate::geom::Rect;
use crate::pencil::{Ink, Part, Pencil, Point, Sketch};
use crate::scene::{Kept, Picture};
use crate::text::Role;

/// The height of the piece, the kit's smallest tap target, and the room of its sign, in design units.
const HIGH: f32 = 24.0;
const SIGN: f32 = 24.0;

/// `picture` drawn small for the note `id`: its top right corner is `at`. None of a picture with no small form.
pub(in crate::layout) fn mark(cx: &Ctx, id: &str, picture: &Picture, at: Point) -> Option<Placed> {
    let Picture::Sheets { made, changed, removed, state } = picture else { return None };
    let files = made.saturating_add(*changed).saturating_add(*removed);
    let gone = *state == Kept::Tossed;
    let words: Vec<String> = match (gone, files) {
        (true, 0) => vec![TOSSED.to_string()],
        (true, _) => vec![count(files), TOSSED.to_string()],
        (false, 0) => vec![count(0)],
        (false, _) => vec![count(files), place(*state).to_string()],
    };
    let role = if gone { Role::Footer } else { Role::Caption };
    let (size, gap, u) = (cx.size(role), cx.px(8.0), cx.px(1.0));
    let widths: Vec<f32> = words.iter().map(|word| cx.measure.width(word, role, size)).collect();
    let wide = SIGN * u + widths.iter().map(|width| width + gap).sum::<f32>() + 8.0 * u;
    let rect = Rect::new(at.0 - wide, at.1, wide, (HIGH * u).max(line_height(size)));
    let (mut placed, mid) = (piece(PieceKind::Small, id, 0, rect), rect.center().1);
    let mut pencil = Pencil::new(&format!("{id}~"), cx.k);
    pencil.ink(if gone { Ink::Quiet } else { Ink::Pencil });
    let mut parts = if gone || files == 0 {
        sheet(&mut pencil, (rect.x, mid), u, gone)
    } else {
        folder(&mut pencil, (rect.x, mid), u, *state == Kept::In)
    };
    let mut x = rect.x + SIGN * u;
    for (word, width) in words.into_iter().zip(widths) {
        placed.text.push(run(word, role, x, mid, width, size));
        x += width + gap;
    }
    pencil.path(&[(x, mid - 4.0 * u), (x + 4.0 * u, mid), (x, mid + 4.0 * u)]);
    parts.push(Part { name: "opens".to_string(), rect: Rect::new(x, mid - 4.0 * u, 4.0 * u, 8.0 * u) });
    let what = format!("small sheets files={files}{}", if gone { " tossed" } else { "" });
    placed.sketch = Some(Sketch { what, parts, strokes: pencil.finish() });
    Some(placed)
}

/// A small folder from `left`, level with `mid`: solid with a tick in the added ink when the work is `inside`
/// it, dashed when it is not.
fn folder(pencil: &mut Pencil, (left, mid): Point, u: f32, inside: bool) -> Vec<Part> {
    let body = Rect::new(left + 2.0 * u, mid - 5.0 * u, 17.0 * u, 12.0 * u);
    pencil.frame(body, !inside);
    let tab = [(0.0, 0.0), (2.5, -3.0), (8.0, -3.0), (10.0, 0.0)].map(|(x, y)| (body.x + x * u, body.y + y * u));
    tab.windows(2).for_each(|pair| pencil.line(pair[0], pair[1]));
    let whole = Rect::new(body.x, body.y - 3.0 * u, body.w, body.h + 3.0 * u);
    let mut parts = vec![Part { name: if inside { "folder solid" } else { "folder dashed" }.to_string(), rect: whole }];
    if inside {
        let tick = Rect::new(body.x + 5.0 * u, body.y + 3.0 * u, 8.0 * u, 6.5 * u);
        pencil.ink(Ink::Added);
        pencil.tick(tick);
        pencil.ink(Ink::Pencil);
        parts.push(Part { name: "tick".to_string(), rect: tick });
    }
    parts
}

/// A small empty sheet from `left`, level with `mid`, crossed out when the work is `gone`.
fn sheet(pencil: &mut Pencil, (left, mid): Point, u: f32, gone: bool) -> Vec<Part> {
    let sheet = Rect::new(left + 5.0 * u, mid - 8.0 * u, 12.0 * u, 16.0 * u);
    pencil.frame(sheet, false);
    let mut parts = vec![Part { name: "sheet empty".to_string(), rect: sheet }];
    if gone {
        let over = Rect::new(sheet.x + 1.5 * u, sheet.y + 2.5 * u, 9.0 * u, 11.0 * u);
        pencil.cross(over);
        parts.push(Part { name: "cross".to_string(), rect: over });
    }
    parts
}
