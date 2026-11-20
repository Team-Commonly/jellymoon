//! What a made wish's run is doing while the wish works (live work; `docs/development/app.md`, "A wish at work").
//! Core keeps the run's steps as they come (`jellymoon_core::doing`); at each drawing of the page they are put
//! before the person: a few words over each working made wish's little one, and one told note of the newest one's
//! last steps, just before the note that says it is being worked on. The texts are Jellymoon's fixed words and
//! core's cleaned names of files. In the note a name is drawn as the result's note draws a changed file's, never a
//! title, an option or a field; on the pill it is used only when it holds no blank (`says`). A step is a tool use
//! the run asked for, not proof that it happened. Nothing here can be tapped. Once the run has ended none of it is
//! drawn.

use std::path::Path;

use jellymoon_core::{Step, Steps, Tool};
use sticker_kit::{Note, Scene, State, ToldRow};

/// The start of the id of the note of a wish's steps, followed by the wish's id.
pub(crate) const STEPS: &str = "steps-";
const TITLE: &str = "What it is doing";
/// What a little one says before its run's first step.
pub(crate) const THINKING: &str = "thinking";

/// What a step is doing, in Jellymoon's words, and the name of its file when core kept one.
fn doing(step: &Step) -> (&'static str, Option<&str>) {
    let (named, nameless) = match step.tool {
        Tool::Read => ("reading", "reading a file"),
        Tool::Edit => ("changing", "changing a file"),
        Tool::Write => ("making", "making a file"),
        Tool::Glob => return ("looking through the folder", None),
        Tool::Grep => return ("searching the files", None),
    };
    match step.file.as_deref() {
        Some(name) => (named, Some(name)),
        None => (nameless, None),
    }
}

/// What a working little one says: its run's last step, or [`THINKING`] before the first. The pill is one run of
/// text, so a name and Jellymoon's words are told apart only by the blank between them: a name that holds a blank
/// of its own (`is done. Kept`, `a file`) could read as Jellymoon's, and the step is said without it.
pub(crate) fn says(steps: Option<&Steps>) -> String {
    let Some(step) = steps.and_then(|steps| steps.last.last()) else { return THINKING.to_string() };
    let named = Step { file: step.file.clone().filter(|name| !name.chars().any(char::is_whitespace)), ..step.clone() };
    match doing(&named) {
        (doing, Some(name)) => format!("{doing} {name}"),
        (doing, None) => doing.to_string(),
    }
}

/// The note of a wish's steps: each kept step a row, newest last, as its words with a capital and the file's name
/// beside them, and how many came. None before the first step.
pub(crate) fn note(wish: &str, steps: &Steps) -> Option<Note> {
    let row = |step: &Step| {
        let (doing, name) = doing(step);
        let label = doing[..1].to_uppercase() + &doing[1..];
        ToldRow { label, name: name.unwrap_or_default().to_string(), ..ToldRow::default() }
    };
    let rows: Vec<ToldRow> = steps.last.iter().map(row).collect();
    let count = if steps.count == 1 { "1 step so far.".to_string() } else { format!("{} steps so far.", steps.count) };
    (!rows.is_empty()).then(|| Note::Told {
        id: format!("{STEPS}{wish}"),
        title: TITLE.to_string(),
        rows,
        end: vec![count],
    })
}

/// The page `shown`, whose last `made` little ones are the day's made wishes, with what each working one's run is
/// doing: its words over it, and the newest one's note before the note that says the made wishes are being worked
/// on, when the page has that note. Core is asked only when a made wish is working.
pub(crate) fn drawn(home: &Path, day: &str, made: usize, shown: &mut Scene) {
    let from = shown.characters.len().saturating_sub(made);
    if !shown.characters[from..].iter().any(|one| one.state == State::Busy) {
        return;
    }
    let doing = jellymoon_core::doing(home, day);
    let steps = |id: &str| doing.iter().find(|(wish, _)| crate::scene::drawn_id(wish) == id).map(|(_, steps)| steps);
    let mut newest = None;
    for one in shown.characters[from..].iter_mut().filter(|one| one.state == State::Busy) {
        one.says = says(steps(&one.id));
        newest = Some(one.id.clone());
    }
    let going =
        |note: &Note| matches!(note, Note::Plain { title, .. } if crate::scene::GOING.contains(&title.as_str()));
    let told = newest.and_then(|id| note(&id, steps(&id)?));
    if let (Some(at), Some(told)) = (shown.notes.iter().position(going), told) {
        shown.notes.insert(at, told);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(tool: Tool, file: Option<&str>) -> Step {
        Step { tool, file: file.map(str::to_string) }
    }

    #[test]
    fn each_step_is_said_in_jellymoon_s_words_with_core_s_name_and_no_other_text() {
        let steps = [
            (step(Tool::Read, Some("hours.txt")), "reading hours.txt", "Reading"),
            (step(Tool::Read, None), "reading a file", "Reading a file"),
            (step(Tool::Edit, Some("src/a.rs")), "changing src/a.rs", "Changing"),
            (step(Tool::Edit, None), "changing a file", "Changing a file"),
            (step(Tool::Write, Some("new.txt")), "making new.txt", "Making"),
            (step(Tool::Write, None), "making a file", "Making a file"),
            (step(Tool::Glob, None), "looking through the folder", "Looking through the folder"),
            (step(Tool::Grep, None), "searching the files", "Searching the files"),
        ];
        assert_eq!(says(None), THINKING);
        assert_eq!(says(Some(&Steps::default())), THINKING);
        for (one, said, label) in steps {
            let kept = Steps { count: 12, last: vec![step(Tool::Glob, None), one.clone()] };
            assert_eq!(says(Some(&kept)), said);
            let Some(Note::Told { id, title, rows, end }) = note("m1", &kept) else { panic!("a note") };
            assert_eq!((id.as_str(), title.as_str(), end), ("steps-m1", TITLE, vec!["12 steps so far.".to_string()]));
            let name = one.file.clone().unwrap_or_default();
            assert_eq!(rows[1], ToldRow { label: label.to_string(), name, ..ToldRow::default() });
        }
        // A name with a blank of its own is not put beside Jellymoon's words on the pill; the note still names it.
        for blank in ["is done. Kept", "a file", "two\tparts.txt", "end .txt", "\u{a0}nbsp.txt"] {
            let kept = Steps { count: 1, last: vec![step(Tool::Read, Some(blank))] };
            assert_eq!(says(Some(&kept)), "reading a file", "{blank:?}");
            let Some(Note::Told { rows, .. }) = note("m1", &kept) else { panic!("a note") };
            assert_eq!(rows[0].name, blank);
        }
        let one = Steps { count: 1, last: vec![step(Tool::Read, Some("b.txt"))] };
        assert!(matches!(note("m1", &one), Some(Note::Told { end, .. }) if end == ["1 step so far."]));
        assert_eq!(note("m1", &Steps::default()), None);
    }
}
