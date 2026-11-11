//! F28: the app draws a page from page data using only the kit. Black-box: each test writes made-up page data, runs
//! the built `jellymoon-app layout` on it and reads the placed pieces it prints, one JSON object a line.

#[path = "f28/door.rs"]
mod door;
mod support;
#[path = "f28/words.rs"]
mod words;

use serde_json::{Value, json};
use support::{Piece, clash, layout, page, pieces, refused, wish};

/// A day with every state, six stars and a settled clash.
fn day() -> Value {
    page(
        vec![
            wish("w1", "Uploader retry", "let_go", &["retry added"]),
            wish("w2", "Fail fast uploads", "done", &["retry removed", "tests pass"]),
            wish("w3", "Login page", "needs_you", &[]),
            wish("w4", "Faster search", "working", &["index built", "cache warmed", "query cut"]),
        ],
        vec![clash("w1", "w2", "add a retry to the uploader", "never retry uploads, fail fast", true)],
    )
}

#[test]
fn every_placed_piece_is_printed_with_its_kind_position_and_size() {
    let placed = pieces(&day(), &[]);
    for piece in &placed {
        let rect = piece.rect();
        assert!(rect.iter().all(|n| n.is_finite()) && rect[2] >= 0.0 && rect[3] >= 0.0, "{piece:?}");
        assert!(piece.0["tilt"].is_number() && piece.0["index"].is_u64() && piece.0["id"].is_string(), "{piece:?}");
    }
    for kind in
        ["paper", "hole", "tape", "title", "hero", "character", "star", "label", "note", "option", "legend", "footer"]
    {
        assert!(placed.iter().any(|piece| piece.kind() == kind), "no {kind} piece was printed");
    }
    let said = |kind: &str| placed.iter().find(|piece| piece.kind() == kind).map(Piece::said).unwrap_or_default();
    assert_eq!(said("title"), "what I wished for today");
    assert_eq!(said("tape"), "Tue · Mar 10");
    assert_eq!(
        said("footer"),
        "Read from 5 things you typed in 2 projects (alpha, beta). This is the model's reading, unedited."
    );
    let legend: Vec<String> = placed.iter().filter(|piece| piece.kind() == "legend").map(Piece::said).collect();
    assert_eq!(legend, ["★ finished", "working on it", "needs you", "let go"]);
}

#[test]
fn a_wish_gets_one_little_one_and_one_star_per_finished_job() {
    let mut day = day();
    day["wishes"][1]["stars"] = json!(["one", "two", "three", "four", "five", "six"]);
    let placed = pieces(&day, &[]);
    let of = |kind: &str, id: &str| placed.iter().filter(|piece| piece.kind() == kind && piece.id() == id).count();
    for (index, (id, stars, bubble, badge)) in
        [("w1", 1, 0, 0), ("w2", 4, 0, 0), ("w3", 0, 0, 1), ("w4", 3, 1, 0)].into_iter().enumerate()
    {
        assert_eq!(of("character", id), 1, "{id}: one little one");
        assert_eq!(of("label", id), 1, "{id}: one label");
        assert_eq!(of("star", id), stars, "{id}: one star per finished job, four at most");
        assert_eq!((of("bubble", id), of("badge", id)), (bubble, badge), "{id}: working and needs-you marks");
        let who = placed.iter().find(|piece| piece.kind() == "character" && piece.id() == id).unwrap();
        assert_eq!(who.0["index"], index, "{id}: its place in page order");
    }
    assert_eq!(placed.iter().filter(|piece| piece.kind() == "character").count(), 4);
    assert_eq!(placed.iter().filter(|piece| piece.kind() == "hero").count(), 1);
}

#[test]
fn a_let_go_wish_is_drawn_greyed_and_no_other_is() {
    let placed = pieces(&day(), &[]);
    let state = |id: &str| {
        let who = placed.iter().find(|piece| piece.kind() == "character" && piece.id() == id).unwrap();
        who.0["state"].as_str().unwrap().to_string()
    };
    assert_eq!([state("w1"), state("w2"), state("w3"), state("w4")], ["dimmed", "idle", "needs_you", "busy"]);
    let label = |id: &str| placed.iter().find(|piece| piece.kind() == "label" && piece.id() == id).unwrap().roles();
    assert_eq!(label("w1"), ["label_struck"]);
    assert_eq!(label("w2"), ["label"]);
}

#[test]
fn a_clash_gets_one_note_with_both_quotes() {
    let mut day = day();
    day["clashes"].as_array_mut().unwrap().push(clash("w3", "w4", "ship it today", "wait for the review", false));
    let placed = pieces(&day, &[]);
    let of = |kind: &str, id: &str| -> Vec<&Piece> {
        placed.iter().filter(|piece| piece.kind() == kind && piece.id() == id).collect()
    };
    for (id, quotes, first) in [
        ("clash-1", ["“add a retry to the uploader”", "“never retry uploads, fail fast”"], "option_struck"),
        ("clash-2", ["“ship it today”", "“wait for the review”"], "option"),
    ] {
        assert_eq!(of("note", id).len(), 1, "{id}: one note");
        let options = of("option", id);
        assert_eq!(options.iter().map(|option| option.said()).collect::<Vec<_>>(), quotes, "{id}: both quotes");
        assert_eq!(options[0].roles(), [first], "{id}: the first is struck only when the clash is settled");
        assert_eq!(options[1].roles(), ["option"]);
    }
    assert_eq!(placed.iter().filter(|piece| piece.kind() == "note" && piece.id().starts_with("clash-")).count(), 2);
}

#[test]
fn the_came_true_note_lists_each_star_with_its_wish() {
    let placed = pieces(&day(), &[]);
    let list = placed.iter().find(|piece| piece.kind() == "note" && piece.said().starts_with("came true today"));
    let rows: Vec<String> =
        list.unwrap().runs().iter().skip(1).map(|run| run["text"].as_str().unwrap().into()).collect();
    let want = [
        ["★", "retry added", "Uploader retry"],
        ["★", "retry removed", "Fail fast uploads"],
        ["★", "tests pass", "Fail fast uploads"],
        ["★", "index built", "Faster search"],
        ["★", "cache warmed", "Faster search"],
        ["★", "query cut", "Faster search"],
    ];
    assert_eq!(rows, want.concat());
}

#[test]
fn a_page_with_no_wishes_says_so() {
    let placed = pieces(&page(vec![], vec![]), &[]);
    assert_eq!(placed.iter().filter(|piece| piece.kind() == "character").count(), 0);
    assert!(
        placed.iter().any(|piece| piece.kind() == "note" && piece.said().contains("Nothing wished for yet today."))
    );
    let footer = placed.iter().find(|piece| piece.kind() == "footer").unwrap().said();
    assert_eq!(footer, "Read from 0 things you typed. This is the model's reading, unedited.");
}

/// Eight labels as long as page data allows: 40 wide letters, 40 East Asian characters, and six sentences of 40.
fn longest() -> Value {
    let labels = [
        "W".repeat(40),
        "手帐を書く日".chars().cycle().take(40).collect(),
        "Rebuild the uploader with a retry budget".to_string(),
        "Make every search answer in under a tick".to_string(),
        "Write the weekly letter to the gardeners".to_string(),
        "Sort four hundred photos by who is in it".to_string(),
        "Teach the parser about nested blockquote".to_string(),
        "Move the old notes into the new notebook".to_string(),
    ];
    let states = ["working", "done", "needs_you", "let_go"];
    let wishes = labels.iter().enumerate().map(|(i, label)| {
        assert_eq!(label.chars().count(), 40);
        wish(&format!("w{}", i + 1), label, states[i % 4], &["one", "two", "three", "four"])
    });
    page(wishes.collect(), vec![clash("w1", "w2", "keep the long names", "cut every name short", false)])
}

#[test]
fn no_two_labels_overlap_on_a_page_of_eight_wishes_with_the_longest_labels() {
    for (size, width, height) in [(None, 1440.0, 900.0), (Some("960x640"), 960.0, 640.0)] {
        for theme in ["sticker", "plain"] {
            let mut args = vec!["--theme", theme];
            args.extend(size.iter().flat_map(|size| ["--size", size]));
            let placed = pieces(&longest(), &args);
            let labels: Vec<&Piece> = placed.iter().filter(|piece| piece.kind() == "label").collect();
            let paper = placed.iter().find(|piece| piece.kind() == "paper").unwrap().rect();
            assert_eq!(labels.len(), 8, "{theme} {size:?}: every wish has a label");
            for (i, label) in labels.iter().enumerate() {
                let [x, y, w, h] = label.rect();
                assert!(x >= 0.0 && y >= 0.0 && x + w <= width && y + h <= height, "{theme} {size:?}: off the window");
                assert!(
                    x >= paper[0] && y >= paper[1] && x + w <= paper[0] + paper[2] && y + h <= paper[1] + paper[3],
                    "{theme} {size:?}: {} leaves the paper",
                    label.id()
                );
                for other in &labels[i + 1..] {
                    assert!(!overlap(label.rect(), other.rect()), "{theme} {size:?}: {} on {}", label.id(), other.id());
                }
                for run in label.runs() {
                    let [rx, _, rw, _] = support::rect(run);
                    assert!(rx >= x - 0.5 && rx + rw <= x + w + 0.5, "{theme} {size:?}: text leaves its label");
                }
            }
        }
    }
}

fn overlap(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3]
}

#[test]
fn text_from_page_data_is_drawn_as_text_only() {
    let nasty = format!(
        "<script>alert(1)</script> & <b>bold</b>\n\tline two\u{200F}\u{202E}\u{0007}\u{1b}[31m{}",
        "x".repeat(5000)
    );
    let calm = pieces(&day(), &[]);
    let mut day = day();
    day["day"] = json!(nasty);
    day["read"]["projects"] = json!([nasty, "beta"]);
    for field in ["label", "said", "project", "id"] {
        day["wishes"][2][field] = json!(nasty);
    }
    day["wishes"][1]["stars"][0] = json!(nasty);
    day["clashes"][0]["quote_a"] = json!(nasty);
    let wild = pieces(&day, &[]);

    let keys =
        |placed: &[Piece]| placed.iter().map(|p| (p.kind().to_string(), p.0["index"].clone())).collect::<Vec<_>>();
    assert_eq!(keys(&calm), keys(&wild), "odd text adds or removes no piece");
    for (before, after) in calm.iter().zip(&wild) {
        if matches!(before.kind(), "paper" | "hole" | "hero" | "character" | "star" | "bubble" | "badge") {
            assert_eq!(before.rect(), after.rect(), "{} moved", before.kind());
        }
    }
    for run in wild.iter().flat_map(Piece::runs) {
        let text = run["text"].as_str().unwrap();
        assert!(text.chars().all(|c| !c.is_control()), "a control character was kept: {text:?}");
        assert!(!text.contains(['\u{200F}', '\u{202E}']), "a direction mark was kept: {text:?}");
        assert!(text.chars().count() <= 401, "a very long text was not cut");
    }
    let labels: Vec<String> = wild.iter().filter(|piece| piece.kind() == "label").map(Piece::said).collect();
    assert!(labels.iter().any(|label| label.starts_with("<script>alert(1)")), "markup is shown, not read: {labels:?}");
    let quote = wild.iter().find(|piece| piece.kind() == "option").unwrap().said();
    assert!(quote.starts_with("“<script>alert(1)</script> & <b>bold</b> line two"), "{quote}");
}

#[test]
fn a_kind_the_app_does_not_know_falls_back_to_a_plain_note() {
    let mut day = day();
    day["wishes"][3]["state"] = json!("paused");
    day["reminders"] = json!([{ "label": "Water the plants" }]);
    // No star, so no list note: the clash and the two plain notes all fit the page.
    (0..3).for_each(|wish| day["wishes"][wish]["stars"] = json!([]));
    let placed = pieces(&day, &[]);

    let little_ones: Vec<&str> = placed.iter().filter(|p| p.kind() == "character").map(Piece::id).collect();
    assert_eq!(little_ones, ["w2", "w3", "w1"], "a wish in an unknown state gets no little one");
    assert!(!placed.iter().any(|piece| piece.id() == "w4"), "and no star, mark or label");
    let plain: Vec<&Piece> =
        placed.iter().filter(|p| p.kind() == "note" && p.roles().contains(&"body".into())).collect();
    let said: Vec<String> = plain.iter().map(|note| note.said()).collect();
    assert_eq!(
        said,
        [
            "Faster search A wish in a state this app cannot draw yet: paused. Its 3 stars are not drawn.",
            "reminders Water the plants"
        ]
    );
    for note in plain {
        assert_eq!(note.roles()[0], "note_title", "a plain note: a title and a body, nothing to tap");
    }
    assert!(placed.iter().any(|piece| piece.kind() == "option"), "what the app does know is still drawn");
}

#[test]
fn both_themes_place_the_same_pieces_in_the_same_order() {
    for size in ["1440x900", "960x640"] {
        let sticker = pieces(&longest(), &["--theme", "sticker", "--size", size]);
        let plain = pieces(&longest(), &["--size", size, "--theme", "plain"]);
        let keys = |placed: &[Piece]| {
            placed.iter().map(|p| (p.kind().to_string(), p.id().to_string(), p.0["index"].clone())).collect::<Vec<_>>()
        };
        assert_eq!(keys(&sticker), keys(&plain));
        assert!(sticker.iter().any(|piece| piece.0["tilt"] != 0.0), "the sticker theme tilts its notes");
        assert!(plain.iter().all(|piece| piece.0["tilt"] == 0.0), "the plain theme is level");
    }
}

#[test]
fn the_same_page_always_prints_the_same_bytes_and_reads_from_stdin() {
    let from_file = layout(&day().to_string(), &[]);
    let from_stdin = support::run(&["layout", "--page", "-"], Some(&day().to_string()));
    assert!(from_file.status.success() && from_stdin.status.success());
    assert!(!from_file.stdout.is_empty());
    assert_eq!(from_file.stdout, from_stdin.stdout);
    assert!(from_file.stderr.is_empty(), "a good run says nothing on stderr");
}

#[test]
fn a_wrong_command_line_exits_2_with_the_usage_and_prints_nothing() {
    let page = day().to_string();
    let wrong: [&[&str]; 10] = [
        &[],
        &["layout"],
        &["draw", "--page", "-"],
        &["layout", "--page", "-", "--theme", "neon"],
        &["layout", "--page", "-", "--size", "960"],
        &["layout", "--page", "-", "--size", "0x640"],
        &["layout", "--page", "-", "--size", "959x640"],
        &["layout", "--page", "-", "--size", "960x639"],
        &["layout", "--page", "-", "--bogus"],
        &["layout", "--page"],
    ];
    for args in wrong {
        let out = support::run(args, Some(&page));
        let said = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {said}");
        assert!(said.starts_with("jellymoon-app: ") && said.contains("usage:"), "{args:?}: {said}");
        assert!(out.stdout.is_empty(), "{args:?} printed something");
    }
}

#[test]
fn a_file_that_is_not_page_data_is_refused() {
    let mut no_read = day();
    no_read.as_object_mut().unwrap().remove("read");
    let mut wrong_type = day();
    wrong_type["wishes"][0]["stars"] = json!("retry added");
    let mut no_label = day();
    no_label["wishes"][0].as_object_mut().unwrap().remove("label");
    let mut odd_state = day();
    odd_state["wishes"][0]["state"] = json!(7);
    let mut odd_clash = day();
    odd_clash["clashes"][0]["settled"] = json!("yes");
    let texts = [
        String::new(),
        "not json".to_string(),
        "[]".to_string(),
        r#"{"day":"2026-03-10"}"#.to_string(),
        no_read.to_string(),
        wrong_type.to_string(),
        no_label.to_string(),
        odd_state.to_string(),
        odd_clash.to_string(),
    ];
    for text in texts {
        let said = refused(&text);
        assert!(said.starts_with("jellymoon-app: the file is not page data: "), "{text}: {said}");
        assert_eq!(said.lines().count(), 1, "one line, and no usage: {said}");
    }
    let missing = support::run(&["layout", "--page", "no-such-file.json"], None);
    assert_eq!(missing.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&missing.stderr).starts_with("jellymoon-app: reading no-such-file.json: "));
    assert!(missing.stdout.is_empty());
}
