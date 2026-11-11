//! F28 in words: whatever a page holds that is not drawn is said, in words that agree with the count, and the command
//! line says its own limits.

use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::{Value, json};

use super::support::{self, Piece, clash, page, pieces, wish};

fn notes(page: &Value) -> Vec<String> {
    pieces(page, &[]).iter().filter(|piece| piece.kind() == "note").map(Piece::said).collect()
}

#[test]
fn the_stars_of_a_wish_in_a_state_the_app_cannot_draw_are_counted_in_its_note() {
    let paused = |stars: &[&str]| page(vec![wish("w1", "Faster search", "paused", stars)], vec![]);
    let note = "Faster search A wish in a state this app cannot draw yet: paused.";
    assert_eq!(notes(&paused(&[])), [note]);
    assert_eq!(notes(&paused(&["indexed"])), [format!("{note} Its 1 star is not drawn.")]);
    assert_eq!(notes(&paused(&["indexed", "cached", "timed"])), [format!("{note} Its 3 stars are not drawn.")]);
    let many = notes(&paused(&["one"; 15]));
    assert_eq!(many[0], "not everything is drawn Left out of this page: 3 stars.");
    assert_eq!(many[1], format!("{note} Its 12 stars are not drawn."), "15 stars: 12 said here, 3 above");
}

#[test]
fn one_of_a_thing_left_out_is_said_in_the_singular() {
    let mut wishes: Vec<Value> = (0..25).map(|i| wish(&format!("w{i}"), "A wish", "working", &[])).collect();
    wishes[0]["stars"] = json!(vec!["one"; 13]);
    let clashes = (0..9).map(|_| clash("w1", "w2", "this", "that", false)).collect();
    let mut over = page(wishes, clashes);
    over["reminders"] = json!(["1", "2", "3", "4", "5", "6", "7", "8", "9"]);
    assert_eq!(
        notes(&over)[0],
        "not everything is drawn Left out of this page: 1 wish, 1 clash, 1 star, 1 thing in a part this app does not \
         know."
    );
    over["wishes"].as_array_mut().unwrap().push(wish("w25", "A wish", "working", &[]));
    over["reminders"].as_array_mut().unwrap().push(json!("10"));
    assert!(notes(&over)[0].ends_with("2 wishes, 1 clash, 1 star, 2 things in parts this app does not know."));
}

#[test]
fn the_size_s_error_line_states_both_ends() {
    let day = page(vec![wish("w1", "A wish", "working", &[])], vec![]).to_string();
    for size in ["16385x640", "960x16385", "65536x65536", "959x640"] {
        let out = support::run(&["layout", "--page", "-", "--size", size], Some(&day));
        let said = String::from_utf8_lossy(&out.stderr);
        let line =
            format!("jellymoon-app: --size must be <W>x<H> in pixels, from 960x640 to 16384x16384, not {size:?}\n");
        assert_eq!(out.status.code(), Some(2), "{size}");
        assert!(said.starts_with(&line), "{size}: {said}");
    }
    for size in ["960x640", "16384x16384"] {
        assert!(support::run(&["layout", "--page", "-", "--size", size], Some(&day)).status.success(), "{size}");
    }
}

#[test]
fn stdin_is_read_only_to_its_cap_however_much_is_piped_in() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_jellymoon-app"));
    cmd.args(["layout", "--page", "-"]).env_clear().stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(root) = std::env::var_os("SystemRoot").filter(|_| cfg!(windows)) {
        cmd.env("SystemRoot", root);
    }
    let mut child = cmd.spawn().unwrap();
    let mut pipe = child.stdin.take().unwrap();
    // 64 MiB are offered. A reader that stops at its cap closes the pipe after 1 MiB and a little; the rest is refused.
    // The pipe itself holds up to 64 KiB more.
    let chunk = vec![b' '; 64 * 1024];
    let taken = (0..1024).take_while(|_| pipe.write_all(&chunk).is_ok()).count();
    drop(pipe);
    let out = child.wait_with_output().unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stderr), "jellymoon-app: the file is not page data: it is over 1 MiB\n");
    assert_eq!(out.status.code(), Some(1));
    assert!(taken * 64 <= 1024 + 256, "{} KiB were read from stdin, which is capped at 1 MiB", taken * 64);
}
