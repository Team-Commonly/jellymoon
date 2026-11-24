//! What a made wish's run is doing while the wish works (live work; `docs/development/app.md`, "A wish at work").
//! Core keeps the run's steps as they come (`jellymoon_core::doing`); at each drawing of the page they are put
//! before the person: a few words over each working made wish's little one, and one drawn note of how far the
//! newest one has come, a track (ADR 0029), in the place of the note that says it is being worked on. That note's
//! sentences are the track's words, with the stage and the count: on the page when the person taps the picture
//! (`drawing.rs`). Every text is Jellymoon's fixed words: the stage of the newest step's tool and the count, never
//! a file's name, which core keeps and the page does not draw (a person wants to know how far along it is, not
//! which file is open). A step is a tool use the run asked for, not proof that it happened. Once the run has
//! ended none of it is drawn.

use std::path::Path;

use jellymoon_core::{Step, Steps, Tool};
use sticker_kit::{Note, Picture, Scene, Stage, State};

/// The start of the id of the note of a wish's steps, followed by the wish's id.
pub(crate) const STEPS: &str = "steps-";
/// What a little one says before its run's first step.
pub(crate) const THINKING: &str = "thinking";

/// The stage a step's tool says the run is at: the pill's words, the note's sentence, and the track's stage.
fn stage(step: &Step) -> (&'static str, &'static str, Stage) {
    match step.tool {
        Tool::Read | Tool::Glob | Tool::Grep => ("looking around", "Looking around your project.", Stage::Looking),
        Tool::Edit | Tool::Write => ("making the change", "Making the change.", Stage::Making),
    }
}

/// What a working little one says: the stage of its run's last step, or [`THINKING`] before the first.
pub(crate) fn says(steps: Option<&Steps>) -> String {
    steps.and_then(|steps| steps.last.last()).map_or(THINKING, |step| stage(step).0).to_string()
}

/// The note of a working wish: a track of core's count of its steps at the stage of the newest, thinking before
/// the first. No file is drawn. Its words are `going`, the title and the body of the note whose place it takes,
/// then the stage as a sentence and how many steps came, each only when there is one.
pub(crate) fn note(wish: &str, steps: Option<&Steps>, going: (&str, &str)) -> Note {
    let newest = steps.and_then(|steps| steps.last.last()).map(stage);
    let count = steps.map_or(0, |steps| steps.count);
    let came = match count {
        0 => String::new(),
        1 => "1 step so far.".to_string(),
        n => format!("{n} steps so far."),
    };
    let words = [going.0, going.1, newest.map_or("", |(_, sentence, _)| sentence), came.as_str()];
    let stage = newest.map_or(Stage::Thinking, |(.., stage)| stage);
    Note::Drawn {
        id: format!("{STEPS}{wish}"),
        picture: Picture::Track { steps: count, stage, file: String::new() },
        words: words.iter().filter(|text| !text.is_empty()).map(|text| text.to_string()).collect(),
        open: false,
        options: Vec::new(),
        field: None,
    }
}

/// The page `shown`, whose last `made` little ones are the day's made wishes, with what each working one's run is
/// doing: its words over it, and the newest one's track in the place of the note that says the made wishes are
/// being worked on, when the page has that note. Core is asked only when a made wish is working.
pub(crate) fn drawn(home: &Path, day: &str, made: usize, shown: &mut Scene) {
    let from = shown.characters.len().saturating_sub(made);
    if shown.characters[from..].iter().any(|one| one.state == State::Busy) {
        tracked(made, shown, &jellymoon_core::doing(home, day));
    }
}

/// [`drawn`] with core's answer given: `doing` is each working wish's steps, as `jellymoon_core::doing` gives them.
pub fn tracked(made: usize, shown: &mut Scene, doing: &[(String, Steps)]) {
    let from = shown.characters.len().saturating_sub(made);
    let steps = |id: &str| doing.iter().find(|(wish, _)| crate::scene::drawn_id(wish) == id).map(|(_, steps)| steps);
    let mut newest = None;
    for one in shown.characters[from..].iter_mut().filter(|one| one.state == State::Busy) {
        one.says = says(steps(&one.id));
        newest = Some(one.id.clone());
    }
    let going = |note: &Note| match note {
        Note::Plain { title, body } if crate::scene::GOING.contains(&title.as_str()) => {
            Some((title.clone(), body.clone()))
        }
        _ => None,
    };
    let found = shown.notes.iter().enumerate().find_map(|(at, note)| Some((at, going(note)?)));
    if let (Some(id), Some((at, (title, body)))) = (newest, found) {
        shown.notes[at] = note(&id, steps(&id), (&title, &body));
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
        let around = ("looking around", "Looking around your project.", Stage::Looking);
        let change = ("making the change", "Making the change.", Stage::Making);
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
        for (one, (said, sentence, stage)) in steps {
            let kept = Steps { count: 12, last: vec![step(Tool::Edit, Some("x.rs")), one.clone()] };
            assert_eq!(says(Some(&kept)), said, "{one:?}");
            let words = ["It is going.", "The wish.", sentence, "12 steps so far."];
            assert_eq!(note("m1", Some(&kept), ("It is going.", "The wish.")), track(12, stage, &words), "{one:?}");
        }
    }

    fn track(steps: usize, stage: Stage, words: &[&str]) -> Note {
        Note::Drawn {
            id: "steps-m1".to_string(),
            picture: Picture::Track { steps, stage, file: String::new() },
            words: words.iter().map(|word| word.to_string()).collect(),
            open: false,
            options: Vec::new(),
            field: None,
        }
    }

    /// A page with a made wish at work, `m1`, the newest, and the note that says so between two others.
    fn page() -> Scene {
        let plain = |title: &str| Note::Plain { title: title.to_string(), body: "The wish.".to_string() };
        let busy = sticker_kit::Character { id: "m1".to_string(), state: State::Busy, ..Default::default() };
        let notes = vec![plain("before"), plain(crate::scene::GOING[0]), plain("after")];
        Scene { characters: vec![busy], notes, ..Scene::default() }
    }

    // A wish's work is drawn: with no step, one and twelve, the page holds one track with core's count and the
    // stage of the newest step, in the place of the note that says the wish is being worked on, whose sentences
    // are its words, and no file's name is anywhere in it.
    #[test]
    fn a_working_wish_is_one_track_with_core_s_count_in_the_place_of_the_note_that_says_it_works() {
        let going = [crate::scene::GOING[0], "The wish."];
        let read = step(Tool::Read, Some("b.txt"));
        let twelve = Steps { count: 12, last: vec![read.clone(), step(Tool::Write, Some("new file.txt"))] };
        let cases = [
            (None, track(0, Stage::Thinking, &going)),
            (Some(Steps::default()), track(0, Stage::Thinking, &going)),
            (
                Some(Steps { count: 1, last: vec![read] }),
                track(1, Stage::Looking, &[going[0], going[1], "Looking around your project.", "1 step so far."]),
            ),
            (Some(twelve), track(12, Stage::Making, &[going[0], going[1], "Making the change.", "12 steps so far."])),
        ];
        for (steps, want) in cases {
            let (mut shown, plain) = (page(), page().notes);
            let doing: Vec<(String, Steps)> = steps.into_iter().map(|steps| ("m1".to_string(), steps)).collect();
            tracked(1, &mut shown, &doing);
            assert_eq!(shown.notes, [plain[0].clone(), want, plain[2].clone()], "{doing:?}");
            let page = format!("{shown:?}");
            assert!(!page.contains("b.txt") && !page.contains("new file.txt"), "{page}");
        }
        // No made wish at work, or no such note on the page: nothing changes.
        let mut idle = page();
        idle.characters[0].state = State::Idle;
        tracked(1, &mut idle, &[]);
        let mut bare = Scene { notes: Vec::new(), ..page() };
        tracked(1, &mut bare, &[]);
        assert_eq!((idle.notes, bare.notes.len(), bare.characters[0].says.as_str()), (page().notes, 0, THINKING));
    }
}
