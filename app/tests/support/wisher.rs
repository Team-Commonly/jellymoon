//! A made-up window for the F42 tests: what `window.rs` does with Enter in the entry, with no window. It is a
//! program of its own so that core's `run_apart` is called with a test's variables and none of the caller's: the
//! command core starts inherits them. Arguments: Jellymoon's home, the logs, the folder of the installed command,
//! the harness, the day, the name on the chosen chip, the words, what to do then, and the `--skip` value, which
//! may be empty. `watch` looks after the asking, as the window does, until a look gives a scene. `ask` enters
//! nothing: the wish is made as `jellymoon wish` makes one, and only its run is asked for. Anything else ends at
//! once. Each scene the window would show is one line of JSON on stdout.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::{env, io, thread};

use jellymoon_app::taps::OnShow;
use jellymoon_app::today::Today;
use sticker_kit::Scene;

fn show(scene: &Scene) {
    let mut stdout = io::stdout().lock();
    stdout.write_all(format!("{}\n", serde_json::to_string(scene).unwrap()).as_bytes()).unwrap();
    stdout.flush().unwrap();
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let [home, logs, bin, harness, day, project, typed, then, skip] = &args[..] else { panic!("nine arguments") };
    let today = Today {
        date: day.clone(),
        logs: PathBuf::from(logs),
        skip: if skip.is_empty() { Vec::new() } else { jellymoon_core::skip_names(skip).unwrap() },
        model: "sonnet".to_string(),
        home: PathBuf::from(home),
        harness: Some(PathBuf::from(harness)),
        fresh: false,
        settings: None,
        bin: Some(PathBuf::from(bin)),
    };
    let (scene, page) = today.first();
    let mut shown = OnShow { today, page, scene };
    let asking = if then == "ask" {
        let made = jellymoon_core::wish(&shown.today.home, Some(day), typed, project).unwrap().wish;
        Some(shown.today.asking(made.id, made.said))
    } else {
        let (scene, asking) = shown.wished(typed, Some(project));
        show(&scene);
        asking
    };
    if let Some(scene) = asking.and_then(|asking| shown.asked(asking())) {
        show(&scene);
    }
    let until = Instant::now() + Duration::from_secs(30);
    while then == "watch" && Instant::now() < until {
        if let Some(scene) = (shown.look())().and_then(|seen| shown.seen(seen)) {
            return show(&scene);
        }
        thread::sleep(Duration::from_millis(20));
    }
}
