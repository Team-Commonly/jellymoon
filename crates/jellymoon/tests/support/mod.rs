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

/// Says why a test left a part out, where a person running the tests reads it: straight to stderr, which the test
/// runner does not keep back as it does what a passing test prints.
pub fn skipped(why: &str) {
    let test = thread::current().name().unwrap_or("a test").to_string();
    let _ = writeln!(std::io::stderr(), "skipped in {test}: {why}");
}

/// Runs `jellymoon` with only the variables given. A run still going after 20 seconds (a reader stuck on a pipe would be)
/// is stopped and fails the test. Output goes to files, so a full pipe can never be what holds it up.
pub fn jellymoon(vars: &[(&str, &OsStr)], args: &[&str]) -> Ran {
    started(Path::new("."), vars, args, "")
}

/// The same, with `input` as the program's input.
pub fn jellymoon_fed(vars: &[(&str, &OsStr)], args: &[&str], input: &str) -> Ran {
    started(Path::new("."), vars, args, input)
}

/// The same, started in `folder`.
pub fn jellymoon_in(folder: &Path, vars: &[(&str, &OsStr)], args: &[&str]) -> Ran {
    started(folder, vars, args, "")
}

/// Started in `folder`, with `input` as the program's input.
pub fn started(folder: &Path, vars: &[(&str, &OsStr)], args: &[&str], input: &str) -> Ran {
    let (mut out, mut err) = (tempfile::tempfile().unwrap(), tempfile::tempfile().unwrap());
    let mut fed = tempfile::tempfile().unwrap();
    fed.write_all(input.as_bytes()).unwrap();
    fed.rewind().unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_jellymoon"));
    cmd.args(args).current_dir(folder).env_clear().envs(vars.iter().copied()).stdin(Stdio::from(fed));
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

/// Puts the made-up harness in `dir` under `name`.
pub fn install(dir: &Path, name: &str) -> PathBuf {
    fs::create_dir_all(dir).unwrap();
    let program = dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    plant(env!("CARGO_BIN_EXE_fake-claude"), &program);
    program
}

/// Puts a built test program at `program`. On Linux it is a link: a program file that was just written cannot be
/// started there while a child of another test still holds it open ("text file busy"). Elsewhere it is a copy: a Mac
/// now and then kills a signed program started through a fresh link.
pub fn plant(built: &str, program: &Path) {
    if !cfg!(target_os = "linux") || fs::hard_link(built, program).is_err() {
        fs::copy(built, program).unwrap();
    }
    // A copy takes the mode the build left, which the group can write to where files are made so (a umask of 002).
    #[cfg(unix)]
    fs::set_permissions(program, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
}

/// Every folder and file under a folder, in name order, each file with its bytes: what a refusal must leave as it was.
pub fn tree(folder: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut paths: Vec<PathBuf> = fs::read_dir(folder).unwrap().map(|entry| entry.unwrap().path()).collect();
    paths.sort();
    let mut held = Vec::new();
    for path in paths {
        let inside = if path.is_dir() { tree(&path) } else { Vec::new() };
        held.push((path.clone(), if path.is_file() { fs::read(&path).unwrap() } else { Vec::new() }));
        held.extend(inside);
    }
    held
}

/// The first folder above `path`, links followed, that its group or everyone can write to and that is not sticky.
/// Core takes no installed command under one, so there a test of a command that is taken says why and leaves that
/// half out (Cargo's target folder under a umask of 002). None where the system has no modes.
pub fn open_above(path: &Path) -> Option<PathBuf> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let open = |folder: &&Path| {
            fs::metadata(folder).is_ok_and(|meta| meta.mode() & 0o022 != 0 && meta.mode() & 0o1000 == 0)
        };
        let found = fs::canonicalize(path).unwrap().ancestors().skip(1).find(open).map(Path::to_path_buf);
        if let Some(open) = &found {
            eprintln!("left out: the half where the command is taken, since others can write to {}", open.display());
        }
        found
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        None
    }
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
/// The first line of a private key, with `gap` where the space after its first word goes.
pub fn key_header(gap: &str) -> String {
    format!("{0}BEGIN{gap}RSA PRIVATE {1}{0}", "-".repeat(5), "KEY")
}
/// A map key's shape of letters only, so no other shape finds it. `fill` is five letters that tell two apart.
pub fn map_key(fill: &str) -> String {
    format!("AI{}{}", "za", fill.repeat(7))
}
/// A payment key's shape, short and of letters only: straight after another key, only its own shape finds it.
pub fn pay_key() -> String {
    format!("sk{}{}", "_live_", "qRsTuVwX".repeat(2))
}
/// A cloud key id's shape. It is one only where no capital or digit follows it.
pub fn key_id() -> String {
    format!("AK{}{}", "IA", "Q7".repeat(8))
}
/// Whether any 8 characters in a row of `secret` can be read in `text`.
pub fn shows(text: &str, secret: &str) -> bool {
    secret.as_bytes().windows(8).any(|run| text.contains(std::str::from_utf8(run).unwrap()))
}
/// Words that cleaning once leaves changed by cleaning again, each with what is kept of it and what was planted in
/// it: a private key's first line with a tab in it; two keys and three with nothing between; two keys where a cut
/// made before the second was found would show half of it; seven keys and eight, which eight cleanings are done
/// with; nine, which they are not, so it has no words to keep.
pub fn restless() -> Vec<(String, String, Vec<String>)> {
    let (one, two, pay) = (map_key("kLmNo"), map_key("pQrSt"), pay_key());
    let (words, many, marks) = ("word ".repeat(33), map_key("dEfGh"), "[redacted]".repeat(8));
    let row = |keys: usize| format!("use {} here", many.repeat(keys));
    vec![
        (format!("paste {} and go on", key_header("\t")), "paste [redacted]".into(), vec![key_header(" ")]),
        (format!("use {one}{pay} here"), format!("use {} here", &marks[..20]), vec![one.clone(), pay.clone()]),
        (
            format!("use {one}{two}{pay} here"),
            format!("use {} here", &marks[..30]),
            vec![one.clone(), two.clone(), pay],
        ),
        (format!("{words}{one}{two}"), format!("{words}{}", &marks[..20]), vec![one, two]),
        (row(7), format!("use {} here", &marks[..70]), vec![many.clone()]),
        (row(8), format!("use {marks} here"), vec![many.clone()]),
        (row(9), String::new(), vec![many]),
    ]
}
pub fn unknown_key() -> String {
    "Ab3/dE+9".repeat(6)
}
