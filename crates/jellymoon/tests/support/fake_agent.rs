//! A made-up agent for the black-box tests of a run (F39): what `jellymoon run` starts in place of Claude Code. The
//! file `play` beside its own program file holds three lines the test wrote: the script to play, the folder to leave
//! its record in, and a number some scripts take. The record is one line in `starts`, a file per argument, the names
//! of its variables, the folder it ran in and its whole input. Then it plays the script: a stream shaped like the one
//! measured from Claude Code 2.1.322 (`docs/adr/0014-a-run-is-started-headless.md`, "Measured"), every line typed
//! here. Whatever it prints as its own words holds [`MARKER`], which no file and no output of Jellymoon may ever hold.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;
use std::{env, fs, io, process, thread};

/// What the agent says, in every text of its own.
const MARKER: &str = "zebra-quartz-said-by-the-agent";

/// Prints to the stream. A stream nobody reads any more ends this program.
fn out(bytes: &[u8]) {
    let mut stdout = io::stdout().lock();
    if stdout.write_all(bytes).and_then(|()| stdout.flush()).is_err() {
        process::exit(0);
    }
}

fn line(text: &str) {
    out(format!("{text}\n").as_bytes());
}

/// The stream's last line, with `pad` more bytes in a field of its own.
fn result(subtype: &str, is_error: &str, denials: usize, pad: usize) -> String {
    let denial = r#"{"tool_name":"Write","tool_use_id":"t1","tool_input":{"file_path":"/elsewhere/x"}}"#;
    let denials = vec![denial; denials].join(",");
    format!(
        r#"{{"type":"result","subtype":"{subtype}","is_error":{is_error},"num_turns":2,"result":"{MARKER} all done","permission_denials":[{denials}],"pad":"{}"}}"#,
        "x".repeat(pad)
    )
}

fn success() -> String {
    result("success", "false", 0, 0)
}

fn starts() {
    line(
        r#"{"type":"system","subtype":"init","tools":["Edit","Glob","Grep","Read","Write"],"mcp_servers":[],"permissionMode":"acceptEdits"}"#,
    );
    line(&format!(
        r#"{{"type":"assistant","message":{{"role":"assistant","content":[{{"type":"text","text":"{MARKER} I will edit a.txt"}}]}}}}"#
    ));
}

/// The work: one file edited, one new, one removed, in the folder it was started in.
fn edits() {
    starts();
    work();
}

fn work() {
    fs::write("a.txt", "edited by the run\n").unwrap();
    fs::write("n.txt", "new from the run\n").unwrap();
    fs::remove_file("b.txt").unwrap();
}

/// Lines that are not the stream's: each is counted and skipped.
fn garbage() {
    line(&format!("{MARKER} is not JSON"));
    line("[1,2,3]");
    line(r#"{"no_type":true}"#);
    line(r#"{"type":7}"#);
    line("");
    out(b"\xff\xfe\x00 not text\n");
    line(r#"{"type":"result","subtype":"success","is_error":false,"permission_denials":"none"}"#);
}

/// Junk lines, then the result, `total` bytes in all.
fn fill(total: usize) {
    let (last, chunk) = (success() + "\n", 64 * 1024);
    let mut left = total - last.len();
    while left > 0 {
        let size = if left >= 2 * chunk { chunk } else { left };
        out(format!("{}\n", "j".repeat(size - 1)).as_bytes());
        left -= size;
    }
    out(last.as_bytes());
}

/// Takes the system's lock on `held` and leaves `started`: a test that can take the lock knows this program is gone.
fn held(rec: &Path) -> fs::File {
    let held = fs::File::create(rec.join("held")).unwrap();
    held.lock().unwrap();
    fs::write(rec.join("started"), "").unwrap();
    held
}

/// The work, then the stream's last line, then the exit code.
fn ends(last: &str, code: i32) -> i32 {
    edits();
    line(last);
    code
}

/// The stream's last line after work already done, and exit 0.
fn ends_with(last: &str) -> i32 {
    line(last);
    0
}

/// `forever` sleeps a minute and fails. `slow` waits for the test to leave `go`, half a minute at most, then does
/// the work. Both hold the lock while they live.
fn waits(slow: bool, rec: &Path) -> i32 {
    let _held = held(rec);
    if !slow {
        thread::sleep(Duration::from_secs(60));
        return 1;
    }
    for _ in 0..3000 {
        if rec.join("go").exists() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    ends(&success(), 0)
}

/// `scribble` keeps writing a file in the folder it was started in and prints nothing, so nothing tells it that
/// its reader is gone. It ends when the test leaves `stop`, or after half a minute.
fn scribbles(rec: &Path) -> i32 {
    let _held = held(rec);
    let mut file = fs::OpenOptions::new().append(true).create(true).open("scribble.txt").unwrap();
    for _ in 0..3000 {
        if rec.join("stop").exists() {
            break;
        }
        file.write_all(b"more\n").unwrap();
        thread::sleep(Duration::from_millis(10));
    }
    0
}

/// `deaf` starts this program again as one that the stop signal does not reach: the system keeps a signal ignored
/// across the start of a program, and `perl` can ask for that. Where that cannot be done it leaves `cannot`.
fn deafen(rec: &Path) -> ! {
    #[cfg(unix)]
    if Path::new("/usr/bin/perl").is_file() {
        let ignore = "$SIG{TERM} = 'IGNORE'; exec @ARGV; exit 9";
        let mut perl = process::Command::new("/usr/bin/perl");
        perl.args(["-e", ignore]).arg(env::current_exe().unwrap()).arg("now");
        let failed = std::os::unix::process::CommandExt::exec(&mut perl);
        eprintln!("perl could not be started: {failed}");
    }
    fs::write(rec.join("cannot"), "").unwrap();
    process::exit(1)
}

fn record(rec: &Path) {
    let mut starts = fs::OpenOptions::new().append(true).create(true).open(rec.join("starts")).unwrap();
    starts.write_all(b"started\n").unwrap();
    for (n, arg) in env::args().skip(1).enumerate() {
        fs::write(rec.join(format!("arg{n}")), arg).unwrap();
    }
    let mut names: Vec<String> = env::vars_os().map(|(name, _)| name.to_string_lossy().into_owned()).collect();
    names.sort();
    fs::write(rec.join("env"), names.join("\n")).unwrap();
    fs::write(rec.join("cwd"), env::current_dir().unwrap().to_string_lossy().as_bytes()).unwrap();
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).unwrap();
    fs::write(rec.join("stdin"), input).unwrap();
}

fn main() {
    let play = fs::read_to_string(env::current_exe().unwrap().with_file_name("play")).unwrap();
    let play: Vec<&str> = play.lines().collect();
    let (script, rec, number) = (play[0], Path::new(play[1]), play[2].parse::<usize>().unwrap());
    if script == "deaf" && env::args().nth(1).as_deref() != Some("now") {
        deafen(rec);
    }
    record(rec);
    let code = match script {
        "edits" => ends(&success(), 0),
        // The smoke chain's wish, done: one new file that holds the one word.
        "hello" => {
            starts();
            fs::write("HELLO.txt", "Hello\n").unwrap();
            ends_with(&success())
        }
        "nothing" => {
            starts();
            line(&success());
            0
        }
        "is-error" => ends(&result("success", "true", 0, 0), 0),
        "not-success" => ends(&result("error_during_execution", "false", 0, 0), 0),
        "error-as-text" => ends(&result("success", "\"false\"", 0, 0), 0),
        "exit-3" => ends(&success(), 3),
        "no-result" => {
            edits();
            eprintln!("{MARKER} gave up");
            1
        }
        "denials" => {
            edits();
            line(r#"{"type":"system","subtype":"permission_denied","tool_name":"Write"}"#);
            ends_with(&result("success", "false", number, 0))
        }
        // The last line is a result written as a list, which is no line of the stream either.
        "garbage" => {
            garbage();
            ends(&success(), 0);
            ends_with(r#"["result","error_during_execution",true,[]]"#)
        }
        // One junk line of `number` bytes with its line break, then the result.
        "long-line" => {
            edits();
            line(&"j".repeat(number - 1));
            ends_with(&success())
        }
        // The result itself, `number` bytes with its line break.
        "padded-result" => ends(&result("success", "false", 0, number - success().len() - 1), 0),
        // The work with no line of its own, so the stream is `number` bytes and not one more.
        "fill" => {
            work();
            fill(number);
            0
        }
        "old" => {
            eprintln!("error: unknown option '--restricted'");
            1
        }
        "forever" | "deaf" | "slow" => waits(script == "slow", rec),
        "scribble" => scribbles(rec),
        other => panic!("no script {other}"),
    };
    process::exit(code);
}
