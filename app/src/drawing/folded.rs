//! The day folds (ADR 0030, decision 12; `docs/development/app.md`, "A read day opens folded"). What the reading
//! put on a day's page is first one drawn note at the column's top, a tally: a row for each project the reading
//! has a wish in, and a small pencil mark for each thing the open page draws for it. A tap on the tally opens
//! the day under it, every piece as it was, and a second tap folds it again. A made wish is never folded.
//!
//! One pure pass over a scene that is already in its column's order ([`folded`]). Which pieces are the
//! reading's is read from that scene: the little ones nobody typed for, and the reading's own notes by their
//! titles and ids. The page data the scene was made from gives two facts the scene does not hold, each by the
//! wish's id: the project of a wish and how many of its jobs came true. No word of the model's is in the tally:
//! a project's name is a folder's, which core checked against the day's own folders.
//!
//! The fold never hides what waits for a tap ([`reading`]): a clash nobody chose on stays under the tally, with
//! its `?` in the tally's row, and so does a note of a read wish that has a choice or a field. Whether the day
//! is open is the window's own state, held with which notes are open (`drawing::Open`).

use jellymoon_core::{PageData, Wish};
use sticker_kit::{Character, MAX_TALLY_ROWS, Mark, Note, Picture, Scene, State, TallyRow, speaker};

use crate::scene::{CAME_TRUE, CHANGED, CLASH, READING, SAME, drawn_id};
use crate::working::WISH;

/// The id of the tally's note: a tap on its picture is this id at the place of no option.
pub const FOLD: &str = "fold";

/// The tally's title. It is Jellymoon's, and so is the line that counts the projects past the fifth.
const TITLE: &str = "today so far";

/// What stands for the projects the tally has no row for.
fn more(count: usize) -> String {
    match count {
        0 => String::new(),
        1 => "and 1 more project".to_string(),
        n => format!("and {n} more projects"),
    }
}

/// The little ones of `column` that are the reading's: the ones no typed words of the column are of.
fn read(column: &Scene) -> Vec<&Character> {
    let typed = |who: &Character| {
        column.notes.iter().any(|note| matches!(note, Note::Said { id, .. } if speaker(id) == who.id))
    };
    column.characters.iter().filter(|who| !typed(who)).collect()
}

/// Whether the fold hides `note`: it is the reading's and nothing of it waits for a tap. What came true, a
/// change of mind the reading found settled, a clash with nothing to choose, and the note a tap on a read wish
/// brought, while it has no choice and no field. A clash that is open, one the person chose on, and every
/// note that is not the reading's are never hidden.
fn reading(note: &Note, read: &[&Character]) -> bool {
    match note {
        Note::List { title, .. } => title == CAME_TRUE,
        Note::Choice { id, title, .. } => id.starts_with("clash-") && title == CHANGED,
        Note::Plain { title, .. } => title == SAME,
        Note::Text { id, options, field, .. } => {
            let of = id.strip_prefix(WISH).is_some_and(|wish| read.iter().any(|who| who.id == wish));
            of && options.is_empty() && field.is_none()
        }
        _ => false,
    }
}

/// The wish of the open clash `note`'s first statement, by its place among the page's clashes.
fn asked<'a>(note: &Note, data: &'a PageData) -> Option<&'a str> {
    let Note::Choice { id, title, .. } = note else { return None };
    let place = id.strip_prefix("clash-").filter(|_| title == CLASH)?.parse::<usize>().ok()?;
    Some(data.clashes.get(place.checked_sub(1)?)?.a.as_str())
}

/// The rows of the tally: a project's read wishes give it its marks, in the order the day's projects were first
/// typed in, which is the order of the entry's chips. One mark for one piece of the open page: a `?` for a
/// little one that needs the person and for an open clash of one of the project's wishes, a mark for a little
/// one at work, a star for each row of what came true, a cross for a little one that was let go.
fn rows(column: &Scene, wishes: &[(&Character, &Wish)], data: &PageData) -> Vec<TallyRow> {
    let mut projects: Vec<&String> = Vec::new();
    for name in data.read.projects.iter().chain(wishes.iter().map(|(_, wish)| &wish.project)) {
        if !projects.contains(&name) && wishes.iter().any(|(_, wish)| wish.project == *name) {
            projects.push(name);
        }
    }
    let row = |project: &String| {
        let own: Vec<&(&Character, &Wish)> = wishes.iter().filter(|(_, wish)| wish.project == *project).collect();
        let of = |state: State| own.iter().filter(|(who, _)| who.state == state).count();
        let clashes = column.notes.iter().filter_map(|note| asked(note, data));
        let asks = of(State::NeedsYou) + clashes.filter(|wish| own.iter().any(|(_, one)| one.id == *wish)).count();
        let stars: usize = own.iter().map(|(_, wish)| wish.stars.len()).sum();
        let counts = [(Mark::NeedsYou, asks), (Mark::Busy, of(State::Busy)), (Mark::Star, stars)];
        let counts = counts.into_iter().chain([(Mark::Dimmed, of(State::Dimmed))]);
        TallyRow { label: project.clone(), marks: counts.flat_map(|(mark, count)| vec![mark; count]).collect() }
    };
    projects.into_iter().map(row).collect()
}

/// `column`, a scene in its column's order, as a window whose day is `open` or folded draws it. With no page
/// data, with no wish of the reading on it, or where a little one of the reading is no wish of `data`, the
/// column is given back as it is: nothing is folded that cannot be counted. Else the tally is the scene's last
/// note, which the kit stands at the column's top, so every other piece of an open day is the piece it was;
/// and folded, the reading's little ones and its notes ([`reading`]) are not on the scene.
pub(super) fn folded(mut column: Scene, data: Option<&PageData>, open: bool) -> Scene {
    let Some(data) = data else { return column };
    let read = read(&column);
    let wish = |who: &Character| data.wishes.iter().find(|wish| drawn_id(&wish.id) == who.id);
    let wishes: Vec<(&Character, &Wish)> =
        read.iter().copied().filter_map(|who| wish(who).map(|wish| (who, wish))).collect();
    if read.is_empty() || wishes.len() != read.len() {
        return column;
    }
    let rows = rows(&column, &wishes, data);
    let more = more(rows.len().saturating_sub(MAX_TALLY_ROWS));
    let (title, under) = (TITLE.to_string(), READING.to_string());
    let picture = Picture::Tally { title, rows, more, under, open, in_place: false };
    let tally =
        Note::Drawn { id: FOLD.to_string(), picture, words: Vec::new(), open: false, options: Vec::new(), field: None };
    if !open {
        let gone: Vec<String> = read.iter().map(|who| who.id.clone()).collect();
        let hidden: Vec<bool> = column.notes.iter().map(|note| reading(note, &read)).collect();
        let mut hidden = hidden.into_iter();
        column.notes.retain(|_| !hidden.next().unwrap_or(false));
        column.characters.retain(|who| !gone.contains(&who.id));
    }
    column.notes.push(tally);
    column
}

#[cfg(test)]
mod tests;
