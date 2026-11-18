//! The window's loop, with no window: the calls `window.rs` makes, in its order, through `Showing`, against a real
//! core in the home the environment names (`JELLYMOON_HOME`, and the rest as `jellymoon-app` reads them). It is
//! meant for a made-up home: what it does is done there, a run included. It starts no program itself; core starts
//! what the app starts (a run, by the installed `jellymoon` command). After each step it prints one line: the step,
//! how long it took, the titles of the notes on the page and each wish's state. Never a word of a wish, a note's body
//! or a result: typed words are counted, and a folder is not named.
//!
//! `walk [--logs <dir>] <folder for the pictures> <W>x<H> <step>...`, the page first opened as the window opens it
//! (the entry, then the day's reading). `--logs` is the app's own: the day is read from that folder's logs, so a
//! walk with a real harness need not read the person's. The steps:
//! - `type:<words>`: the words in the entry, in place of what it held.
//! - `chip:<name>`: the chip of that project chosen.
//! - `enter`: Enter in the entry: a wish for the chosen chip, or on a first page a new project's name.
//! - `start`: `Start` tapped: Enter, or with nothing typed the note that says what to type first.
//! - `folder:<path>`: that folder chosen with `Choose a folder`, as the picker gives it.
//! - `look`: one look at the day, as the window makes once a second.
//! - `until:<name>:<seconds>`: a look once a second until a wish is in the state `name` (`working`, `done`,
//!   `needs_you`, `let_go`) or a note of that title or id is on the page; past the bound it fails.
//! - `poke:<wish id>`: a tap on that wish's character.
//! - `tap:<note title or id>:<option>`: a tap on that option, only where the page places it.
//! - `picture:<name>`: `<name>.png` in the folder, the page as it is now, in the sticker theme.
//!
//! A step that fails says why on one line on stderr, and the walk ends with 1.

use std::path::PathBuf;
use std::process::ExitCode;
use std::thread::sleep;
use std::time::{Duration, Instant};

use jellymoon_app::showing::{Entered, Showing};
use jellymoon_app::starting::{Given, Shown, named};
use jellymoon_app::taps::OnShow;
use jellymoon_app::today::{Asked, Today};
use jellymoon_app::{SMALLEST, file_part, lines, one_line, sides};
use serde_json::Value;
use sticker_kit::draw::headless::{Typing, snapshot_typing};
use sticker_kit::{Entry, Note, Scene, State, Theme, Viewport};

/// The page as the window holds it: the day behind it, and the entry with its words and its chosen chip.
struct Walk {
    shown: Showing,
    entry: Option<Entry>,
    typed: String,
    chosen: usize,
    size: Viewport,
    folder: PathBuf,
}

fn title(note: &Note) -> &str {
    match note {
        Note::List { title, .. }
        | Note::Choice { title, .. }
        | Note::Plain { title, .. }
        | Note::Text { title, .. }
        | Note::Whole { title, .. }
        | Note::Told { title, .. } => title,
    }
}

/// A note's id and its options, when it has any.
fn choices(note: &Note) -> Option<(&str, &[String])> {
    match note {
        Note::Choice { id, options, .. } | Note::Text { id, options, .. } => Some((id, options)),
        _ => None,
    }
}

fn id_of(note: &Note) -> Option<&str> {
    match note {
        Note::Told { id, .. } => Some(id),
        _ => choices(note).map(|(id, _)| id),
    }
}

/// A wish's state in core's words.
fn state(state: State) -> &'static str {
    match state {
        State::Busy => "working",
        State::Idle => "done",
        State::NeedsYou => "needs_you",
        State::Dimmed => "let_go",
    }
}

impl Walk {
    /// The first scene, the first page's entry, then each step of the day's reading, as `window::reads` takes them.
    fn open(logs: Option<PathBuf>, folder: PathBuf, size: Viewport) -> Result<Walk, String> {
        let today = Today::from_env(Asked { logs, ..Asked::default() })?;
        let (scene, page) = today.first();
        let mut later = Some(today.clone().later(page.clone()));
        let shown = Showing::of(Some(OnShow { today, page, scene }));
        let mut walk = Walk { shown, entry: None, typed: String::new(), chosen: 0, size, folder };
        walk.set_entry(walk.shown.offering().and_then(|offering| offering()));
        while let Some(work) = later.take().and_then(|step| walk.shown.reading(step)) {
            let Some((next, entry)) = work() else { break };
            walk.shown.read(&next);
            walk.set_entry(entry);
            later = next.later;
        }
        Ok(walk)
    }

    fn scene(&self) -> Scene {
        self.shown.scene().unwrap_or_default()
    }

    fn at(&self, chip: &str) -> Option<usize> {
        self.entry.as_ref()?.options.iter().position(|option| option == chip)
    }

    /// The entry the page is given. One that holds words is never taken away, and the chosen chip stays chosen;
    /// the project core listed has its chip chosen as soon as an entry offers it (`window::choose_waiting`).
    fn set_entry(&mut self, entry: Option<Entry>) {
        if entry.is_none() && !self.typed.trim().is_empty() {
            return;
        }
        let chip = self.entry.as_ref().and_then(|now| now.options.get(self.chosen)).cloned();
        self.entry = entry;
        self.chosen = chip.and_then(|chip| self.at(&chip)).unwrap_or(0);
        if let Some(at) = self.shown.waiting().and_then(|name| self.at(&name)) {
            self.chosen = at;
            self.shown.wait_for(None);
        }
    }

    /// Enter: a new project's name in the entry that takes one, else a wish for the chosen chip, and its run.
    fn enter(&mut self) -> Result<(), String> {
        let entry = self.entry.clone().ok_or("no entry is on the page")?;
        if let Some(given) = named(&entry.id, &self.typed) {
            return self.list(given, true);
        }
        let chip = entry.options.get(self.chosen).cloned();
        let answer = self.shown.wishing(&self.typed, chip.as_deref()).map(|wishing| wishing());
        let Entered { kept, asking, .. } = self.shown.answered(answer);
        if kept {
            self.typed.clear();
        }
        if let Some(asking) = asking {
            self.shown.asked(asking());
        }
        Ok(())
    }

    /// A folder or a name given to core; then the page's entry, asked again while the project has no chip.
    fn list(&mut self, given: Given, entered: bool) -> Result<(), String> {
        let listed = self.shown.listing(given).map(|listing| listing());
        let Some(Shown { entry, name, .. }) = listed.and_then(|listed| self.shown.listed(listed)) else {
            return Ok(());
        };
        if entered && name.is_some() {
            self.typed.clear();
        }
        self.shown.wait_for(name);
        self.set_entry(entry);
        if self.shown.waiting().is_some() {
            let offered = self.shown.offering().and_then(|offering| offering());
            self.set_entry(offered);
        }
        Ok(())
    }

    fn look(&mut self, look: Option<jellymoon_app::wishing::Look>) {
        if let Some(seen) = look.and_then(|look| look()) {
            self.shown.seen(seen);
        }
    }

    /// Whether a wish is in the state `name`, or a note of that title or id is on the page.
    fn on_page(&self, name: &str) -> bool {
        let scene = self.scene();
        scene.notes.iter().any(|note| title(note) == name || id_of(note) == Some(name))
            || scene.characters.iter().any(|wish| state(wish.state) == name)
    }

    fn until(&mut self, name: &str, seconds: u64) -> Result<(), String> {
        let bound = Instant::now() + Duration::from_secs(seconds);
        while !self.on_page(name) {
            if Instant::now() >= bound {
                return Err(format!("{name:?} was not on the page after {seconds} seconds"));
            }
            sleep(Duration::from_secs(1));
            self.look(self.shown.look());
        }
        Ok(())
    }

    /// A tap on an option the page places, as the kit's view gives one; what it leaves is run, and shown.
    fn tap(&mut self, note: &str, option: &str) -> Result<(), String> {
        let scene = self.scene();
        let named = |one: &&Note| title(one) == note || id_of(one) == Some(note);
        let found = scene.notes.iter().find(named).and_then(choices);
        let (id, options) = found.ok_or_else(|| format!("no note {note:?}"))?;
        let index = options.iter().position(|one| one == option).ok_or_else(|| format!("no option {option:?}"))?;
        let placed = lines::lines(&scene, self.entry.as_ref(), &Theme::sticker(), self.size);
        let is = |piece: Value| piece["kind"] == "option" && piece["id"] == id && piece["index"] == index;
        if !placed.lines().filter_map(|line| serde_json::from_str(line).ok()).any(is) {
            return Err(format!("{option:?} of {note:?} is not placed on the page"));
        }
        if let Some((_, Some(after))) = self.shown.chose(id, index) {
            self.shown.ended(after.run());
        }
        Ok(())
    }

    fn picture(&self, name: &str) -> Result<(), String> {
        let out = self.folder.join(format!("{}.png", file_part(name)));
        let typing = self.entry.clone().map(|entry| Typing { entry, typed: Some((self.typed.clone(), self.chosen)) });
        snapshot_typing(self.scene(), Theme::sticker(), typing, self.size.w, self.size.h, &out)
            .map_err(|error| format!("could not draw {name:?}: {error}"))
    }

    fn step(&mut self, step: &str) -> Result<(), String> {
        let (name, rest) = step.split_once(':').unwrap_or((step, ""));
        let split = |rest: &str| rest.rsplit_once(':').map(|(a, b)| (a.to_string(), b.to_string()));
        match name {
            "type" => self.typed = rest.to_string(),
            "chip" => self.chosen = self.at(rest).ok_or_else(|| format!("no chip {rest:?}"))?,
            "enter" => self.enter()?,
            "start" if self.typed.trim().is_empty() => {
                let id = self.entry.as_ref().map(|entry| entry.id.clone()).ok_or("no entry is on the page")?;
                self.shown.unready(&id);
            }
            "start" => self.enter()?,
            "folder" => self.list(Given::Folder(PathBuf::from(rest)), false)?,
            "look" => self.look(self.shown.look()),
            "until" => {
                let (what, seconds) = split(rest).ok_or("until:<name>:<seconds>")?;
                self.until(&what, seconds.parse().map_err(|_| "until:<name>:<seconds>")?)?;
            }
            "poke" => self.look(self.shown.poked(rest)),
            "tap" => {
                let (note, option) = split(rest).ok_or("tap:<note>:<option>")?;
                self.tap(&note, &option)?;
            }
            "picture" => self.picture(rest)?,
            _ => return Err(format!("no step {name:?}")),
        }
        Ok(())
    }

    /// The line after a step: titles and states only.
    fn said(&self, step: &str, took: Duration) -> String {
        let step = match step.split_once(':') {
            Some(("type", words)) => format!("type ({} characters)", words.chars().count()),
            Some(("folder", _)) => "folder".to_string(),
            _ => one_line(step),
        };
        let scene = self.scene();
        let notes: Vec<String> = scene.notes.iter().map(|note| format!("{:?}", title(note))).collect();
        let wishes: Vec<String> =
            scene.characters.iter().map(|wish| format!("{} {}", one_line(&wish.id), state(wish.state))).collect();
        format!("{step} | {} ms | notes: {} | wishes: {}", took.as_millis(), notes.join(", "), wishes.join(", "))
    }
}

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let logs = (args.first().map(String::as_str) == Some("--logs") && args.len() > 1)
        .then(|| PathBuf::from(args.drain(..2).nth(1).unwrap_or_default()));
    let [folder, size, steps @ ..] = args.as_slice() else {
        eprintln!("usage: walk [--logs <dir>] <folder> <W>x<H> <step>...");
        return ExitCode::from(2);
    };
    let Some(size) = sides(size, SMALLEST) else {
        eprintln!("walk: the size must be <W>x<H>, from 960x640");
        return ExitCode::from(2);
    };
    let started = Instant::now();
    let mut walk = match Walk::open(logs, PathBuf::from(folder), size) {
        Ok(walk) => walk,
        Err(why) => {
            eprintln!("walk: {}", one_line(&why));
            return ExitCode::FAILURE;
        }
    };
    println!("{}", walk.said("open", started.elapsed()));
    for step in steps {
        let started = Instant::now();
        if let Err(why) = walk.step(step) {
            eprintln!("walk: {step:?}: {}", one_line(&why));
            return ExitCode::FAILURE;
        }
        println!("{}", walk.said(step, started.elapsed()));
    }
    ExitCode::SUCCESS
}
