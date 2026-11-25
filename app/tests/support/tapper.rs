//! A made-up window for the F47 tests: what `window.rs` does with a tap, with no window. It is a program of its
//! own so that the git core starts has a test's variables and none of the caller's. Arguments: Jellymoon's home,
//! the logs, the harness, the day, and then the acts, done in their order. Each act but `hold`, `left`, `bin` and `size`
//! prints one line of JSON on stdout: the scene the window would show, or `null` when it would show nothing.
//!
//! - `poke:<id>`: a tap on a character. The look is made on another thread, as the window makes it.
//! - `look`: the look the window makes once a second, made on another thread too.
//! - `choose:<note>:<index>`: a tap on an option. The window is given a tap by the kit's view, which finds one
//!   only among the pieces it placed: so the scene on show is laid out here as `layout` lays it out, at the
//!   window's size, and a tap on an option that is not placed is no tap, and prints `null`. The line is the
//!   scene shown at once; what the tap left to do is kept for `run`, as the window's other thread would hold
//!   it. Nothing of it has begun. The tap carries the option's words as the view holds them, as the kit's does.
//! - `choose:<note>:<index>:<words>`: the same tap carrying those words: the ones a drawing before this one had
//!   at that place, for a click on a page that changed under the pointer.
//! - `size:<W>x<H>`: the window's size from here on. It is 1440x900 until then.
//! - `placed`: the pieces `layout` places of the scene on show at the window's size, as a list.
//! - `column`: the scene on show as the window gives it to the kit's view (`Showing::column`): its drawn notes
//!   open or closed, the offer of a follow-up under one wish, in the order of its column.
//! - `run`: the oldest work a tap left, on another thread, and the scene its answer shows.
//! - `hold:<file>`: waits until the test leaves that file, half a minute at most.
//! - `left:<file>`, as the first act only: the note in that file, as JSON, is on show from the start, as a page
//!   left open would have it. It prints no line.
//! - `bin:<folder>`, as the first act only: the folder the installed `jellymoon` command is in. It prints no
//!   line. Without it there is none, as for a person who has not installed it.
//! - `type:<note>:<words>`: Enter in the field of a note, with those words in it (F54). The kit gives an Enter
//!   only from a field it placed, so the scene is laid out as for `choose`, and a note whose field is not placed
//!   prints `null`. Core is asked on another thread. The line is `{"kept":…,"scene":…,"at_once":…}`: what the
//!   field is told, the scene shown, and the scene shown at once, before core was asked, or `null` when
//!   nothing was. The asking for a run it left is kept for `start`.
//! - `enter:<note>:<words>` and `answer`: a `type` in two acts, so that a test can act between Enter and core's
//!   answer. `enter` prints the scene shown at once, or `null`, and keeps what is to be asked of core; `answer`
//!   asks it on another thread and prints `{"kept":…,"scene":…}`.
//! - `enter:wish@<project>:<words>` and `type:wish@<project>:<words>`: the same for Enter in the entry, with
//!   that project's chip chosen (F91): the scene shown at once says the wish is being read.
//! - `press:wish@<project>:<words>`: the words typed into the window's entry, which is the kit's own state of
//!   one (`EntryState`), and Enter pressed there with that project's chip chosen. What the entry gives goes on
//!   as `enter:wish@…` does; an entry that is owed its answer gives nothing, and the line is `null`. The
//!   `answer` to an Enter the entry gave answers the entry as `window.rs` does, and its line has two more
//!   keys: `again`, the scene shown at once by an Enter the entry remembered and gave then, or `null`, and
//!   `entry`, the words typed in it then.
//! - `start`: the oldest asking a `type` left, on another thread, and the scene it shows, or `null`.
//! - `core:<wish>:<index>`: no window at all: core's own answer to a tap on that choice of the wish's work, as
//!   `[title, body]`, or `null` when it is no sentence.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::VecDeque;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use std::{env, fs, io, thread};

use jellymoon_app::showing::{After, Entered, Showing};
use jellymoon_app::taps::OnShow;
use jellymoon_app::today::Today;
use jellymoon_app::wishing::{Asking, Look, Wishing};
use jellymoon_core::{RunOptions, WorkDone, WorkTap};
use serde::Serialize;
use serde_json::{Value, json};
use sticker_kit::entry::{EntryState, Made};
use sticker_kit::{Scene, Theme, Viewport};

/// The made-up window's entry: the kit's state of it, and the project on its chosen chip.
#[derive(Default)]
struct Typed(EntryState, String);

/// What each Enter left to ask of core, oldest first, and whether the entry gave it and is owed its answer.
type Entering = VecDeque<(Wishing, bool)>;

impl Typed {
    /// `words` typed where the caret is, and Enter with the chip of `note`'s project chosen: what the entry gives.
    fn pressed(&mut self, note: &str, words: &str) -> Option<Made> {
        self.1 = note.strip_prefix("wish@").expect("the entry").to_string();
        self.0.replace(None, words);
        self.0.enter()
    }

    /// What the entry gave, carried on as the window's handler carries an Enter: the scene shown at once, with
    /// what is to be asked of core kept as the entry's own.
    fn given(&self, showing: &Showing, made: Option<Made>, entering: &mut Entering) -> Option<Scene> {
        let wishing = showing.wishing(&made?.text, Some(&self.1))?;
        entering.push_back((wishing, true));
        showing.scene()
    }
}

/// A look or a tap on a character, made on another thread, and what the window then shows.
fn looked(showing: &Showing, now: &mut Scene, look: Look) {
    shown(now, elsewhere(look).and_then(|seen| showing.seen(seen)));
}

/// The pieces `layout` places of a scene: what `jellymoon-app layout` prints of it, a piece a line.
fn placed(scene: &Scene, size: Viewport) -> Vec<Value> {
    let lines = jellymoon_app::lines::lines(scene, None, &Theme::sticker(), size);
    lines.lines().map(|line| serde_json::from_str(line).unwrap()).collect()
}

/// Prints what an act showed, and keeps it as the scene on show when it showed one.
fn shown(now: &mut Scene, scene: Option<Scene>) {
    show(&scene);
    if let Some(scene) = scene {
        *now = scene;
    }
}

/// Whether a tap can be on that option: it is one of the placed pieces.
fn tappable(scene: &Scene, size: Viewport, note: &str, index: usize) -> bool {
    let is = |piece: &Value| piece["kind"] == "option" && piece["id"] == note && piece["index"] == index;
    placed(scene, size).iter().any(is)
}

/// A tap on option `index` of `note`, when `layout` places it: the scene shown at once, and what the tap left to
/// do. It carries the words `drawn`, or else the option's words as the view holds them.
fn chose(showing: &Showing, now: &mut Scene, size: Viewport, tap: (&str, &str, Option<&str>)) -> Option<After> {
    let (note, index, drawn) = tap;
    let index = index.parse().unwrap();
    let words = drawn.or(now.option(note, index)).unwrap_or_default().to_string();
    let tapped = tappable(now, size, note, index).then(|| showing.chose(note, index, &words)).flatten();
    let (scene, after) = tapped.unzip();
    shown(now, scene);
    after.flatten()
}

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

fn core(today: &Today, wish: &str, index: usize) -> Option<(String, Vec<String>)> {
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

/// The day the window opens on, the scene on show at its start, and the acts that are left to do.
fn opened(args: &[String]) -> (Today, Scene, Showing, &[String]) {
    let [home, logs, harness, day, acts @ ..] = args else { panic!("four arguments, and the acts") };
    let (bin, acts) = match acts.split_first() {
        Some((first, rest)) if first.starts_with("bin:") => (Some(PathBuf::from(&first[4..])), rest),
        _ => (None, acts),
    };
    let today = Today {
        date: day.clone(),
        logs: PathBuf::from(logs),
        skip: Vec::new(),
        model: "sonnet".to_string(),
        home: PathBuf::from(home),
        harness: Some(PathBuf::from(harness)),
        fresh: false,
        settings: None,
        bin,
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
    let showing = Showing::of(Some(OnShow { today: today.clone(), page, scene: scene.clone(), typed: None }));
    (today, scene, showing, acts)
}

/// Enter in the field of `note`, when `layout` places one: the scene shown at once, and what is to be asked of
/// core.
fn entered(
    showing: &Showing,
    now: &Scene,
    size: Viewport,
    note: &str,
    typed: &str,
) -> Option<(Option<Scene>, Wishing)> {
    if let Some(project) = note.strip_prefix("wish@") {
        let wishing = showing.wishing(typed, Some(project))?;
        return Some((showing.scene(), wishing));
    }
    let is = |piece: &Value| piece["kind"] == "field" && piece["id"] == note;
    placed(now, size).iter().any(is).then(|| showing.typing(note, typed)).flatten()
}

/// Core's answer to an Enter, asked for on another thread: the line of what the field is told and the scene
/// then on show, with the scene shown at once when the Enter's own act did not print it. The asking for a run
/// it left is given back. `entry` is the entry when it gave that Enter: it is answered as `window.rs` answers
/// it, forgetting the Enter it remembered when the page takes none, and what it gives then is carried on.
fn answered(
    showing: &Showing,
    typing: Wishing,
    at_once: Option<Option<Scene>>,
    now: &mut Scene,
    entry: Option<(&mut Typed, &mut Entering)>,
) -> Option<Asking> {
    let Entered { kept, scene, asking } = showing.answered(Some(elsewhere(typing)));
    let mut line = json!({ "kept": kept, "scene": scene });
    if let Some(at_once) = at_once {
        line["at_once"] = json!(at_once);
    }
    let mut on_show = scene;
    if let Some((entry, entering)) = entry {
        if !showing.takes_enter() {
            entry.0.forget_enter();
        }
        let made = entry.0.answer_and_enter(kept);
        let again = entry.given(showing, made, entering);
        (line["again"], line["entry"]) = (json!(again), json!(entry.0.text()));
        on_show = again.or(on_show);
    }
    show(&line);
    if let Some(scene) = on_show {
        *now = scene;
    }
    asking
}

/// Waits until the test leaves that file, half a minute at most.
fn held(file: &str) {
    let until = Instant::now() + Duration::from_secs(30);
    while !Path::new(file).exists() && Instant::now() < until {
        thread::sleep(Duration::from_millis(10));
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let (today, scene, showing, acts) = opened(&args);
    let mut left: VecDeque<After> = VecDeque::new();
    let (mut asked, mut entering, mut entry) = (VecDeque::new(), Entering::new(), Typed::default());
    // The scene on show, as the view has it, and the window's size.
    let (mut now, mut size) = (scene, Viewport { w: 1440.0, h: 900.0 });
    for act in acts {
        // A file's name can hold a colon of its own, so only `hold` keeps the rest whole.
        let parts: Vec<&str> = match act.split_once(':').map(|(name, _)| name) {
            Some("hold") => act.splitn(2, ':').collect(),
            Some("type" | "enter" | "press") => act.splitn(3, ':').collect(),
            _ => act.split(':').collect(),
        };
        match parts[..] {
            ["poke", id] => looked(&showing, &mut now, showing.poked(id).unwrap()),
            ["look"] => looked(&showing, &mut now, showing.look().unwrap()),
            ["choose", note, index] => left.extend(chose(&showing, &mut now, size, (note, index, None))),
            ["choose", note, index, drawn] => left.extend(chose(&showing, &mut now, size, (note, index, Some(drawn)))),
            ["run"] => {
                let after = left.pop_front().expect("a tap left work");
                shown(&mut now, showing.ended(elsewhere(move || after.run())));
            }
            ["size", given] => {
                let (w, h) = given.split_once('x').unwrap();
                size = Viewport { w: w.parse().unwrap(), h: h.parse().unwrap() };
            }
            ["type", note, typed] => match entered(&showing, &now, size, note, typed) {
                Some((at_once, typing)) => asked.extend(answered(&showing, typing, Some(at_once), &mut now, None)),
                None => show(&Value::Null),
            },
            ["enter", note, typed] => {
                let (at_once, typing) = entered(&showing, &now, size, note, typed).expect("a field on show");
                shown(&mut now, at_once);
                entering.push_back((typing, false));
            }
            ["press", note, typed] => {
                let made = entry.pressed(note, typed);
                shown(&mut now, entry.given(&showing, made, &mut entering));
            }
            ["answer"] => {
                let (typing, pressed) = entering.pop_front().expect("an Enter left an answer to ask for");
                let entry = pressed.then_some((&mut entry, &mut entering));
                asked.extend(answered(&showing, typing, None, &mut now, entry));
            }
            ["start"] => {
                let asking = asked.pop_front().expect("a follow-up left an asking");
                shown(&mut now, showing.asked(elsewhere(asking)));
            }
            ["placed"] => show(&placed(&now, size)),
            ["column"] => show(&showing.column(&now)),
            ["hold", file] => held(file),
            ["core", wish, index] => show(&core(&today, wish, index.parse().unwrap())),
            _ => panic!("no act {act:?}"),
        }
    }
}
