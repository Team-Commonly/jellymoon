//! The made-up agent's scripts for what the gate found: streams that are not a run's, results that earn no star,
//! and harnesses that leave something going or turn out too old late.

use std::path::Path;
use std::time::Duration;
use std::{env, fs, process, thread};

use super::{
    MARKER, MODE, TOOLS, edits, ends, ends_with, first, garbage, held, init, line, litter, result, starts, success,
    work,
};

/// Every made-up agent, and the program one leaves going, is minded by the test that started it. It holds the
/// system's lock on a file of its own in `rec` while it lives, and ends when the file `over` is left there: the
/// test's world leaves it as it is dropped, and waits for these locks (`tests/f39/guard.rs`). So a test that
/// ended, by a panic too, leaves no agent, on a system with no signals as well.
pub fn minded(rec: &Path) {
    let live = fs::File::create(rec.join(format!("live-{}", process::id()))).unwrap();
    live.lock().unwrap();
    let over = rec.join("over");
    thread::spawn(move || {
        let _live = live;
        while !over.exists() {
            thread::sleep(Duration::from_millis(20));
        }
        process::exit(3)
    });
}

/// The scripts of the gate review's tests (`tests/f39/stream.rs`, `tests/f39/safe.rs`).
pub fn play(script: &str, number: usize, rec: &Path) -> i32 {
    match script {
        // What may not come after a success: 0 a line that is not the stream's, 1 one over the line cap, 2 a second
        // success, 3 a result that is not one, 4 the list. 5 is a success after a result that was not one.
        "after" => {
            edits();
            let (fine, not) = (success(), result("error_during_execution", "true", 0, 0));
            line(if number == 5 { &not } else { &fine });
            match number {
                0 => line(&format!("{MARKER} is not JSON")),
                1 => line(&"j".repeat(1 << 20)),
                2 | 5 => line(&fine),
                3 => line(&not),
                _ => line(r#"["result","success",false,[]]"#),
            }
            0
        }
        // A result that does not say whether it is an error: 0 leaves the part out, 1 has it as nothing.
        "unsaid" => {
            let said = if number == 0 { "" } else { r#""is_error":null,"# };
            ends(&format!(r#"{{"type":"result","subtype":"success",{said}"permission_denials":[]}}"#), 0)
        }
        // Before the result, a result written as a list, which is no line of the stream either.
        "garbage" => {
            starts();
            garbage();
            work();
            line(r#"["result","error_during_execution",true,[]]"#);
            ends_with(&success())
        }
        "first" => first(number, rec),
        "litter" => litter(rec, number),
        // A harness that did its work and then fails in the words of one too old.
        "worked-old" => {
            edits();
            eprintln!("error: unknown option '--restricted'");
            1
        }
        other => round_2(other, number, rec),
    }
}

const OLD: &str = "error: unknown option '--restricted'";

/// `holds`: the measured start and the work, then a program of its own left going in another process group with
/// this one's output open, then the words of a harness too old and a failure. 1: with no line printed at all. 2: it
/// does not end by itself after that, and holds the lock while it lives.
fn holds(rec: &Path, number: usize) -> i32 {
    if number != 1 {
        edits();
    }
    let mut left = process::Command::new(env::current_exe().unwrap());
    left.arg("litters").arg(rec).stdin(process::Stdio::null()).stderr(process::Stdio::null());
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut left, 0);
    drop(left.spawn().unwrap());
    while !rec.join("litter.txt").exists() {
        thread::sleep(Duration::from_millis(10));
    }
    if number == 2 {
        return lingers(rec, 0);
    }
    eprintln!("{OLD}");
    1
}

/// The end of `lingers`: a result that says success and counts `denials`, when there are any, and then a minute's
/// sleep with the lock held, which only a stop ends early.
fn lingers(rec: &Path, denials: usize) -> i32 {
    if denials > 0 {
        line(&result("success", "false", denials, 0));
    }
    let _held = held(rec);
    thread::sleep(Duration::from_secs(60));
    1
}

/// The scripts of the gate's second round (`tests/f39/stream.rs`, `tests/f39/refused.rs`), and F40's.
fn round_2(script: &str, number: usize, rec: &Path) -> i32 {
    match script {
        // Too old, but only once the test leaves `go`: it holds the lock while it lives.
        "old-late" => {
            let _held = held(rec);
            for _ in 0..3000 {
                if rec.join("go").exists() {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }
            eprintln!("{OLD}");
            1
        }
        "holds" => holds(rec, number),
        // F48. Both report a success only once the test leaves `go`, half a minute at most, and hold the lock
        // until then. `worked` has done its work by then. `gone` removes the folder it was started in, from outside it.
        "worked" | "gone" => {
            if script == "worked" {
                edits()
            } else {
                starts()
            }
            let _held = held(rec);
            for _ in 0..3000 {
                if rec.join("go").exists() {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }
            if script == "gone" {
                let copy = env::current_dir().unwrap();
                env::set_current_dir(rec).unwrap();
                fs::remove_dir_all(copy).unwrap();
            }
            ends_with(&success())
        }
        // F40. The measured start and the work, and then it does not end by itself.
        "lingers" => {
            edits();
            lingers(rec, number)
        }
        // The work, and a result that is no success and counts `number` denials.
        "denied-failed" => ends(&result("error_during_execution", "true", number, 0), 0),
        // The words of one too old, and no failure.
        "old-0" => {
            eprintln!("{OLD}");
            0
        }
        // After the measured start a second one: 0 with one tool more, 1 with a server, 2 in another mode. 3 is
        // the measured one again.
        "again" => {
            starts();
            match number {
                0 => init(r#"["Bash","Edit","Glob","Grep","Read","Write"]"#, "[]", MODE),
                1 => init(TOOLS, r#"[{"name":"files","status":"connected"}]"#, MODE),
                2 => init(TOOLS, "[]", r#""bypassPermissions""#),
                _ => init(TOOLS, "[]", MODE),
            }
            work();
            ends_with(&success())
        }
        // Two results: the first counts one denial, the second three.
        "denied-twice" => {
            edits();
            line(&result("success", "false", 1, 0));
            ends_with(&result("success", "false", 3, 0))
        }
        other => panic!("no script {other}"),
    }
}
