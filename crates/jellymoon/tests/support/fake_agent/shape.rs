//! The made-up agent asked to shape a wish (F91): a command line with `--add-dir` and `--output-format json` is
//! the shaping call and no run. It counts no start. What it was given goes to `shaping` in the test's `rec`, and
//! it answers from the file `shape` beside its program: a first line that says how (`says`, `exit-1` or
//! `forever`), then what it prints, to the last byte. With no such file the wish is clear.

use std::io::Read;
use std::path::Path;
use std::time::Duration;
use std::{env, fs, io, process, thread};

use super::{held, out};

const CLEAR: &str = r#"says
{"type":"result","is_error":false,"result":"{\"clear\":true,\"wish\":\"\",\"done_when\":\"\",\"not_touching\":\"\",\"why\":\"\",\"ask\":\"\"}"}"#;

/// Answers and ends when this is a shaping call; returns when it is not.
pub fn asked(rec: &Path) {
    let args: Vec<String> = env::args().skip(1).collect();
    let json = args.windows(2).any(|pair| pair[0] == "--output-format" && pair[1] == "json");
    if !json || !args.iter().any(|arg| arg == "--add-dir") {
        return;
    }
    let at = rec.join("shaping");
    fs::create_dir_all(&at).unwrap();
    for (n, arg) in args.iter().enumerate() {
        fs::write(at.join(format!("arg{n}")), arg).unwrap();
    }
    let mut names: Vec<String> = env::vars_os().map(|(name, _)| name.to_string_lossy().into_owned()).collect();
    names.sort();
    fs::write(at.join("env"), names.join("\n")).unwrap();
    fs::write(at.join("cwd"), env::current_dir().unwrap().to_string_lossy().as_bytes()).unwrap();
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).unwrap();
    fs::write(at.join("stdin"), input).unwrap();
    let script = fs::read_to_string(env::current_exe().unwrap().with_file_name("shape"));
    let script = script.unwrap_or_else(|_| CLEAR.to_string());
    let (how, says) = script.split_once('\n').unwrap_or((&script, ""));
    if how.trim() == "forever" {
        let _held = held(rec);
        thread::sleep(Duration::from_secs(600));
    }
    out(says.as_bytes());
    process::exit(i32::from(how.trim() == "exit-1"));
}
