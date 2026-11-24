//! The placed pieces as lines a person, or a test, reads: no window and no picture file.

use std::fmt::Write as _;

use super::Placed;

/// One line a piece, in the order they were placed: its kind, id and place; for a piece a pencil draws, what the
/// picture is and counts and the parts it is drawn in; then every text the piece draws, in quotes.
pub fn print(placed: &[Placed]) -> String {
    let mut out = String::new();
    for piece in placed {
        let _ = write!(out, "{} {} {}", format!("{:?}", piece.piece).to_lowercase(), piece.id, piece.index);
        if let Some(sketch) = &piece.sketch {
            let mut names: Vec<&str> = Vec::new();
            for part in &sketch.parts {
                if !names.contains(&part.name.as_str()) {
                    names.push(&part.name);
                }
            }
            let counted = |name: &&str| format!("{name}×{}", sketch.count(name));
            let _ = write!(out, " [{}] ({})", sketch.what, names.iter().map(counted).collect::<Vec<_>>().join(", "));
        }
        for run in &piece.text {
            let _ = write!(out, " \"{}\"", run.text);
        }
        out.push('\n');
    }
    out
}
