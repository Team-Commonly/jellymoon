//! F54: a made wish whose run asks is drawn with its question and a field to answer in, and one whose work
//! waits or was kept offers `More like this` with a field. The rules and every refusal are core's, proved at
//! `jellymoon run answer`, `wish --after` and `run waiting` (`crates/jellymoon/tests/f52.rs`, `f53.rs`,
//! `f54.rs`). Here is the app's part: what `layout` prints of such a wish, and what is typed, carried as the
//! window carries it by a made-up window in a process of its own (`support/tapper.rs`). The wishes are made and
//! run by the `jellymoon` command with the made-up agent of F39's tests: nothing here starts the real Claude
//! Code, and no window is opened.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "f54/answered.rs"]
mod answered;
#[path = "f54/drawn.rs"]
mod drawn;
#[path = "f54/more.rs"]
mod more;
mod support;
#[cfg(all(target_os = "macos", feature = "window"))]
#[path = "f54/window.rs"]
mod window;

use std::fs;

use serde_json::Value;
use sticker_kit::{Note, Scene};
use support::Piece;
use support::day::DAY;
use support::wished::{SAID, World, wait_until, world};

/// What the made-up agent asks when its question is a plain one.
pub const QUESTION: &str = "Which of the two uploaders did you mean?";
pub const NEEDS: &str = "This wish needs you.";
pub const WORK: &str = "Work waits for this wish.";
pub const DONE: &str = "This wish is done.";
pub const MORE: &str = "More like this";
pub const ANSWER_HINT: &str = "Type your answer, then Enter";
pub const MORE_HINT: &str = "Type what to do next, then Enter";
/// The key of the field a wish's question is answered in.
pub fn answer_key(wish: &str, question: &str) -> String {
    format!("answer-{wish}-{question}")
}

pub const UNANSWERED: &str = "That answer was not taken.";
pub const UNSETTLED: &str = "The answer was given, but what came after it failed.";
pub const AT_WORK: &str = "This wish is being worked on.";
pub const UNKEPT: &str = "That wish was not kept.";
pub const UNSTARTED: &str = "Your wish is kept, and no run was started for it.";

pub fn quoted(said: &str) -> String {
    format!("“{said}”")
}

/// The world of `wished.rs` with a harness folder of its own, and no made wish yet.
pub fn ready() -> World {
    let w = world();
    fs::create_dir(w.at("harness")).unwrap();
    w
}

impl World {
    /// Tells the made-up agent which script to play, and its number.
    pub fn plays(&self, script: &str, number: usize) {
        fs::write(self.at("agent").join("play"), format!("{script}\n{}\n{number}\n", self.at("rec").display()))
            .unwrap();
    }

    /// A wish for `alpha` made in a terminal: its id.
    pub fn wishes(&self) -> String {
        let made = self.jellymoon(&["wish", "--date", DAY, "--say", SAID, "--project", "alpha"]);
        made["wish"]["id"].as_str().unwrap().to_string()
    }

    /// `jellymoon run <verb>` for a made wish, in the repo `alpha`: the line it printed.
    pub fn runs(&self, verb: &[&str], wish: &str) -> Value {
        let (repo, logs) = (self.at("work").join("alpha"), self.at("logs"));
        let named = ["--wish", wish, "--date", DAY, "--repo", repo.to_str().unwrap(), "--logs", logs.to_str().unwrap()];
        self.jellymoon(&[&["run"], verb, &named[..]].concat())
    }

    /// A wish for `alpha` and its run, with the agent playing `script` with `number`: the wish's id.
    pub fn ran(&self, script: &str, number: usize, outcome: &str) -> String {
        self.plays(script, number);
        let wish = self.wishes();
        let ran = self.runs(&[], &wish);
        assert_eq!(ran["outcome"], outcome, "{ran}");
        wish
    }

    /// A made wish as it is kept.
    pub fn wish(&self, id: &str) -> Value {
        let kept = fs::read_to_string(self.at("jm").join("pages").join(format!("{DAY}.wishes.json"))).unwrap();
        let kept: Value = serde_json::from_str(&kept).unwrap();
        kept["wishes"].as_array().unwrap().iter().find(|kept| kept["id"] == id).cloned().unwrap_or_default()
    }
}

/// The pieces of one kind that carry an id, in the order they were placed.
pub fn of<'a>(placed: &'a [Piece], kind: &str, id: &str) -> Vec<&'a Piece> {
    placed.iter().filter(|piece| piece.kind() == kind && piece.id() == id).collect()
}

/// One note of a wish on a scene: its title, body, quoted words, options, and its field's offer and hint.
pub type Text<'a> = (&'a str, Vec<&'a str>, &'a str, Vec<&'a str>, Option<(&'a str, &'a str)>);

/// The note with that id on a scene.
pub fn text<'a>(scene: &'a Scene, id: &str) -> Option<Text<'a>> {
    fn strs(all: &[String]) -> Vec<&str> {
        all.iter().map(String::as_str).collect()
    }
    scene.notes.iter().find_map(|note| match note {
        Note::Text { id: own, title, body, options, quoted, field } if own == id => {
            let field = field.as_ref().map(|field| (field.offer.as_str(), field.hint.as_str()));
            Some((title.as_str(), strs(body), quoted.as_str(), strs(options), field))
        }
        _ => None,
    })
}

/// The titles and bodies of a scene's plain notes, in their order.
pub fn plain(scene: &Scene) -> Vec<(&str, &str)> {
    let notes = scene.notes.iter().filter_map(|note| match note {
        Note::Plain { title, body } => Some((title.as_str(), body.as_str())),
        _ => None,
    });
    notes.collect()
}

/// What the made-up window printed for its acts, once it has ended: a value for each act that prints.
pub fn acted(w: &World, acts: &[&str]) -> Vec<Value> {
    w.taps("logs", acts).ended().iter().map(|line| serde_json::from_str(line).expect(line)).collect()
}

/// A scene the made-up window printed.
pub fn scene(printed: &Value) -> Scene {
    serde_json::from_value(printed.clone()).expect("a scene")
}

/// The scene a `type` act showed at once, before core was asked, when it showed one.
pub fn at_once(printed: &Value) -> Option<Scene> {
    serde_json::from_value(printed["at_once"].clone()).expect("a scene, or none")
}

/// The key of the field under the note with that id.
pub fn key<'a>(scene: &'a Scene, id: &str) -> Option<&'a str> {
    scene.notes.iter().find_map(|note| match note {
        Note::Text { id: own, field: Some(field), .. } if own == id => Some(field.key.as_str()),
        _ => None,
    })
}

/// What a `type` act printed: whether the field was told its words were kept, and the scene shown.
pub fn typed(printed: &Value) -> (bool, Scene) {
    (printed["kept"].as_bool().expect("an Enter that was carried"), scene(&printed["scene"]))
}

impl World {
    /// Waits for the run of a made wish to have ended: a test ends it before its folder goes.
    pub fn run_ended(&self, wish: &str) {
        wait_until("the wish's run", || self.wish(wish)["run"].is_string());
        let run = self.wish(wish)["run"].as_str().unwrap().to_string();
        wait_until("the end of the run", || !self.at("jm").join("runs").join(&run).join("run.lock").exists());
    }
}
