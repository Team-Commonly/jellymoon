//! What a made wish's run is doing while the wish works (live work; `docs/development/app.md`, "A wish at work").
//! Core keeps the run's steps as they come (`jellymoon_core::doing`); at each drawing of the page they are put
//! before the person: a few words over each working made wish's little one, and one told note of how far the newest
//! one has come, just before the note that says it is being worked on. Every text is Jellymoon's fixed words: the
//! stage of the newest step's tool and the count, never a file's name, which core keeps and the page does not draw
//! (a person wants to know how far along it is, not which file is open). A step is a tool use the run asked for,
//! not proof that it happened. Nothing here can be tapped. Once the run has ended none of it is drawn.

use std::path::Path;

use jellymoon_core::{Step, Steps, Tool};
use sticker_kit::{Note, Scene, State};

/// The start of the id of the note of a wish's steps, followed by the wish's id.
pub(crate) const STEPS: &str = "steps-";
const TITLE: &str = "What it is doing";
/// What a little one says before its run's first step.
pub(crate) const THINKING: &str = "thinking";

/// The stage a step's tool says the run is at: the pill's words, and the note's sentence.
fn stage(step: &Step) -> (&'static str, &'static str) {
    match step.tool {
        Tool::Read | Tool::Glob | Tool::Grep => ("looking around", "Looking around your project."),
        Tool::Edit | Tool::Write => ("making the change", "Making the change."),
    }
}

/// What a working little one says: the stage of its run's last step, or [`THINKING`] before the first.
pub(crate) fn says(steps: Option<&Steps>) -> String {
    steps.and_then(|steps| steps.last.last()).map_or(THINKING, |step| stage(step).0).to_string()
}

/// The note of a wish's steps: the stage of the newest as a sentence, and how many came. No row. None before the
/// first step.
pub(crate) fn note(wish: &str, steps: &Steps) -> Option<Note> {
    let newest = steps.last.last()?;
    let count = if steps.count == 1 { "1 step so far.".to_string() } else { format!("{} steps so far.", steps.count) };
    Some(Note::Told {
        id: format!("{STEPS}{wish}"),
        title: TITLE.to_string(),
        rows: Vec::new(),
        end: vec![stage(newest).1.to_string(), count],
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
    fn each_step_is_said_as_its_stage_in_fixed_words_and_never_with_a_name() {
        let around = ("looking around", "Looking around your project.");
        let change = ("making the change", "Making the change.");
        let steps = [
            (step(Tool::Read, Some("hours.txt")), around),
            (step(Tool::Read, Some("is done. Kept")), around),
            (step(Tool::Read, None), around),
            (step(Tool::Glob, None), around),
            (step(Tool::Grep, None), around),
            (step(Tool::Edit, Some("src/a.rs")), change),
            (step(Tool::Edit, None), change),
            (step(Tool::Write, Some("new file.txt")), change),
            (step(Tool::Write, None), change),
        ];
        assert_eq!(says(None), THINKING);
        assert_eq!(says(Some(&Steps::default())), THINKING);
        for (one, (said, stage)) in steps {
            let kept = Steps { count: 12, last: vec![step(Tool::Edit, Some("x.rs")), one.clone()] };
            assert_eq!(says(Some(&kept)), said, "{one:?}");
            let told = Note::Told {
                id: "steps-m1".to_string(),
                title: TITLE.to_string(),
                rows: Vec::new(),
                end: vec![stage.to_string(), "12 steps so far.".to_string()],
            };
            assert_eq!(note("m1", &kept), Some(told), "{one:?}");
        }
        let one = Steps { count: 1, last: vec![step(Tool::Read, Some("b.txt"))] };
        assert!(matches!(note("m1", &one), Some(Note::Told { end, .. }) if end[1] == "1 step so far."));
        assert_eq!(note("m1", &Steps::default()), None);
    }
}
