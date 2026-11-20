//! What a run says it did, over the note of its work that waits (F86, ADR 0023, decisions 3 to 6). Core checks
//! each piece against the work at every look and prints what passed (`Waits::result`); here it is put in one note
//! of the kit's, `told-<wish>`, placed before `work-<wish>`. Every text of the run is a quote of that note and
//! nothing else: the title, the labels and the last sentences are Jellymoon's fixed words and core's cleaned file
//! names. The note has no option and no field, so no tap finds a word of it, and the work's note is the note it
//! is without a result: every choice and every rule of the yes stay there. No file of the work is opened or run.
//! A run that finished and changed nothing has no work's note: its result's note stands alone over the wish's
//! own, and says first that nothing in the folder was changed (told alone, `jellymoon_core::Alone`).
//!
//! The note first comes onto the page with no row of a change, so with no file's name and no changed line: one
//! sentence says what core's check of them found. The rows come with a tap on `Show me` (`working.rs`, `notes_of`).

use jellymoon_core::{Alone, Piece, Told, Waits};
use sticker_kit::{Note, ToldRow};

/// The start of the id of the note, followed by the wish's id.
pub(crate) const TOLD: &str = "told-";

const TITLE: &str = "What it says it did";
const DECIDED: &str = "Decided";
const UNSURE: &str = "Not sure about";
/// Said on every such note: a run cannot run a command, so nothing it says was tried.
const NOTHING_RUN: &str = "Nothing was run to try this. A run reads and edits files, and no more.";
/// Said in the place of every change's row before `Show me`, when a change passed core's check: each line it
/// quotes is a whole line the work took from its file or put in it. Not said of a change core left out or dropped.
/// It starts the paragraph of [`NOTHING_RUN`] and so adds none of its own: at 960x640 what the run did is still
/// read (`tests/f86.rs`).
const CHECKED: &str = "The changed lines it reported match the copy.";
/// Said last when core dropped a piece that failed its check.
const UNCHECKED: &str = "Something the run said could not be checked, so it is not here.";
/// Said first of a run that finished and changed nothing.
const UNCHANGED: &str = "Nothing in your folder was changed.";

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
/// no result, or when every piece of it was dropped. `shown`: the person tapped `Show me`, and each change has its
/// row; else [`CHECKED`] stands for them all.
pub(crate) fn told(waits: &Waits, shown: bool) -> Option<Note> {
    note(&waits.wish, waits.result.as_ref()?, None, shown)
}

/// The note of what the run of the made wish `wish` says it did, when it finished and changed nothing and core
/// printed a piece of it that passed. None of any other end.
pub(crate) fn told_alone(wish: &str, alone: &Alone) -> Option<Note> {
    let Alone::Unchanged(Some(told)) = alone else { return None };
    note(wish, told, Some(UNCHANGED), false)
}

/// The note of `told` of the wish `wish`, its sentences after `first`, when one is given. With `shown` a change
/// is its row, and without it no row: then the changes are said once, among the last sentences. The rows of the
/// changes come before every other: they are what the tap asked for, and the kit leaves out the rows that do not
/// fit from the end.
fn note(wish: &str, told: &Told, first: Option<&str>, shown: bool) -> Option<Note> {
    if told.pieces.is_empty() {
        return None;
    }
    let (mut changes, mut rows, mut checked) = (Vec::new(), Vec::new(), None);
    let (mut decided, mut unsure) = (Vec::new(), Vec::new());
    for piece in &told.pieces {
        match piece {
            // What it did has no label: the title is its label, and it is read first, or after the changes' rows.
            Piece::Did { text } => rows.push(row("", std::slice::from_ref(text))),
            Piece::Change { .. } if !shown => checked = Some(CHECKED),
            Piece::Change { name, made, before, after, .. } => changes.push(changed(name, *made, before, after)),
            Piece::Decided { text } => decided.push(text.clone()),
            Piece::Unsure { text } => unsure.push(text.clone()),
        }
    }
    let grouped = [(DECIDED, decided), (UNSURE, unsure)];
    let mut rows: Vec<ToldRow> = changes.into_iter().chain(rows).collect();
    rows.extend(grouped.into_iter().filter(|(_, texts)| !texts.is_empty()).map(|(label, texts)| row(label, &texts)));
    let run = checked.map_or(NOTHING_RUN.to_string(), |checked| format!("{checked} {NOTHING_RUN}"));
    let mut end: Vec<String> = first.map(str::to_string).into_iter().chain([run]).collect();
    end.extend((told.dropped > 0).then(|| UNCHECKED.to_string()));
    Some(Note::Told { id: format!("{TOLD}{wish}"), title: TITLE.to_string(), rows, end })
}
