//! The gate on PR 154: an arc the renderer's path parser draws nothing for does not move the parser, so the next
//! arc to the same place is another, and each is one more call of the parser by itself. The door moved its own
//! place, took the second for an arc to where it stood, which is drawn, and began its count again: 1022 of them
//! passed. The door now holds how far from its own place the parser may stand, and counts an arc that may go
//! nowhere from there.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use sticker_kit::theme::{MAX_ART_IDLE, svg_refusal};

/// How many segments the renderer's path parser draws of `d`.
fn drawn(d: &str) -> usize {
    svgtypes::SimplifyingPathParser::from(d).map_while(Result::ok).count()
}

/// Whether the door refuses the path for a run of segments the parser draws nothing for; any other refusal fails.
fn held(d: &str) -> bool {
    let path = format!(r##"<path d="{d}" fill="none" stroke="#123456" stroke-width="2"/>"##);
    let said = svg_refusal(&format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 40 40">{path}</svg>"#));
    assert!(said.iter().all(|why| why.starts_with("a path holds more than 32 closes, or arcs that go nowhere")));
    said.is_some()
}

#[test]
fn a_run_of_arcs_to_one_place_is_a_run_the_parser_draws_nothing_for() {
    // The gate's own: every arc after the first is written to the very place the first was.
    let to_one_place = |arcs: usize| format!("M0 0A9 9 0 0 0 1e-17 0{}L9 9", " 9 9 0 001e-17 0".repeat(arcs - 1));
    assert_eq!(drawn(&to_one_place(MAX_ART_IDLE + 1)), 2, "the parser draws the move and the line, and no arc");
    // The gate's 1023 are asked of a child (`tests/ordinary.rs`): the parser would end this test on its stack.
    assert_eq!(
        (held(&to_one_place(MAX_ART_IDLE)), held(&to_one_place(MAX_ART_IDLE + 1)), held(&to_one_place(64))),
        (false, true, true)
    );
}

// The other simple door stays at every arc it counts. It counts more arcs than the parser draws nothing for, since
// it does not do an arc's sums: so the parser may have moved on where the door stayed, and an arc back to the
// door's place is then one that goes nowhere, which that door took for an arc to where it stands.
#[test]
fn the_door_counts_an_arc_that_may_go_nowhere_from_where_the_parser_may_stand() {
    // A small arc the parser does draw, a hair long, and then large arcs back to where the path began: from
    // the hair's end each goes nowhere.
    let back = |arcs: usize| format!("M0 0A2e-5 2e-5 0 0 0 1e-18 0{}L9 9", "A1e6 1e6 0 0 0 0 0".repeat(arcs));
    assert_eq!(drawn(&back(40)), 3, "the parser draws the move, the hair and the line");
    assert_eq!((held(&back(MAX_ART_IDLE - 1)), held(&back(MAX_ART_IDLE))), (false, true));

    // An arc that goes somewhere is drawn, wherever the path stood, and begins the count again.
    let apart = format!("M0 0{}", "A9 9 0 0 0 1e-17 0A9 9 0 0 0 9 9A9 9 0 0 0 1e-17 0".repeat(40));
    assert!(!held(&apart));
}
