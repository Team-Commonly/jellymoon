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

fn row(label: String, quotes: &[String]) -> ToldRow {
    ToldRow { label, quotes: quotes.to_vec(), with_last: false }
}

/// The rows of a change: the lines as they were and as they are, each under a label that names the file and the
/// side, or of a made file its lines under one label. A side with no line has no row.
fn changed(name: &str, made: bool, before: &[String], after: &[String]) -> Vec<ToldRow> {
    if made {
        return vec![row(format!("New file {name}"), after)];
    }
    let sides = [("before", before), ("after", after)];
    let mut rows: Vec<ToldRow> = sides
        .iter()
        .filter(|(_, lines)| !lines.is_empty())
        .map(|(side, lines)| row(format!("Changed in {name}, {side}"), lines))
        .collect();
    // The two sides are one unit: drawn together or not at all.
    rows.iter_mut().skip(1).for_each(|row| row.with_last = true);
    rows
}

/// The note of what the run of `waits` says it did, when core printed a piece of it that passed. None when there is
/// no result, or when every piece of it was dropped.
pub(crate) fn told(waits: &Waits) -> Option<Note> {
    let told = waits.result.as_ref().filter(|told| !told.pieces.is_empty())?;
    let (mut rows, mut decided, mut unsure) = (Vec::new(), Vec::new(), Vec::new());
    for piece in &told.pieces {
        match piece {
            // What it did has no label: the title is its label, and it is read first.
            Piece::Did { text } => rows.push(row(String::new(), std::slice::from_ref(text))),
            Piece::Change { name, made, before, after, .. } => rows.extend(changed(name, *made, before, after)),
            Piece::Decided { text } => decided.push(text.clone()),
            Piece::Unsure { text } => unsure.push(text.clone()),
        }
    }
    let grouped = [(DECIDED, decided), (UNSURE, unsure)];
    rows.extend(
        grouped.into_iter().filter(|(_, texts)| !texts.is_empty()).map(|(label, texts)| row(label.to_string(), &texts)),
    );
    let mut end = vec![NOTHING_RUN.to_string()];
    end.extend((told.dropped > 0).then(|| UNCHECKED.to_string()));
    Some(Note::Told { id: format!("{TOLD}{}", waits.wish), title: TITLE.to_string(), rows, end })
}
