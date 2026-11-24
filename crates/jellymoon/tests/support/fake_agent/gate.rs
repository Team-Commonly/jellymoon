//! The made-up agent's scripts for what the gate found: streams that are not a run's, results that earn no star,
//! and harnesses that leave something going or turn out too old late.

use std::path::{Path, PathBuf};
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
    super::shape::asked(rec); // asked to shape a wish, it is no run: it answers and ends here
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
        "rules" => rules(),
        "many" | "past" | "walls" | "wall8" => beside(script),
        "crowd" | "long" | "longs" | "six" | "seven" | "eight" | "masked" | "blank" | "object" | "joined"
        | "counted" => obeyed(script),
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

/// F47. The work, and beside it a file the person's tools obey and one whose text would be more than text to
/// anything that read it as a page, a link, a terminal's colours or a command.
fn rules() -> i32 {
    edits();
    fs::write("CLAUDE.md", "rules from the run\n").unwrap();
    let odd = "<script>open('file:///')</script>\n\u{1b}[31m[tap](https://example.test)\n$(touch ran)\n";
    fs::write("0.md", odd).unwrap();
    ends_with(&success())
}

/// F47, after its gate. The work, and beside it: `many`, 230 plain files, with one file the person's tools obey
/// before them and two after, past what a list of the work holds; `past`, the same with only one such file, after;
/// `walls`, seven files of one word longer than is read of a file, which are text and cannot be shown; `wall8`,
/// eight of them.
fn beside(script: &str) -> i32 {
    edits();
    if script.starts_with("wall") {
        for n in 0..if script == "walls" { 7 } else { 8 } {
            fs::write(format!("c{n}.txt"), "x".repeat(64 * 1024 + 1)).unwrap();
        }
        return ends_with(&success());
    }
    for n in 0..230 {
        fs::write(format!("f{n:03}.txt"), "plain\n").unwrap();
    }
    let below = Path::new("zz").join(".claude");
    fs::create_dir_all(&below).unwrap();
    if script == "many" {
        fs::write("CLAUDE.md", "rules from the run\n").unwrap();
        fs::write(below.join("settings.json"), "{}\n").unwrap();
    }
    fs::write(Path::new("zz").join(".envrc"), "PATH_add bin\n").unwrap();
    ends_with(&success())
}

/// F47, after its second gate. The work, and beside it files the person's tools obey and no other: `crowd`,
/// three of them; `long`, three whose names are long, and `longs`, six; `six`, `seven` and `eight`, that many;
/// `masked`, one, whose name has the shape of a credential and is not printed. And names printed whole that a
/// person does not read: `blank`, two with a letter that draws as a blank; `object`, one with the object
/// replacement character, before one that is read; `joined`, one with a joiner between two letters, before one
/// that is read; `counted`, one that reads like the line a note counts with, after one that is read.
fn obeyed(script: &str) -> i32 {
    edits();
    let deep = Path::new("a folder with a long name of plain words").join("and a second one as long as the first");
    let deep = deep.join("and a third folder of plain words below").join("and a fourth one below all the others");
    let rules = |folder: &Path, n: usize| folder.join(format!("r{n}")).join("CLAUDE.md");
    let files: Vec<PathBuf> = match script {
        "crowd" => vec![
            Path::new(".claude").join("settings.json"),
            Path::new(".github").join("workflows").join("deploy.yml"),
            PathBuf::from("CLAUDE.md"),
        ],
        "long" => (0..3).map(|n| rules(&deep, n)).collect(),
        "longs" => (0..6).map(|n| rules(&deep, n)).collect(),
        "six" => (0..6).map(|n| rules(Path::new(""), n)).collect(),
        "seven" => (0..7).map(|n| rules(Path::new(""), n)).collect(),
        "masked" => vec![Path::new(".claude").join(format!("AK{}{}.md", "IA", "Q7".repeat(8)))],
        "blank" => vec![Path::new(".claude").join("\u{fffc}"), Path::new("\u{3164}").join("CLAUDE.md")],
        "object" => vec![Path::new(".claude").join("\u{fffc}"), PathBuf::from("CLAUDE.md")],
        "joined" => vec![Path::new(".claude").join("a\u{200d}b.md"), PathBuf::from("CLAUDE.md")],
        "counted" => vec![Path::new(".cargo").join("config.toml"), Path::new(".claude").join("+5 more")],
        _ => (0..8).map(|n| rules(Path::new(""), n)).collect(),
    };
    for file in files {
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, "rules from the run\n").unwrap();
    }
    ends_with(&success())
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
        // `idle` does neither: it changes nothing.
        "worked" | "gone" | "idle" => {
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
