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

#[path = "fake_agent/gate.rs"]
mod gate;

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

const TOOLS: &str = r#"["Edit","Glob","Grep","Read","Write"]"#;
const MODE: &str = r#""acceptEdits""#;

/// The stream's first line as it was measured, but for the three parts given, each as JSON.
fn init(tools: &str, servers: &str, mode: &str) {
    let parts = format!(r#""tools":{tools},"mcp_servers":{servers},"permissionMode":{mode}"#);
    line(&format!(r#"{{"type":"system","subtype":"init","cwd":"{MARKER}",{parts}}}"#));
}

fn says() {
    line(&format!(
        r#"{{"type":"assistant","message":{{"role":"assistant","content":[{{"type":"text","text":"{MARKER} I will edit a.txt"}}]}}}}"#
    ));
}

fn starts() {
    init(TOOLS, "[]", MODE);
    says();
}

/// `first`: a stream whose start is not the measured one, by `number`; the measured start then comes too late, and
/// the agent never ends by itself (`waits`). 13 and 14 are two such that end at once, with the work and a success.
/// From 15 on the start is fine: the work follows, and a success.
fn first(number: usize, rec: &Path) -> i32 {
    let hook = r#"{"type":"system","subtype":"hook_started"}"#;
    let less = |part: &str| line(&format!(r#"{{"type":"system","subtype":"init",{part}}}"#));
    match number {
        0 | 13 => init(r#"["Bash","Edit","Glob","Grep","Read","Write"]"#, "[]", MODE),
        1 => init(TOOLS, r#"[{"name":"files","status":"connected"}]"#, MODE),
        2 => init(TOOLS, "[]", r#""bypassPermissions""#),
        3 => init(r#"["Edit","Glob","Grep","Read"]"#, "[]", MODE),
        4 => less(&format!(r#""tools":{TOOLS},"mcp_servers":[]"#)),
        5 => less(&format!(r#""tools":{TOOLS},"permissionMode":{MODE}"#)),
        6 => less(&format!(r#""mcp_servers":[],"permissionMode":{MODE}"#)),
        7 => says(),
        8 => line(&format!("{MARKER} is not JSON")),
        9 => line(&result("error_during_execution", "true", 0, 0)),
        10 => init(r#"["Edit","Glob","Grep","Read","Write","Write"]"#, "[]", MODE),
        11 => {
            line(hook);
            init(TOOLS, "[]", r#""default""#);
        }
        12 => line(&"j".repeat(1 << 20)),
        14 => {}
        // Fine: a line of the harness's own before it, the tools in another order, and parts nobody measured.
        _ => {
            line(hook);
            init(r#"["Write","Read","Grep","Glob","Edit"],"model":"m","slash_commands":["x"]"#, "[]", MODE);
        }
    }
    if number < 13 {
        starts();
        return waits(false, rec);
    }
    says();
    work();
    ends_with(&success())
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
    let first =
        format!(r#"{{"type":"system","subtype":"init","tools":{TOOLS},"mcp_servers":[],"permissionMode":{MODE}}}"#);
    line(&first);
    let mut left = total - last.len() - first.len() - 1;
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

/// `litter` leaves a program of its own going, in the agent's process group and with none of its pipes, then ends as
/// the number says: 0 with a success, any other with that exit and no result. The program it leaves is this one,
/// started with `litters`: it holds the lock on `litter-held` and writes `litter.txt`, for half a minute.
#[allow(clippy::zombie_processes, reason = "the program left going is the point: it must outlive this one")]
fn litter(rec: &Path, code: usize) -> i32 {
    let mut left = process::Command::new(env::current_exe().unwrap());
    left.arg("litters").arg(rec).stdin(process::Stdio::null()).stdout(process::Stdio::null());
    left.stderr(process::Stdio::null()).spawn().unwrap();
    while !rec.join("litter.txt").exists() {
        thread::sleep(Duration::from_millis(10));
    }
    edits();
    if code == 0 { ends_with(&success()) } else { i32::try_from(code).unwrap() }
}

fn litters(rec: &Path) -> ! {
    let held = fs::File::create(rec.join("litter-held")).unwrap();
    held.lock().unwrap();
    for _ in 0..3000 {
        if rec.join("stop").exists() {
            break;
        }
        let mut file = fs::OpenOptions::new().append(true).create(true).open(rec.join("litter.txt")).unwrap();
        file.write_all(b"more\n").unwrap();
        thread::sleep(Duration::from_millis(10));
    }
    process::exit(0)
}

/// `deaf` starts this program again as one that the stop signal does not reach: the system keeps a signal ignored
/// across the start of a program, and `perl` can ask for that. Where that cannot be done it leaves `cannot`. By its
/// number it is deaf to the stop signal (0), to the interrupt (1) or to both (2). 3 is [`minded`].
fn deafen(rec: &Path, number: usize) -> ! {
    #[cfg(not(unix))]
    let _ = number; // nothing here can be made deaf
    #[cfg(unix)]
    if Path::new("/usr/bin/perl").is_file() {
        let deaf = ["$SIG{TERM}", "$SIG{INT}", "$SIG{TERM} = $SIG{INT}"][number.min(2)];
        let ignore = format!("{deaf} = 'IGNORE'; exec @ARGV; exit 9");
        let mut perl = process::Command::new("/usr/bin/perl");
        perl.args(["-e", &ignore]).arg(env::current_exe().unwrap()).arg("now");
        let failed = std::os::unix::process::CommandExt::exec(&mut perl);
        eprintln!("perl could not be started: {failed}");
    }
    cannot(rec)
}

fn cannot(rec: &Path) -> ! {
    let why = if cfg!(unix) { "there is no /usr/bin/perl to start one with" } else { "this system has no signals" };
    fs::write(rec.join("cannot"), format!("nothing here starts a program deaf to the stop signal: {why}")).unwrap();
    process::exit(1)
}

/// What `perl` is told for [`minded`]. It starts the program as a child of its own, in its process group.
#[cfg(unix)]
const MINDS: &str = r#"my ($rec, $sig, $last, $flood) = splice(@ARGV, 0, 4);
$SIG{INT} = 'IGNORE' if $sig eq 'TERM';
my $pid = fork;
if (!$pid) { exec @ARGV; exit 9 }
$SIG{$sig} = sub {
    open(my $heard, '>', "$rec/heard"); close $heard;
    $| = 1;
    print(('j' x 65535) . "\n") for 1 .. $flood;
    print "$last\n" if length $last;
    exit 0
};
waitpid($pid, 0);
exit 1"#;

/// `deaf` 3, `obeys` and `floods` start this program again below a `perl` that minds one signal. At that signal
/// the `perl` leaves `heard`, prints `flood` lines of junk and then `last`, when there is one, and exits 0, while
/// the signal ends this program. One that minds the stop signal is deaf to the interrupt, and so is this program
/// below it. Where that cannot be done it leaves `cannot`.
fn minded(rec: &Path, signal: &str, last: &str, flood: usize) -> ! {
    #[cfg(not(unix))]
    let _ = (signal, last, flood); // nothing here can be made to mind a signal
    #[cfg(unix)]
    if Path::new("/usr/bin/perl").is_file() {
        let mut perl = process::Command::new("/usr/bin/perl");
        perl.args(["-e", MINDS]).arg(rec).args([signal, last, &flood.to_string()]);
        let failed = std::os::unix::process::CommandExt::exec(perl.arg(env::current_exe().unwrap()).arg("now"));
        eprintln!("perl could not be started: {failed}");
    }
    cannot(rec)
}

/// The scripts that start this program a second time, with something done about signals first.
fn again(script: &str, rec: &Path, number: usize) {
    match script {
        "deaf" if number < 3 => deafen(rec, number),
        "deaf" => minded(rec, "TERM", "", 0),
        "obeys" => minded(rec, "INT", &if number == 1 { success() } else { String::new() }, 0),
        "floods" => minded(rec, "INT", "", 1100),
        _ => {}
    }
}

/// `obeys` and `floods`, started the second time: the work, the result too when it comes `first`, and then a wait
/// that the `perl` above ends at the interrupt. There `obeys` reports a success (1) or has already (0), and
/// `floods` prints more than is read of a run.
fn obeys(first: bool, rec: &Path) -> i32 {
    edits();
    if first {
        line(&success());
    }
    waits(false, rec)
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
    for name in ["PATH", "HOME", "TMPDIR", "CLAUDE_CONFIG_DIR", "USERPROFILE"] {
        let value = env::var_os(name).unwrap_or_default();
        fs::write(rec.join(format!("var-{name}")), value.to_string_lossy().as_bytes()).unwrap();
    }
    fs::write(rec.join("cwd"), env::current_dir().unwrap().to_string_lossy().as_bytes()).unwrap();
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).unwrap();
    fs::write(rec.join("stdin"), input).unwrap();
}

fn main() {
    if env::args().nth(1).as_deref() == Some("litters") {
        litters(Path::new(&env::args().nth(2).unwrap()));
    }
    let play = fs::read_to_string(env::current_exe().unwrap().with_file_name("play")).unwrap();
    let play: Vec<&str> = play.lines().collect();
    let (script, rec, number) = (play[0], Path::new(play[1]), play[2].parse::<usize>().unwrap());
    if env::args().nth(1).as_deref() != Some("now") {
        again(script, rec, number);
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
        "obeys" | "floods" => obeys(script == "obeys" && number == 0, rec),
        "scribble" => scribbles(rec),
        other => gate::play(other, number, rec),
    };
    process::exit(code);
}
