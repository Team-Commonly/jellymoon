//! Invented scenes for the examples and the tests.

use crate::scene::{Bar, Character, Kept, Mood, Note, Picture, Scene, Stage, State};

fn small(id: &str, label: &str, hue: u8, stars: u8, state: State) -> Character {
    Character { id: id.into(), label: label.into(), hue, stars, state, mood: Mood::Happy, says: String::new() }
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
            footer: "Sample content, made up for the kit.".into(),
        }
    }

    /// Every picture a drawn note can hold, in each of its states, down one column. The fourth is open, with its
    /// words and its options; the fifth has words and is closed.
    pub fn pictures() -> Scene {
        let track = |steps, stage| Picture::Track { steps, stage, file: "hello.py".into() };
        let sheets = |made, changed, removed, state| Picture::Sheets { made, changed, removed, state };
        let bar = |name: &str, added, taken| Bar { name: name.into(), added, taken };
        let rows = vec![bar("src/hello.py", 30, 10), bar("README.md", 12, 0), bar("notes of mine.txt", 0, 7)];
        let pictures = vec![
            track(3, Stage::Thinking),
            track(7, Stage::Looking),
            track(26, Stage::Making),
            sheets(1, 0, 0, Kept::Waiting),
            sheets(3, 3, 2, Kept::In),
            sheets(1, 1, 1, Kept::Tossed),
            sheets(0, 0, 0, Kept::Waiting),
            Picture::Bars { rows, more: 2 },
        ];
        let drawn = |(n, picture): (usize, Picture)| Note::Drawn {
            id: format!("drawn-{n}"),
            picture,
            words: if n == 3 || n == 4 {
                vec!["Made up: the change is in a copy until it is kept.".into()]
            } else {
                Vec::new()
            },
            open: n == 3,
            options: if n == 3 { vec!["Keep it".into(), "Toss it".into()] } else { Vec::new() },
            field: None,
        };
        let notes = pictures.into_iter().enumerate().map(drawn).collect();
        Scene { date_label: "pencil".into(), title: "what the pencil draws".into(), notes, ..Scene::default() }
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
