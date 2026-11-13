//! A file's name is data: one that looks like an option, or holds a line break, a quote, a colon or characters
//! nobody sees, reaches git and the page only as a name. And the list's own bounds: the count it is cut at, and
//! work of ten thousand files.

use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::{Entry, WORK, finished, listed, printed, refused};

#[test]
fn a_name_that_looks_like_an_option_or_holds_what_nobody_sees_is_only_ever_a_name() {
    let made = finished();
    let holds = |n: usize| format!("{WORK} in file {n}\n").into_bytes();
    let held: Vec<Vec<u8>> = (1..=9).map(holds).collect();
    let names = [
        "--output=stolen",
        "-rf",
        ".cla\u{200b}ude/hooks.json",
        "HEAD:a.txt",
        "a\nb.txt",
        "co:lon \"quoted\" 'single'.txt",
        "main",
        "red\u{1b}[31m\u{202e}txt.exe",
        "tab\tand  spaces",
    ];
    let entries: Vec<Entry> = names.iter().zip(&held).map(|(name, held)| ("100644", *name, &held[..])).collect();
    made.rework(&entries);
    let before = made.held();
    let ran = made.show(&[]);
    let shown = printed(&ran);
    let size = held[0].len();
    let files = json!([
        listed(1, "--output=stolen", "made", size, "text", false),
        listed(2, "-rf", "made", size, "text", false),
        listed(3, ".claude/hooks.json", "made", size, "text", true),
        listed(4, "HEAD:a.txt", "made", size, "text", false),
        listed(5, "a b.txt", "made", size, "text", false),
        listed(6, "co:lon 'quoted' 'single'.txt", "made", size, "text", false),
        listed(7, "main", "made", size, "text", false),
        listed(8, "redtxt.exe", "made", size, "text", false),
        listed(9, "tab and spaces", "made", size, "text", false),
    ]);
    assert_eq!(shown["files"], files);
    for raw in ['\u{1b}', '\u{202e}', '\u{200b}'] {
        assert!(!ran.stdout.contains(raw) && !ran.stdout.contains(&format!("\\u{:04x}", raw as u32)), "{}", ran.stdout);
    }
    // Each number names its own file, whatever the name would mean to git.
    for n in 1..=9 {
        let file = printed(&made.show(&["--file", &n.to_string()]));
        assert_eq!((&file["text"], &file["name"]), (&json!(format!("{WORK} in file {n}")), &files[n - 1]["name"]));
    }
    assert_eq!(made.held(), before);
    assert!(!made.w.at("stolen").exists() && !made.repo.join("stolen").exists());
}

#[test]
fn a_long_name_is_cut_and_a_name_shaped_like_a_credential_is_not_shown() {
    let made = finished();
    let (long, key) = (format!("deep/{}.txt", "n".repeat(300)), crate::support::hub_key());
    made.rework(&[("100644", &long, b"x\n"), ("100644", &format!("keys/{key}"), b"x\n")]);
    let ran = made.show(&[]);
    let shown = printed(&ran);
    let name = shown["files"][0]["name"].as_str().unwrap();
    assert_eq!((name.chars().count(), name.ends_with('…')), (200, true), "{name}");
    assert_eq!(shown["files"][1]["name"], "keys/[redacted]");
    assert!(!crate::support::shows(&ran.stdout, &key));
}

/// Work of `count` files over `main`, each a few bytes.
fn many(count: usize) -> (super::Made, Vec<String>) {
    let made = finished();
    let names: Vec<String> = (0..count).map(|n| format!("many/file-{n:05}.txt")).collect();
    let entries: Vec<Entry> = names.iter().map(|name| ("100644", name.as_str(), &b"one of many\n"[..])).collect();
    made.rework(&entries);
    (made, names)
}

#[test]
fn the_list_is_cut_at_two_hundred_files_and_says_how_many_more_there_are() {
    for (count, more) in [(200, 0), (201, 1), (205, 5)] {
        let (made, names) = many(count);
        let shown = printed(&made.show(&[]));
        let files = shown["files"].as_array().unwrap();
        assert_eq!((files.len(), &shown["more"], &shown["most"]), (200, &json!(more), &json!(200)), "{count}");
        assert_eq!(
            (&files[0]["n"], &files[199]["n"], &files[199]["name"]),
            (&json!(1), &json!(200), &json!(names[199]))
        );
        let last = printed(&made.show(&["--file", "200"]));
        assert_eq!((&last["name"], &last["text"]), (&json!(names[199]), &json!("one of many")));
        // A file past the cut has no number: the list is what a number is read against.
        assert!(refused(&made.show(&["--file", "201"])).contains("names no file"));
    }
}

#[test]
fn a_list_of_names_longer_than_is_read_is_refused_and_not_cut_in_the_middle_of_a_name() {
    let made = finished();
    // Six thousand names of some 3,000 bytes each: more than the 16 MiB of git's list that are read.
    // The folders are named in a letter no run id holds: an id is sixteen hex digits, and one in three hundred
    // has `ddd` in it, which failed this test on a day nothing was wrong.
    let deep = [&"z".repeat(240)[..]; 12].join("/");
    let names: Vec<String> = (0..6000).map(|n| format!("{deep}/file-{n:04}.txt")).collect();
    let entries: Vec<Entry> = names.iter().map(|name| ("100644", name.as_str(), &b"one of many\n"[..])).collect();
    made.rework(&entries);
    let before = made.held();
    for more in [&[][..], &["--file", "1"]] {
        let said = made.show(more);
        assert!(refused(&said).contains("is more files than Jellymoon lists"), "{}", said.stderr.len());
        assert!(!said.stderr.contains("zzz"));
    }
    assert_eq!(made.held(), before);
}

#[test]
fn work_of_ten_thousand_files_is_answered_in_time() {
    let (made, _) = many(10_000);
    let before = made.held();
    let started = Instant::now();
    let shown: Value = printed(&made.show(&[]));
    assert!(started.elapsed() < Duration::from_secs(60), "{:?}", started.elapsed());
    assert_eq!((shown["files"].as_array().unwrap().len(), &shown["more"]), (200, &json!(9800)));
    assert_eq!(made.held(), before);
}
