//! The standing set: ordinary pictures, typed here in the shapes drawing programs write, that the door must take.
//! Every later change to a rule or to a cost must still accept each of them, through the kit's own child, with
//! room to spare. The gate on PR 154 found four of these refused by a rule that had just been added.
//!
//! And the other side of the cost: a picture inside every written rule that ends, and costs the page a second
//! and hundreds of megabytes, is refused for what it costs and not for how long the machine took.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::fs;
use std::time::{Duration, Instant};

use sticker_kit::theme::{MOST_MEMORY, MOST_PROCESSOR, TRIAL_LIMIT, svg_refusal};
use support::{Folder, OUTER, alone, sized, trial_as, wide_stroke};

/// How many times an ordinary picture's cost fits in what a picture may cost, at the least.
const ROOM: u64 = 4;

fn open(size: &str, inside: &str) -> String {
    format!(r#"<svg xmlns="http://www.w3.org/2000/svg" {size}>{inside}</svg>"#)
}

fn ordinary() -> Vec<(&'static str, String)> {
    let heart = r##"<defs><linearGradient id="g" x1="0%" y1="0%" x2="100%" y2="100%"><stop offset="0" stop-color="#ff6b8a"/><stop offset="1" stop-color="#c2185b"/></linearGradient></defs><path d="M32 56C12 42 4 30 4 20a14 14 0 0 1 28-4 14 14 0 0 1 28 4c0 10-8 22-28 36z" fill="url(#g)"/>"##;
    let background =
        r##"<rect width="100%" height="100%" fill="#fff4d6"/><circle cx="24" cy="24" r="14" fill="#f2a900"/>"##;
    let sun = r##"<defs><radialGradient id="s" cx="50%" cy="50%" r="50%"><stop offset="0" stop-color="#fff59d"/><stop offset="1" stop-color="#fbc02d"/></radialGradient></defs><circle cx="32" cy="32" r="28" fill="url(#s)"/>"##;
    let pill = r##"<rect x="4" y="16" width="56" height="32" rx="50%" fill="#7e57c2"/><rect x="10" y="22" width="44" height="20" rx="10" ry="10" fill="#ede7f6"/>"##;
    let star = r##"<polygon points="32,6 39,24 58,24 43,36 48,54 32,43 16,54 21,36 6,24 25,24" fill="#ffd54f" stroke="#f57f17" stroke-width="3" stroke-linejoin="round"/><path d="M12 58q20 -8 40 0" fill="none" stroke="#f57f17" stroke-width="4" stroke-linecap="round" stroke-linejoin="round"/><line x1="8" y1="8" x2="14" y2="14" stroke="#f57f17" stroke-width="10%" stroke-linecap="round"/>"##;
    let leaf = r##"<g transform="translate(24 24) rotate(-30)"><ellipse rx="18" ry="9" fill="#66bb6a"/><line x1="-18" y1="0" x2="18" y2="0" stroke="#2e7d32" stroke-width="1.5" stroke-linecap="round"/></g>"##;
    let badge = format!(r##"<path d="M20 2{}z" fill="#42a5f5"/>"##, "a1.5 1.5 0 0 1 2 1".repeat(40));
    let cloud = format!(r##"<path d="M4 30{}z" fill="#eceff1"/>"##, "a0.5 0.5 0 0 1 1 0".repeat(33));
    let wave = format!(
        r##"<path d="M2 20{}" fill="none" stroke="#0288d1" stroke-linecap="round"/>"##,
        "a0.5 0.5 0 0 1 1 0a0.5 0.5 0 0 0 1 0".repeat(18)
    );
    let size = r#"width="64" height="64" viewBox="0 0 64 64""#;
    let commented = format!("<!-- {} -->\n{}", "=".repeat(50), open(size, heart));
    vec![
        ("a heart filled with a gradient from corner to corner", open(size, heart)),
        ("a background the size of the picture", open(r#"width="48" height="48""#, background)),
        ("a sun with a round gradient", open(size, sun)),
        ("a pill with corners of half its size", open(size, pill)),
        ("a star and a smile in round strokes", open(size, star)),
        ("a leaf as large as what holds it", open(r#"width="100%" height="100%" viewBox="0 0 48 48""#, leaf)),
        ("a heart under a line of signs in a comment", commented),
        ("a badge of forty scallops", sized(128, &badge)),
        ("a cloud of 33 bumps", sized(64, &cloud)),
        ("a wave of 36 half turns", sized(64, &wave)),
    ]
}

/// The numbers of an answer's line: what the rules count twice, the pixels drawn, and what the draw cost in
/// milliseconds of processor time and KiB of memory.
fn accepted(line: &str) -> Option<[u64; 5]> {
    let numbers: Vec<u64> = line.strip_prefix("accepted ")?.trim_end().split(' ').flat_map(str::parse).collect();
    numbers.try_into().ok()
}

#[test]
fn every_ordinary_picture_is_accepted_by_the_kit_s_child_with_room_to_spare() {
    let (most_ms, most_kib) = (MOST_PROCESSOR.as_millis() as u64, MOST_MEMORY / 1024);
    for (name, svg) in ordinary() {
        assert_eq!(svg_refusal(&svg), None, "{name}: no rule refuses it");
        let (status, said, lived) = alone(&["trial"], &svg, OUTER, false);
        let Some([_, _, pixels, ms, kib]) = accepted(&said).filter(|_| status.is_some_and(|status| status.success()))
        else {
            panic!("{name}: {said:?} ({status:?})");
        };
        eprintln!("{name}: {ms} ms of processor time and {kib} KiB for {pixels} pixels, the child gone in {lived:?}");
        assert!(pixels > 0, "{name} was drawn");
        assert!(ms * ROOM <= most_ms, "{name}: {ms} ms is not a {ROOM}th of the {most_ms} a picture may take");
        assert!(kib * ROOM <= most_kib, "{name}: {kib} KiB is not a {ROOM}th of the {most_kib} a picture may take");
    }
}

// The gate drew these in the window's process: 60 pairs of curves for 0.75 s and 220 MB, 100 for 0.62 s and
// 248 MB on a small stack. Each ends, so the clock says nothing of them.
#[test]
fn a_picture_that_ends_and_costs_the_page_dear_is_refused_for_its_cost_and_the_refusal_is_kept() {
    let folder = Folder::new("dear");
    let count = folder.0.join("count");
    let mut counted = trial_as(&["counted", count.to_str().unwrap()]);
    counted.kept = Some(folder.0.join("kept"));
    for (nth, pairs) in [60, 100].into_iter().enumerate() {
        // No rule is asked of it here: the parser alone takes seconds over it, which is what is measured.
        let svg = wide_stroke(pairs);
        let started = Instant::now();
        let said = folder.star(&svg, &counted).expect_err("too dear");
        let took = started.elapsed();
        eprintln!("{pairs} pairs: {said} in {took:?}");
        // The cost refused it, and no rule: every rule passed it to the parser, whose time this is.
        assert!(said.starts_with("drawing it took more than "), "{pairs} pairs: {said}");
        assert!(took < TRIAL_LIMIT / 2, "{pairs} pairs: refused long before the clock would have: {took:?}");
        // The refusal is about the picture, on any machine and under any load: it is kept.
        assert_eq!(folder.star(&svg, &counted), Err(said));
        assert_eq!(fs::read(&count).unwrap().len(), nth + 1, "{pairs} pairs: tried once");
    }
    assert!(TRIAL_LIMIT >= 4 * MOST_PROCESSOR, "the clock ends only what the cost has not");
    assert!(MOST_PROCESSOR >= Duration::from_millis(100) && MOST_MEMORY >= 16 * 1024 * 1024);
}

// The gate's picture of 1022 arcs to one place, each of which the parser draws nothing for and calls itself: with
// the rule that refuses it switched off, a child draws it on a stack of the page's size. Whether that stack holds
// it or the child ends on it, the caller lives and the next picture is tried.
#[test]
fn a_run_of_arcs_to_one_place_is_drawn_on_the_page_s_stack_and_costs_one_child_at_most() {
    let folder = Folder::new("one-place");
    let arcs = format!("M0 0A9 9 0 0 0 1e-17 0{}L9 9", " 9 9 0 001e-17 0".repeat(1022));
    let svg = sized(64, &format!(r##"<path d="{arcs}" fill="none" stroke="#123456" stroke-width="2"/>"##));
    // Asked of a child, as every picture is: read here by a door that let it by, it would end this test.
    let refused = folder.star(&svg, &trial_as(&["trial"])).expect_err("the rule refuses it");
    assert!(refused.starts_with("a path holds more than 32 closes, or arcs that go nowhere, in a row"), "{refused}");
    let said = folder.star(&svg, &trial_as(&["unruled"]));
    eprintln!("1022 arcs to one place, held to no rule, on the page's stack: {said:?}");
    let ended = said.as_ref().is_err_and(|said| said.starts_with("its trial ended badly ("));
    assert!(ended || said == Ok(svg.clone()), "{said:?}");
    assert_eq!(folder.star(support::HONEST, &trial_as(&["trial"])), Ok(support::HONEST.to_string()));
}
