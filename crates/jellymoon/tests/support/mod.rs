//! What the black-box tests share: running the built `jellymoon`, and writing session logs in the shape Claude Code 2.1
//! writes. Every line is made up here.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use std::ffi::OsStr;
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use std::{fs, thread};

use serde_json::{Value, json};
use tempfile::TempDir;

/// The tests' time zone: UTC-7 on the fixture day, 2026-03-10.
pub const ZONE: &str = "America/Los_Angeles";

pub const A: &str = "/work/alpha";
pub const B: &str = "/work/private-notes";
pub const C: &str = "/work/beta/.claude/worktrees/fix-login/web";
pub const G: &str = "/work/gamma";

pub struct Ran {
    pub stdout: String,
    pub stderr: String,
    pub code: i32,
}

/// A folder that is removed when the test ends. It is under Cargo's target folder, on the same disk as the built programs.
pub fn scratch() -> TempDir {
    tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap()
}

/// Runs `jellymoon` with only the variables given. A run still going after 20 seconds (a reader stuck on a pipe would be)
/// is stopped and fails the test. Output goes to files, so a full pipe can never be what holds it up.
pub fn jellymoon(vars: &[(&str, &OsStr)], args: &[&str]) -> Ran {
    jellymoon_fed(vars, args, "")
}

/// The same, with `input` as the program's input.
pub fn jellymoon_fed(vars: &[(&str, &OsStr)], args: &[&str], input: &str) -> Ran {
    let (mut out, mut err) = (tempfile::tempfile().unwrap(), tempfile::tempfile().unwrap());
    let mut fed = tempfile::tempfile().unwrap();
    fed.write_all(input.as_bytes()).unwrap();
    fed.rewind().unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_jellymoon"));
    cmd.args(args).env_clear().envs(vars.iter().copied()).stdin(Stdio::from(fed));
    cmd.stdout(out.try_clone().unwrap()).stderr(err.try_clone().unwrap());
    if let Some(root) = std::env::var_os("SystemRoot").filter(|_| cfg!(windows)) {
        cmd.env("SystemRoot", root); // a Windows program needs it to start
    }
    let mut child = cmd.spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > deadline {
            child.kill().unwrap();
            panic!("jellymoon {args:?} was still running after 20 seconds");
        }
        thread::sleep(Duration::from_millis(5));
    };
    let read = |file: &mut fs::File| {
        let mut text = String::new();
        file.rewind().unwrap();
        file.read_to_string(&mut text).unwrap();
        text
    };
    Ran { stdout: read(&mut out), stderr: read(&mut err), code: status.code().unwrap_or(-1) }
}

/// Puts the made-up harness in `dir` under `name`. On Linux it is a link: a program file that was just written cannot
/// be started there while a child of another test still holds it open ("text file busy"). Elsewhere it is a copy: a Mac
/// now and then kills a signed program started through a fresh link.
pub fn install(dir: &Path, name: &str) -> PathBuf {
    fs::create_dir_all(dir).unwrap();
    let (built, program) =
        (env!("CARGO_BIN_EXE_fake-claude"), dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX)));
    if !cfg!(target_os = "linux") || fs::hard_link(built, &program).is_err() {
        fs::copy(built, &program).unwrap();
    }
    program
}

/// One session log line. `origin` is the origin kind; "" leaves origin out, as a prompt sent by another program has none.
pub fn line(kind: &str, at: &str, cwd: &str, origin: &str, content: Value, extra: Value) -> String {
    let mut o = json!({"type": kind, "timestamp": at, "cwd": cwd, "sessionId": "s", "message": {"role": kind, "content": content}});
    if !origin.is_empty() {
        o["origin"] = json!({ "kind": origin });
    }
    for (key, value) in extra.as_object().unwrap() {
        o[key] = value.clone();
    }
    o.to_string()
}

pub fn said(at: &str, cwd: &str, text: &str) -> String {
    line("user", at, cwd, "human", json!(text), json!({}))
}

pub fn reply(at: &str, cwd: &str, text: &str) -> String {
    let content = json!([{"type": "thinking", "thinking": "LEAK thinking"}, {"type": "text", "text": text}]);
    line("assistant", at, cwd, "", content, json!({}))
}

/// Writes one file of the projects directory; `name` is `<project folder>/<file>`.
pub fn write_log(logs: &Path, name: &str, lines: &[String]) {
    let path = logs.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, lines.join("\n") + "\n").unwrap();
}

// Credential shapes, joined at run time so no scanner reads one in this file.
pub fn hub_key() -> String {
    format!("gh{}{}", "p_", "a1B2c3D4e5".repeat(4))
}
pub fn model_key() -> String {
    format!("sk{}{}", "-ant-", "Zx9Yw8Vu7T".repeat(3))
}
pub fn run_token() -> String {
    format!("jm{}{}", "r_", "Q7".repeat(20))
}
pub fn unknown_key() -> String {
    "Ab3/dE+9".repeat(6)
}
