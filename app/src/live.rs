//! What a made wish's run is doing while the wish works (live work; `docs/development/app.md`, "A wish at work").
//! Core keeps the run's steps as they come (`jellymoon_core::doing`); at each drawing of the page they are put
//! before the person: a few words over each working made wish's little one, and for each of them one drawn note
//! of how far it has come, a track (ADR 0029, ADR 0030), under the words typed for it. The sentences that say the
//! wish is being worked on are the track's words, with the stage and the count: on the page when the person taps the picture
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
/// the first. No file is drawn. Its words are `going`, the sentence that the wish is being worked on and the one
/// that names it, then the stage as a sentence and how many steps came, each only when there is one.
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
/// doing: its words over it, and its track, when the page has one. Core is asked only when a made wish is working.
pub(crate) fn drawn(home: &Path, day: &str, made: usize, shown: &mut Scene) {
    let from = shown.characters.len().saturating_sub(made);
    if shown.characters[from..].iter().any(|one| one.state == State::Busy) {
        tracked(made, shown, &jellymoon_core::doing(home, day));
    }
}

/// [`drawn`] with core's answer given: `doing` is each working wish's steps, as `jellymoon_core::doing` gives them.
/// Every working made wish has its words over it, and its own track (`steps-<wish>`, which `scene::ruled` put
/// on the page) with core's count of its own steps. A wish core has no steps for is thinking, with none.
pub fn tracked(made: usize, shown: &mut Scene, doing: &[(String, Steps)]) {
    let from = shown.characters.len().saturating_sub(made);
    let steps = |id: &str| doing.iter().find(|(wish, _)| crate::scene::drawn_id(wish) == id).map(|(_, steps)| steps);
    let mut working = Vec::new();
    for one in shown.characters[from..].iter_mut().filter(|one| one.state == State::Busy) {
        one.says = says(steps(&one.id));
        working.push(one.id.clone());
    }
    for track in &mut shown.notes {
        let Note::Drawn { id, words, .. } = track else { continue };
        let Some(wish) = working.iter().find(|wish| id.strip_prefix(STEPS) == Some(wish.as_str())) else { continue };
        if let [title, body, ..] = &words[..] {
            *track = note(wish, steps(wish), (&title.clone(), &body.clone()));
        }
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

    /// A page with two made wishes at work, `m1` and `m2`, each with its track, between two other notes.
    fn page() -> Scene {
        let plain = |title: &str| Note::Plain { title: title.to_string(), body: "The wish.".to_string() };
        let busy = |id: &str| sticker_kit::Character { id: id.to_string(), state: State::Busy, ..Default::default() };
        let going = (crate::scene::GOING, "The wish.");
        let notes = vec![plain("before"), note("m1", None, going), note("m2", None, going), plain("after")];
        Scene { characters: vec![busy("m1"), busy("m2")], notes, ..Scene::default() }
    }

    // A wish's work is drawn: with no step, one and twelve, its track has core's count and the stage of the newest
    // step, its words are the sentences that say the wish is being worked on, and no file's name is anywhere in it.
    // The other working wish's track is its own: core's count of the other's steps, never this one's.
    #[test]
    fn every_working_wish_has_its_own_track_with_core_s_count_of_its_own_steps() {
        let going = [crate::scene::GOING, "The wish."];
        let read = step(Tool::Read, Some("b.txt"));
        let twelve = Steps { count: 12, last: vec![read.clone(), step(Tool::Write, Some("new file.txt"))] };
        let cases = [
            (None, track(0, Stage::Thinking, &going)),
            (Some(Steps::default()), track(0, Stage::Thinking, &going)),
            (
                Some(Steps { count: 1, last: vec![read.clone()] }),
                track(1, Stage::Looking, &[going[0], going[1], "Looking around your project.", "1 step so far."]),
            ),
            (Some(twelve), track(12, Stage::Making, &[going[0], going[1], "Making the change.", "12 steps so far."])),
        ];
        let other = Steps { count: 3, last: vec![read] };
        for (steps, want) in cases {
            let (mut shown, plain) = (page(), page().notes);
            let mut doing: Vec<(String, Steps)> = steps.into_iter().map(|steps| ("m1".to_string(), steps)).collect();
            doing.push(("m2".to_string(), other.clone()));
            tracked(2, &mut shown, &doing);
            let theirs = note("m2", Some(&other), (going[0], going[1]));
            assert_eq!(shown.notes, [plain[0].clone(), want, theirs, plain[3].clone()], "{doing:?}");
            let page = format!("{shown:?}");
            assert!(!page.contains("b.txt") && !page.contains("new file.txt"), "{page}");
        }
        // No made wish at work, or no track on the page (one with a clash): no note changes.
        let mut idle = page();
        idle.characters.iter_mut().for_each(|one| one.state = State::Idle);
        tracked(2, &mut idle, &[("m1".to_string(), other)]);
        let mut bare = Scene { notes: Vec::new(), ..page() };
        tracked(2, &mut bare, &[]);
        assert_eq!((idle.notes, bare.notes.len(), bare.characters[0].says.as_str()), (page().notes, 0, THINKING));
    }
}
