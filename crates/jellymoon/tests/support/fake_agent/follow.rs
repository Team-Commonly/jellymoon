//! The made-up agent's part in a follow-up (F53, `tests/f53.rs`): a harness that is told to fork a session it
//! keeps, and one that keeps it no more. Each start leaves what it was given, under the number of that start:
//! its whole command line in `line<n>`, an argument a line, and in `found<n>` what the folder it was started in
//! held, which is how a test knows the commit a follow-up's copy was made at. Every line is typed here.

use std::path::Path;
use std::{env, fs};

use super::asks::{init, listed, option};
use super::{MARKER, TOOLS, held, line, says, starts, success, waits_held};

/// Leaves what this start was given, and says whether it was told to fork a session.
fn given(rec: &Path) -> bool {
    let n = fs::read_to_string(rec.join("starts")).map_or(0, |starts| starts.lines().count());
    let args: Vec<String> = env::args().skip(1).collect();
    fs::write(rec.join(format!("line{n}")), args.join("\n")).unwrap();
    let held = |file: &str| format!("{file}: {}", fs::read_to_string(file).unwrap_or_else(|_| "none\n".to_string()));
    fs::write(rec.join(format!("found{n}")), ["a.txt", "b.txt", "c.txt", "n.txt", "m.txt"].map(held).concat()).unwrap();
    args.iter().any(|arg| arg == "--fork-session")
}

/// What a follow-up adds to the work it follows: the file that work edited is edited again, and one is new, or
/// a line longer when a follow-up before this one made it.
fn more() {
    fs::write("a.txt", "edited by the run, and again by the follow-up\n").unwrap();
    let before = fs::read_to_string("m.txt").unwrap_or_default();
    fs::write("m.txt", format!("{before}more from the follow-up\n")).unwrap();
}

/// A harness that keeps no session of the id it is told to fork (ADR 0018, "Measured"): it names that id, on
/// stderr and in a result that is an error, and exits with 1.
fn lost() -> i32 {
    let said = format!("No conversation found with session ID: {}", option("--resume").unwrap_or_default());
    eprintln!("{said}");
    line(&format!(
        r#"{{"type":"result","subtype":"error_during_execution","is_error":true,"errors":["{said}"],"result":"{MARKER}"}}"#
    ));
    1
}

/// The scripts of F53. None: the script is not one of them.
pub fn play(script: &str, number: usize, rec: &Path) -> Option<i32> {
    let code = match (script, number) {
        // A follow-up's turn: 0 does its work, 1 fails, 2 changes nothing. 3 is a fork that starts under the
        // session it was told to fork and not under the new id it was given: it never ends by itself.
        ("follow", 1) => {
            given(rec);
            starts();
            3
        }
        ("follow", 3) => {
            given(rec);
            let held = held(rec);
            let forked = option("--resume").unwrap_or_default();
            init(&listed(TOOLS), &format!(r#""session_id":"{forked}","#));
            says();
            waits_held(false, rec, held)
        }
        ("follow", _) => {
            given(rec);
            starts();
            if number != 2 {
                more();
            }
            line(&success());
            0
        }
        // A harness that keeps the followed session no more. Told to fork, it says so and has done nothing;
        // started again under a new session, 0 does the work, 1 fails, and 2 says once more that it keeps no
        // such session, which nothing asked it for.
        ("unkept", _) if given(rec) => lost(),
        ("unkept", 2) => lost(),
        ("unkept", _) => {
            starts();
            if number == 1 {
                return Some(3);
            }
            more();
            line(&success());
            0
        }
        _ => return None,
    };
    Some(code)
}
