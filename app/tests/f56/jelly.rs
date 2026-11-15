//! What the F56 tests share: lines of `jellymoon hear`, typed here as core prints them, the built `jellymoon-app
//! layout --jelly` run on one, and what it printed. Nothing here opens a window or starts another program.

use std::path::Path;
use std::process::{Command, Output, Stdio};

use serde_json::{Value, json};

use crate::support::{Piece, rect, run};

pub const SCREEN: &str = "1440x900";
pub const TAKE_BACK: &str = "press the key again to take it back";
pub const ASKS: &str = "a run asks, and waits for your answer on the page";
pub const WAITS: &str = "a run's work waits for you on the page";
pub const NO_LINE: &str = "jellymoon-app: the input is not one line of `jellymoon hear`\n";

fn line(state: &str, says: &str, words: &str, project: &str, left: u64) -> Value {
    json!({"state": state, "says": says, "words": words, "project": project, "left": left})
}

fn with(mut line: Value, name: &str, value: Value) -> Value {
    line[name] = value;
    line
}

pub fn quiet() -> Value {
    line("quiet", "", "", "", 0)
}

pub fn listening(words: &str) -> Value {
    line("listening", "", words, "", 0)
}

pub fn heard(words: &str, project: &str, left: u64) -> Value {
    line("heard", TAKE_BACK, words, project, left)
}

pub fn working(going: usize) -> Value {
    with(line("working", "", "", "", 0), "going", json!(going))
}

pub fn asks(question: &str) -> Value {
    with(line("asks", ASKS, "", "", 0), "question", json!(question))
}

pub fn waits() -> Value {
    line("waits", WAITS, "", "", 0)
}

pub fn cannot(says: &str) -> Value {
    line("cannot", says, "", "", 0)
}

/// `jellymoon-app layout --jelly - --screen <screen>` with `input` typed into it and `more` after it, with no
/// variable but `vars`.
pub fn app(input: &str, screen: &str, more: &[&str], vars: &[(&str, &Path)]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_jellymoon-app"));
    cmd.args(["layout", "--jelly", "-", "--screen", screen]).args(more).env_clear();
    cmd.envs(vars.iter().copied()).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(root) = std::env::var_os("SystemRoot").filter(|_| cfg!(windows)) {
        cmd.env("SystemRoot", root); // a Windows program needs it to start
    }
    let mut child = cmd.spawn().unwrap();
    let _ = std::io::Write::write_all(&mut child.stdin.take().unwrap(), input.as_bytes());
    child.wait_with_output().unwrap()
}

/// What a run printed: its exit code, its stdout and its stderr.
pub fn said(out: &Output) -> (Option<i32>, String, String) {
    let text = |bytes: &[u8]| String::from_utf8(bytes.to_vec()).unwrap();
    (out.status.code(), text(&out.stdout), text(&out.stderr))
}

/// The pieces a good run printed: every line of stdout is one JSON object, and stderr is empty.
pub fn placed(out: &Output) -> Drawn {
    let (code, lines, err) = said(out);
    assert_eq!((code, err.as_str()), (Some(0), ""), "{lines}");
    assert!(lines.ends_with('\n'), "the last line is not ended");
    Drawn(lines.lines().map(|line| Piece(serde_json::from_str(line).expect(line))).collect())
}

/// What is drawn of one line of `hear` on a screen of 1440 by 900.
pub fn drawn(line: &Value) -> Drawn {
    drawn_on(line, SCREEN)
}

pub fn drawn_on(line: &Value, screen: &str) -> Drawn {
    placed(&run(&["layout", "--jelly", "-", "--screen", screen], Some(&format!("{line}\n"))))
}

/// The lines `layout --jelly` printed, in their order.
pub struct Drawn(pub Vec<Piece>);

impl Drawn {
    pub fn of(&self, kind: &str) -> Vec<&Piece> {
        self.0.iter().filter(|piece| piece.kind() == kind).collect()
    }

    /// Each line's kind and the window it is of.
    pub fn kinds(&self) -> Vec<(&str, &str)> {
        self.0.iter().map(|piece| (piece.kind(), piece.id())).collect()
    }

    /// Where the window `id` stands on the screen. None when it is not opened.
    pub fn window(&self, id: &str) -> Option<[f64; 4]> {
        let mut windows = self.of("window").into_iter().filter(|window| window.id() == id);
        let found = windows.next().map(Piece::rect);
        assert!(windows.next().is_none(), "two windows are named {id}");
        found
    }

    /// The jellyfish itself: its state and its mood.
    pub fn jellyfish(&self) -> (&str, &str) {
        let all = self.of("character");
        assert_eq!(all.len(), 1, "one jellyfish");
        (all[0].0["state"].as_str().unwrap(), all[0].0["mood"].as_str().unwrap())
    }

    /// The bubble's text runs, in their order.
    pub fn runs(&self) -> Vec<&Value> {
        self.of("bubble").into_iter().flat_map(|bubble| bubble.runs()).collect()
    }

    /// The bubble's texts of one role, its lines joined by spaces.
    pub fn text(&self, role: &str) -> String {
        let of_role = self.runs().into_iter().filter(|run| run["role"] == role);
        of_role.map(|run| run["text"].as_str().unwrap()).collect::<Vec<_>>().join(" ")
    }

    /// The roles of the bubble's runs, each once, in their order.
    pub fn roles(&self) -> Vec<&str> {
        let mut roles: Vec<&str> = self.runs().into_iter().map(|run| run["role"].as_str().unwrap()).collect();
        roles.dedup();
        roles
    }

    /// How full the ring is. None when no ring is drawn.
    pub fn ring(&self) -> Option<f64> {
        self.of("ring").first().map(|ring| ring.0["ring"].as_f64().unwrap())
    }

    /// Every line but its text: what a text may never change.
    pub fn bare(&self) -> Vec<(String, String, [f64; 4])> {
        self.0.iter().map(|piece| (piece.kind().to_string(), piece.id().to_string(), piece.rect())).collect()
    }
}

/// Whether `inner` lies in `outer`, both as x, y, width and height.
pub fn inside(inner: [f64; 4], outer: [f64; 4]) -> bool {
    let [x, y, w, h] = inner;
    x >= outer[0] && y >= outer[1] && x + w <= outer[0] + outer[2] + 0.011 && y + h <= outer[1] + outer[3] + 0.011
}

/// A run's own box.
pub fn at(run: &Value) -> [f64; 4] {
    rect(run)
}
