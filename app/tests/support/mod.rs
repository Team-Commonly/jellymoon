//! What the F28 tests share: made-up page data, running the built `jellymoon-app`, and reading the pieces it prints.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

pub mod day;

use std::fs;
use std::io::Write;
use std::process::{Command, Output, Stdio};

use serde_json::{Value, json};

/// Page data for 2026-03-10, a Tuesday. A page with wishes was read from 5 turns in two projects.
pub fn page(wishes: Vec<Value>, clashes: Vec<Value>) -> Value {
    let read = if wishes.is_empty() {
        json!({ "turns": 0, "projects": [] })
    } else {
        json!({ "turns": 5, "projects": ["alpha", "beta"] })
    };
    json!({ "day": "2026-03-10", "wishes": wishes, "clashes": clashes, "read": read })
}

pub fn wish(id: &str, label: &str, state: &str, stars: &[&str]) -> Value {
    json!({ "id": id, "label": label, "said": format!("please: {label}"), "project": "alpha", "state": state, "stars": stars })
}

pub fn clash(a: &str, b: &str, quote_a: &str, quote_b: &str, settled: bool) -> Value {
    json!({ "a": a, "b": b, "quote_a": quote_a, "quote_b": quote_b, "settled": settled })
}

/// Runs `jellymoon-app` with nothing in its environment, `stdin` typed into it.
pub fn run(args: &[&str], stdin: Option<&str>) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_jellymoon-app"));
    cmd.args(args).env_clear().stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(root) = std::env::var_os("SystemRoot").filter(|_| cfg!(windows)) {
        cmd.env("SystemRoot", root); // a Windows program needs it to start
    }
    let mut child = cmd.spawn().unwrap();
    let mut pipe = child.stdin.take().unwrap();
    // A run that fails on its command line never reads the pipe, so a refused write is not the test's failure.
    let _ = pipe.write_all(stdin.unwrap_or_default().as_bytes());
    drop(pipe);
    child.wait_with_output().unwrap()
}

/// Runs `jellymoon-app layout` on a file holding `text`.
pub fn layout(text: &str, args: &[&str]) -> Output {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let file = dir.path().join("page.json");
    fs::write(&file, text).unwrap();
    let mut all = vec!["layout", "--page", file.to_str().unwrap()];
    all.extend(args);
    run(&all, None)
}

/// The pieces a good run prints: every line of stdout is one JSON object.
pub fn pieces(page: &Value, args: &[&str]) -> Vec<Piece> {
    let out = layout(&page.to_string(), args);
    let said = String::from_utf8(out.stderr).unwrap();
    assert!(out.status.success() && said.is_empty(), "layout {args:?} failed: {said}");
    let lines = String::from_utf8(out.stdout).unwrap();
    assert!(lines.ends_with('\n'), "the last line is not ended");
    lines.lines().map(|line| Piece(serde_json::from_str(line).expect(line))).collect()
}

/// What a refused run says on stderr: it exits 1 and prints nothing on stdout.
pub fn refused(text: &str) -> String {
    let out = layout(text, &[]);
    let said = String::from_utf8(out.stderr).unwrap();
    assert_eq!(out.status.code(), Some(1), "{text}: {said}");
    assert!(out.stdout.is_empty(), "{text}: a refused page printed something");
    said
}

/// The x, y, width and height of a piece or of one of its text runs.
pub fn rect(of: &Value) -> [f64; 4] {
    ["x", "y", "w", "h"].map(|key| of[key].as_f64().unwrap_or_else(|| panic!("no number {key} in {of}")))
}

/// One placed piece, as printed.
#[derive(Debug)]
pub struct Piece(pub Value);

impl Piece {
    pub fn kind(&self) -> &str {
        self.0["kind"].as_str().unwrap()
    }

    pub fn id(&self) -> &str {
        self.0["id"].as_str().unwrap()
    }

    pub fn rect(&self) -> [f64; 4] {
        rect(&self.0)
    }

    pub fn runs(&self) -> &[Value] {
        let runs = self.0["text"].as_array().unwrap();
        runs.iter().for_each(|run| assert!(rect(run).iter().all(|n| n.is_finite()) && run["role"].is_string()));
        runs
    }

    pub fn roles(&self) -> Vec<String> {
        self.runs().iter().map(|run| run["role"].as_str().unwrap().to_string()).collect()
    }

    /// The piece's text runs, joined with spaces.
    pub fn said(&self) -> String {
        self.runs().iter().map(|run| run["text"].as_str().unwrap()).collect::<Vec<_>>().join(" ")
    }
}
