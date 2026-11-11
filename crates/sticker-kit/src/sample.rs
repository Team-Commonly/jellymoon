//! Invented scenes for the examples and the tests.

use crate::scene::{Character, LegendItem, Mark, Mood, Note, Scene, State};

fn small(id: &str, label: &str, hue: u8, stars: u8, state: State) -> Character {
    Character { id: id.into(), label: label.into(), hue, stars, state, mood: Mood::Happy }
}

fn pair(a: &str, b: &str) -> (String, String) {
    (a.into(), b.into())
}

impl Scene {
    /// A full page: a big character, eight small ones, three kinds of note.
    pub fn sample() -> Scene {
        let mut characters = vec![
            small("garden", "Plant the herb garden", 1, 3, State::Busy),
            small("bread", "Bake sourdough", 2, 1, State::Busy),
            small("letters", "Write to grandma", 3, 2, State::Idle),
            small("kite", "Fix the red kite", 4, 1, State::Idle),
            small("picnic", "Plan the picnic", 1, 1, State::NeedsYou),
            small("songs", "Learn two songs", 2, 0, State::Busy),
            small("shelf", "Sort the 本棚 shelf", 3, 0, State::NeedsYou),
            small("swim", "Morning swim club", 4, 0, State::Dimmed),
        ];
        characters[4].mood = Mood::Cross;
        Scene {
            date_label: "Sat · May 9".into(),
            title: "what I hoped for today".into(),
            hero: Some(small("me", "", 0, 0, State::Idle)),
            characters,
            notes: vec![
                Note::List {
                    title: "done today".into(),
                    rows: vec![
                        pair("Basil and mint potted", "Plant the herb garden"),
                        pair("Seeds labelled", "Plant the herb garden"),
                        pair("Watering can mended", "Plant the herb garden"),
                        pair("First loaf out of the oven", "Bake sourdough"),
                        pair("Two pages posted", "Write to grandma"),
                        pair("New string on the kite", "Fix the red kite"),
                    ],
                },
                Note::Choice {
                    id: "picnic-place".into(),
                    title: "you changed your mind".into(),
                    options: vec![
                        "“let's have the picnic by the lake”".into(),
                        "“the hill has the best view, let's eat up there”".into(),
                    ],
                    footer: "the lake was let go".into(),
                    struck_first: true,
                    whole: false,
                },
            ],
            legend: vec![
                legend(Mark::Star, "finished"),
                legend(Mark::Busy, "working on it"),
                legend(Mark::NeedsYou, "needs you"),
                legend(Mark::Dimmed, "set aside"),
            ],
            footer: "Sample content, made up for the kit.".into(),
        }
    }

    /// `count` small characters and nothing else to read, for measuring
    /// how the page holds up when it is crowded.
    pub fn crowd(count: usize) -> Scene {
        let states = [State::Busy, State::Idle, State::NeedsYou, State::Idle];
        let characters = (0..count)
            .map(|i| {
                let id = format!("c{i}");
                let label = format!("Job {i}");
                small(&id, &label, (i % 5) as u8, (i % 4) as u8, states[i % 4])
            })
            .collect();
        Scene { date_label: "crowd".into(), title: format!("{count} at once"), characters, ..Scene::default() }
    }
}

fn legend(mark: Mark, text: &str) -> LegendItem {
    LegendItem { mark, text: text.into() }
}
