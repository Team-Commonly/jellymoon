//! A made-up window for the F47 tests: what `window.rs` does with a tap, with no window. It is a program of its
//! own so that the git core starts has a test's variables and none of the caller's. Arguments: Jellymoon's home,
//! the logs, the harness, the day, and then the acts, done in their order. Each act but `hold` and `left` prints one line of
//! JSON on stdout: the scene the window would show, or `null` when it would show nothing.
//!
//! - `poke:<id>`: a tap on a character. The look is made on another thread, as the window makes it.
//! - `choose:<note>:<index>`: a tap on an option. The line is the scene shown at once; what the tap left to do is
//!   kept for `run`, as the window's other thread would hold it. Nothing of it has begun.
//! - `run`: the oldest work a tap left, on another thread, and the scene its answer shows.
//! - `hold:<file>`: waits until the test leaves that file, half a minute at most.
//! - `left:<file>`, as the first act only: the note in that file, as JSON, is on show from the start, as a page
//!   left open would have it. It prints no line.
//! - `core:<wish>:<index>`: no window at all: core's own answer to a tap on that choice of the wish's work, as
//!   `[title, body]`, or `null` when it is no sentence.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::VecDeque;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use std::{env, fs, io, thread};

use jellymoon_app::showing::{After, Showing};
use jellymoon_app::taps::OnShow;
use jellymoon_app::today::Today;
use jellymoon_core::{RunOptions, WorkDone, WorkTap};
use serde::Serialize;

fn show<T: Serialize>(shown: &T) {
    let mut stdout = io::stdout().lock();
    stdout.write_all(format!("{}\n", serde_json::to_string(shown).unwrap()).as_bytes()).unwrap();
    stdout.flush().unwrap();
}

/// Work as the window runs it: on another thread, while this one keeps the page.
fn elsewhere<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    let here = thread::current().id();
    thread::spawn(move || {
        assert_ne!(thread::current().id(), here);
        work()
    })
    .join()
    .unwrap()
}

fn core(today: &Today, wish: &str, index: usize) -> Option<(String, String)> {
    let options = RunOptions {
        wish: wish.to_string(),
        day: Some(today.date.clone()),
        repo: None,
        logs: Some(today.logs.clone()),
        harness: today.harness.clone(),
        skip: Vec::new(),
    };
    let choice = ["Show me", "Keep it", "Toss it"].get(index)?;
    match jellymoon_core::work_tapped(&today.home, &options, WorkTap::of(choice, &[], 0)?) {
        WorkDone::Said(title, body) => Some((title.to_string(), body)),
        _ => None,
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let [home, logs, harness, day, acts @ ..] = &args[..] else { panic!("four arguments, and the acts") };
    let today = Today {
        date: day.clone(),
        logs: PathBuf::from(logs),
        skip: Vec::new(),
        model: "sonnet".to_string(),
        home: PathBuf::from(home),
        harness: Some(PathBuf::from(harness)),
        fresh: false,
        settings: None,
        bin: None,
    };
    let (mut scene, page) = today.first();
    let acts = match acts.split_first() {
        Some((first, rest)) if first.starts_with("left:") => {
            let note = fs::read_to_string(first.trim_start_matches("left:")).unwrap();
            scene.notes.insert(0, serde_json::from_str(&note).unwrap());
            rest
        }
        _ => acts,
    };
    let showing = Showing::of(Some(OnShow { today: today.clone(), page, scene }));
    let mut left: VecDeque<After> = VecDeque::new();
    for act in acts {
        // A file's name can hold a colon of its own, so only `hold` keeps the rest whole.
        let parts: Vec<&str> =
            if act.starts_with("hold:") { act.splitn(2, ':').collect() } else { act.split(':').collect() };
        match parts[..] {
            ["poke", id] => {
                let look = showing.poked(id).unwrap();
                show(&elsewhere(look).and_then(|seen| showing.seen(seen)));
            }
            ["choose", note, index] => {
                let (scene, after) = showing.chose(note, index.parse().unwrap()).unzip();
                left.extend(after.flatten());
                show(&scene);
            }
            ["run"] => {
                let after = left.pop_front().expect("a tap left work");
                show(&showing.ended(elsewhere(move || after.run())));
            }
            ["hold", file] => {
                let until = Instant::now() + Duration::from_secs(30);
                while !Path::new(file).exists() && Instant::now() < until {
                    thread::sleep(Duration::from_millis(10));
                }
            }
            ["core", wish, index] => show(&core(&today, wish, index.parse().unwrap())),
            _ => panic!("no act {act:?}"),
        }
    }
}
