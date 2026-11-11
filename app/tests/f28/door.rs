//! F28 at the door: page data is untrusted, so the app holds a file to a size and each list to a length, and whatever
//! it or the kit leaves out is counted in the printout.

use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::support::{self, Piece, clash, layout, page, pieces, refused, wish};

const MEBIBYTE: usize = 1024 * 1024;

/// A day of four wishes, six stars and `clashes` clashes.
fn day(clashes: usize) -> Value {
    let mut day = super::day();
    day["clashes"] =
        (0..clashes).map(|i| clash("w1", "w2", &format!("do {i}"), &format!("never do {i}"), false)).collect();
    day
}

/// The notes placed, by id, and what the kit's last sheet says when it left notes out.
fn notes(placed: &[Piece]) -> (Vec<&str>, Option<String>) {
    let notes: Vec<&Piece> = placed.iter().filter(|piece| piece.kind() == "note").collect();
    let more = notes.iter().find(|note| note.id() == "more").map(|note| note.said());
    (notes.iter().map(|note| note.id()).filter(|id| *id != "more").collect(), more)
}

#[test]
fn a_page_file_over_one_mebibyte_is_refused_with_one_line() {
    let page = day(1).to_string();
    let padded = |bytes: usize| format!("{page}{}", " ".repeat(bytes - page.len()));
    let line = "jellymoon-app: the file is not page data: it is over 1 MiB\n";
    assert_eq!(refused(&padded(MEBIBYTE + 1)), line);
    let piped = support::run(&["layout", "--page", "-"], Some(&padded(8 * MEBIBYTE)));
    assert_eq!((piped.status.code(), String::from_utf8_lossy(&piped.stderr).as_ref()), (Some(1), line));
    assert!(piped.stdout.is_empty());
    let whole = layout(&padded(MEBIBYTE), &[]);
    assert!(whole.status.success(), "a file of exactly the size is read");
    assert_eq!(whole.stdout, layout(&page, &[]).stdout);
}

#[test]
fn a_page_with_thousands_of_everything_comes_back_fast_and_says_what_it_left_out() {
    let many = ["one"; 3000];
    let wishes = (0..2000).map(|i| {
        let state = if i % 2 == 0 { "working" } else { "paused" };
        wish(&format!("w{i}"), &format!("Wish {i}"), state, if i == 0 { &many[..] } else { &[] })
    });
    let mut huge = page(wishes.collect(), day(2000)["clashes"].as_array().unwrap().clone());
    huge["reminders"] = (0..3000).map(|i| json!({ "label": format!("Reminder {i}") })).collect();
    huge["later"] = json!("one more");
    assert!(huge.to_string().len() < MEBIBYTE);

    for theme in ["sticker", "plain"] {
        let start = Instant::now();
        let placed = pieces(&huge, &["--theme", theme]);
        assert!(start.elapsed() < Duration::from_secs(5), "{theme}: {:?}", start.elapsed());
        assert_eq!(placed.iter().filter(|piece| piece.kind() == "character").count(), 12, "24 wishes, 12 drawable");
        assert!(placed.len() < 300, "{} pieces", placed.len());
        let first = placed.iter().find(|piece| piece.kind() == "note").unwrap().said();
        assert_eq!(
            first,
            "not everything is drawn Left out of this page: 1976 wishes, 1992 clashes, 2988 stars, 2993 things in parts \
             this app does not know."
        );
        // 1 note for what was left out, the list, 8 clashes, 12 paused wishes and 8 things from unknown parts.
        let (drawn, more) = notes(&placed);
        assert_eq!(more, Some(format!("+{} more of 30 notes", 30 - drawn.len())), "{theme}: {drawn:?}");
    }
}

#[test]
fn three_clashes_and_the_list_note_are_each_drawn_or_counted() {
    let all = ["note-0", "clash-1", "clash-2", "clash-3"];
    let mut counted = 0;
    for size in ["1440x900", "960x640"] {
        for theme in ["sticker", "plain"] {
            let placed = pieces(&day(3), &["--theme", theme, "--size", size]);
            let (drawn, more) = notes(&placed);
            assert!(!drawn.is_empty(), "{theme} {size}");
            assert_eq!(drawn, all[..drawn.len()], "{theme} {size}: the first notes in page order, none skipped");
            let left_out = all.len() - drawn.len();
            let want = (left_out > 0).then(|| format!("+{left_out} more of 4 notes"));
            assert_eq!(more, want, "{theme} {size}: {drawn:?}");
            counted += usize::from(left_out > 0);

            let list = placed.iter().find(|piece| piece.kind() == "note").unwrap().said();
            assert!(list.starts_with("came true today ★ retry added"), "{theme} {size}: {list}");
            let rows = list.matches('★').count();
            assert!(rows == 6 || list.ends_with(&format!("+{} more", 6 - rows)), "{theme} {size}: {list}");
            let paper = placed.iter().find(|piece| piece.kind() == "paper").unwrap().rect();
            for note in placed.iter().filter(|piece| piece.kind() == "note" && piece.0["tilt"] == 0.0) {
                let [x, y, w, h] = note.rect();
                assert!(y >= paper[1] && y + h <= paper[1] + paper[3] && x + w <= paper[0] + paper[2], "{note:?}");
            }
        }
    }
    assert!(counted > 0, "no size or theme left a note out, so the count was never looked at");
}

#[test]
fn a_page_with_no_wishes_but_a_part_the_app_does_not_know_does_not_say_nothing_was_wished() {
    let mut empty = page(vec![], vec![]);
    empty["reminders"] = json!(["Water the plants"]);
    let placed = pieces(&empty, &[]);
    let said: Vec<String> = placed.iter().filter(|piece| piece.kind() == "note").map(Piece::said).collect();
    assert_eq!(said, ["reminders Water the plants"]);
}

/// Two quotes of 200 characters each, three ways: words, a script written without spaces, and one unbroken word.
fn quote_pairs() -> [(&'static str, [String; 2]); 3] {
    let long = |unit: &str| unit.chars().cycle().take(200).collect::<String>();
    let words = [
        "please make the uploader retry three times before it gives up on a file, and wait a little longer \
         between each try so that a slow connection is not hammered, and tell me in the log what happened there",
        "never retry an upload, fail fast and loudly on the first error so that I can see what went wrong at \
         once, because a quiet retry hid the broken token from me for a whole week the last time we tried it.",
    ];
    [
        ("words", words.map(str::to_string)),
        (
            "no spaces",
            [
                long("上传失败时请重试三次再放弃并把经过写进日志。"),
                long("永远不要重试上传，第一次出错就立刻大声失败。"),
            ],
        ),
        ("one word", [long("https://example.test/retry/three-times/then-give-up/"), long("never-retry_fail-fast_")]),
    ]
}

/// The fullest page core can write: 8 wishes with labels of 40, 4 stars of 60 each, 2 clashes with quotes of 200.
fn fullest(quotes: &[String; 2]) -> Value {
    let star = "Wrote the retry budget and proved it on a slow connection...";
    let stars = [star; 4];
    let wishes = (1..=8).map(|i| {
        let state = ["working", "done", "needs_you", "let_go"][i % 4];
        wish(&format!("w{i}"), "Rebuild the uploader with a retry budget", state, &stars)
    });
    assert_eq!((star.chars().count(), quotes[0].chars().count(), quotes[1].chars().count()), (60, 200, 200));
    let clashes =
        vec![clash("w1", "w2", &quotes[0], &quotes[1], true), clash("w3", "w4", &quotes[1], &quotes[0], false)];
    page(wishes.collect(), clashes)
}

/// The lines of each option of the choice note `id`.
fn option_lines(placed: &[Piece], id: &str) -> Vec<Vec<String>> {
    let options = placed.iter().filter(|piece| piece.kind() == "option" && piece.id() == id);
    options
        .map(|option| option.runs().iter().map(|run| run["text"].as_str().unwrap_or_default().to_string()).collect())
        .collect()
}

#[test]
fn the_fullest_page_core_can_write_is_drawn_whole() {
    for (kind, quotes) in quote_pairs() {
        let full = fullest(&quotes);
        for (size, theme) in
            [("1440x900", "sticker"), ("1440x900", "plain"), ("960x640", "sticker"), ("960x640", "plain")]
        {
            let at = format!("{kind} {theme} {size}");
            let placed = pieces(&full, &["--theme", theme, "--size", size]);
            let count = |kind: &str| placed.iter().filter(|piece| piece.kind() == kind).count();
            assert_eq!((count("character"), count("label"), count("star")), (8, 8, 32), "{at}");
            assert_eq!(notes(&placed), (vec!["note-0", "clash-1", "clash-2"], None), "{at}: nothing is counted");

            for (id, order) in [("clash-1", [0, 1]), ("clash-2", [1, 0])] {
                let options = option_lines(&placed, id);
                assert_eq!(options.len(), 2, "{at} {id}: both options");
                for (lines, quote) in options.iter().zip(order.map(|i| format!("“{}”", quotes[i]))) {
                    assert_eq!(lines.len(), 2, "{at} {id}: two lines beside another clash: {lines:?}");
                    let shown = lines[1].strip_suffix('…').unwrap_or_else(|| panic!("{at}: {lines:?} is not cut"));
                    if kind == "words" {
                        let shown = format!("{} {shown}", lines[0]);
                        assert!(quote.starts_with(&shown) && quote[shown.len()..].starts_with(' '), "{at}: {lines:?}");
                    } else {
                        assert!(quote.starts_with(&format!("{}{shown}", lines[0])), "{at}: {lines:?}");
                        let short = lines.iter().map(|line| line.chars().count()).min().unwrap();
                        assert!(short >= 12, "{at}: a line is left nearly empty: {lines:?}");
                    }
                }
            }
            let list = placed.iter().find(|piece| piece.kind() == "note").unwrap().said();
            let rows = list.matches('★').count();
            assert!(list.starts_with("came true today ★ Wrote the retry budget"), "{at}: {list}");
            assert!(rows >= 1 && list.ends_with(&format!("+{} more", 32 - rows)), "{at}: {list}");
        }
    }
}

#[test]
fn a_quote_with_no_spaces_fills_its_three_lines_like_one_with_words() {
    for (kind, quotes) in quote_pairs() {
        let mut one = fullest(&quotes);
        one["clashes"].as_array_mut().unwrap().truncate(1);
        for theme in ["sticker", "plain"] {
            let placed = pieces(&one, &["--theme", theme]);
            for (lines, quote) in option_lines(&placed, "clash-1").iter().zip(&quotes) {
                assert_eq!(lines.len(), 3, "{kind} {theme}: {lines:?}");
                assert!(lines[2].ends_with('…'), "{kind} {theme}: 200 characters do not fit three lines");
                let shown: usize = lines.iter().map(|line| line.chars().count()).sum();
                assert!(shown * 5 > quote.chars().count(), "{kind} {theme}: only {shown} characters of 200 are shown");
            }
        }
    }
}
