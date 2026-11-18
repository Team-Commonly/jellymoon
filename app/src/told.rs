//! What a run says it did, over the note of its work that waits (F86, ADR 0023, decisions 3 to 6). Core checks
//! each piece against the work at every look and prints what passed (`Waits::result`); here it is put in one note
//! of the kit's, `told-<wish>`, placed before `work-<wish>`. Every text of the run is a quote of that note and
//! nothing else: the title, the labels and the last sentences are Jellymoon's fixed words and core's cleaned file
//! names. The note has no option and no field, so no tap finds a word of it, and the work's note is the note it
//! is without a result: every choice and every rule of the yes stay there. No file of the work is opened or run.

use jellymoon_core::{Piece, Waits};
use sticker_kit::{Note, ToldRow};

/// The start of the id of the note, followed by the wish's id.
pub(crate) const TOLD: &str = "told-";

const TITLE: &str = "What it says it did";
const DECIDED: &str = "Decided";
const UNSURE: &str = "Not sure about";
/// Said on every such note: a run cannot run a command, so nothing it says was tried.
const NOTHING_RUN: &str = "Nothing was run to try this. A run reads and edits files, and no more.";
/// Said last when core dropped a piece that failed its check.
const UNCHECKED: &str = "Something the run said could not be checked, so it is not here.";

fn row(label: &str, quotes: &[String]) -> ToldRow {
    ToldRow { label: label.to_string(), quotes: quotes.to_vec(), ..ToldRow::default() }
}

/// The row of a change, one block drawn whole or not at all: Jellymoon's words and the file's name, which the
/// kit puts on their line only when the two are one line and the name holds no blank (else the name is the
/// row's first quote); then the lines as they were and as they are, the first of each side marked "before" or
/// "after". Of a made file, its lines.
fn changed(name: &str, made: bool, before: &[String], after: &[String]) -> ToldRow {
    let named = |label: &str, quotes: &[String]| ToldRow { name: name.to_string(), ..row(label, quotes) };
    if made {
        return named("New file", after);
    }
    let mut block = named("Changed in", &[before, after].concat());
    for (side, lines) in [("before", before), ("after", after)] {
        block.marks.extend((0..lines.len()).map(|at| if at == 0 { side.to_string() } else { String::new() }));
    }
    block
}

/// The note of what the run of `waits` says it did, when core printed a piece of it that passed. None when there is
/// no result, or when every piece of it was dropped.
pub(crate) fn told(waits: &Waits) -> Option<Note> {
    let told = waits.result.as_ref().filter(|told| !told.pieces.is_empty())?;
    let (mut rows, mut decided, mut unsure) = (Vec::new(), Vec::new(), Vec::new());
    for piece in &told.pieces {
        match piece {
            // What it did has no label: the title is its label, and it is read first.
            Piece::Did { text } => rows.push(row("", std::slice::from_ref(text))),
            Piece::Change { name, made, before, after, .. } => rows.push(changed(name, *made, before, after)),
            Piece::Decided { text } => decided.push(text.clone()),
            Piece::Unsure { text } => unsure.push(text.clone()),
        }
    }
    let grouped = [(DECIDED, decided), (UNSURE, unsure)];
    rows.extend(grouped.into_iter().filter(|(_, texts)| !texts.is_empty()).map(|(label, texts)| row(label, &texts)));
    let mut end = vec![NOTHING_RUN.to_string()];
    end.extend((told.dropped > 0).then(|| UNCHECKED.to_string()));
    Some(Note::Told { id: format!("{TOLD}{}", waits.wish), title: TITLE.to_string(), rows, end })
}
