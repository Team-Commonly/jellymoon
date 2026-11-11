//! What the F34 tests share: one made-up day in session logs, a jellymoon home, and the made-up harness of F27's
//! tests on PATH as `claude`, which records what it was given and prints the reply the test chose.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::{env, fs};

use serde_json::json;
use tempfile::TempDir;

use super::Piece;

pub const DAY: &str = "2026-03-10";
pub const FIRST: &str = "add a retry to the uploader";
pub const SECOND: &str = "actually never retry uploads, fail fast";

/// One test's logs, jellymoon home and made-up harness. `rec` is the folder the harness is in, where it leaves what
/// it saw.
pub struct World {
    pub dir: TempDir,
    pub home: PathBuf,
    pub rec: PathBuf,
    pub vars: Vec<(&'static str, OsString)>,
}

/// One line of a session log, as Claude Code writes what a person typed.
pub fn said(at: &str, cwd: &str, text: &str) -> String {
    json!({"type": "user", "timestamp": at, "cwd": cwd, "sessionId": "s", "origin": {"kind": "human"},
        "message": {"role": "user", "content": text}})
    .to_string()
}

pub fn write_log(world: &World, name: &str, lines: &[String]) {
    let path = world.dir.path().join("logs").join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, lines.join("\n") + "\n").unwrap();
}

/// The harness's reply for a model that answered with `text`.
pub fn result(text: &str) -> String {
    json!({"type": "result", "is_error": false, "result": text}).to_string()
}

/// A reading of the day: two wishes in the person's words and the clash between them.
pub fn reading() -> String {
    json!({
        "wishes": [
            {"id": "a", "label": "Retry uploads", "said": FIRST, "project": "alpha", "state": "let_go", "stars": []},
            {"id": "b", "label": "Fail fast", "said": SECOND, "project": "alpha", "state": "done", "stars": ["no retry left"]},
        ],
        "clashes": [{"a": "a", "b": "b", "quote_a": FIRST, "quote_b": "never retry uploads", "settled": true}],
    })
    .to_string()
}

/// A day of two things typed in `alpha` and one in `private-notes`, and a harness on PATH that answers [`reading`].
pub fn world() -> World {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let at = |name: &str| dir.path().join(name);
    let bin = at("bin");
    fs::create_dir_all(&bin).unwrap();
    let (built, program) = (env!("CARGO_BIN_EXE_f34-claude"), bin.join(format!("claude{}", env::consts::EXE_SUFFIX)));
    // A link on Linux and a copy elsewhere, for the reasons F27's tests give.
    if !cfg!(target_os = "linux") || fs::hard_link(built, &program).is_err() {
        fs::copy(built, &program).unwrap();
    }
    fs::create_dir(at("user")).unwrap();
    let vars = vec![
        ("TZ", "America/Los_Angeles".into()),
        ("HOME", at("user").into()),
        ("USERPROFILE", at("user").into()),
        ("JELLYMOON_HOME", at("jm").into()),
        ("PATH", bin.clone().into()),
    ];
    let world = World { home: at("jm"), rec: bin, vars, dir };
    let lines = [
        said("2026-03-10T16:00:00.000Z", "/work/alpha", FIRST),
        said("2026-03-10T17:00:00.000Z", "/work/alpha", SECOND),
    ];
    write_log(&world, "-work-alpha/s1.jsonl", &lines);
    write_log(
        &world,
        "-work-private-notes/s2.jsonl",
        &[said("2026-03-10T18:00:00.000Z", "/work/private-notes", "dear diary")],
    );
    answer(&world, &result(&reading()), 0);
    world
}

pub fn answer(world: &World, reply: &str, exit: i32) {
    fs::write(world.rec.join("reply"), reply).unwrap();
    fs::write(world.rec.join("exit"), exit.to_string()).unwrap();
}

/// Runs `jellymoon-app layout` on the day with the world's logs, and the world's variables with `more` over them.
pub fn run(world: &World, args: &[&str], more: &[(&str, &str)]) -> Output {
    let logs = world.dir.path().join("logs");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_jellymoon-app"));
    cmd.args(["layout", "--date", DAY, "--logs"]).arg(logs).args(args);
    cmd.env_clear().envs(world.vars.iter().map(|(name, value)| (name, value))).stdin(Stdio::null());
    cmd.envs(more.iter().copied());
    if let Some(root) = env::var_os("SystemRoot").filter(|_| cfg!(windows)) {
        cmd.env("SystemRoot", root);
    }
    cmd.output().unwrap()
}

/// What [`run`] placed. It must exit 0 and say nothing on stderr.
pub fn shown(world: &World, args: &[&str]) -> Vec<Piece> {
    let out = run(world, args, &[]);
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success() && said.is_empty(), "layout {args:?}: {said}");
    let lines = String::from_utf8(out.stdout).unwrap();
    lines.lines().map(|line| Piece(serde_json::from_str(line).expect(line))).collect()
}

pub fn notes(placed: &[Piece]) -> Vec<String> {
    placed.iter().filter(|piece| piece.kind() == "note").map(Piece::said).collect()
}

pub fn labels(placed: &[Piece]) -> Vec<String> {
    placed.iter().filter(|piece| piece.kind() == "label").map(Piece::said).collect()
}

/// Whether the harness was called since the last look. Its record of the call is removed.
pub fn called(world: &World) -> bool {
    fs::remove_file(world.rec.join("stdin")).is_ok()
}

pub fn kept(world: &World) -> PathBuf {
    world.home.join("pages").join(format!("{DAY}.json"))
}

pub fn names(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> =
        fs::read_dir(folder).unwrap().map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    names
}
