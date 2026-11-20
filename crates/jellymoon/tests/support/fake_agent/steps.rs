//! The made-up agent's steps (live work): tool uses in its stream, with a wait between them.

use std::path::Path;
use std::time::Duration;
use std::{env, fs, thread};

use super::{ends_with, held, line, starts, success};

/// One tool use, in the shape of an `assistant` line of the stream (`says`) with a `tool_use` block as a session
/// log holds one (`tests/f26.rs`), its input a `file_path` as a denial's `tool_input` names one: the file of that
/// name in the folder the agent was started in, by its whole path.
fn uses(tool: &str, file: &str) {
    let path = env::current_dir().unwrap().join(file).display().to_string();
    let path = path.replace('\\', "\\\\").replace('"', "\\\"");
    let block = format!(r#"{{"type":"tool_use","id":"t1","name":"{tool}","input":{{"file_path":"{path}"}}}}"#);
    line(&format!(r#"{{"type":"assistant","message":{{"role":"assistant","content":[{block}]}}}}"#));
}

/// `steps`: a run that reads `b.txt`, then waits for the test's `go`, holding the lock as `slow` does, half a
/// minute at most; then changes `a.txt` and ends with a success.
pub fn steps(rec: &Path) -> i32 {
    starts();
    uses("Read", "b.txt");
    let _held = held(rec);
    wait(rec, "go", 3000);
    uses("Edit", "a.txt");
    fs::write("a.txt", "edited by the run\n").unwrap();
    ends_with(&success())
}

/// Waits until the test leaves `left`, `turns` times 10 ms at most.
pub(super) fn wait(rec: &Path, left: &str, turns: usize) {
    for _ in 0..turns {
        if rec.join(left).exists() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
}
