//! F34, the kept page: when it is shown, when it is read again, and how it is kept. The built `jellymoon-app` is
//! run as in `f34.rs`; one case follows the window's own steps through `today`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::{env, fs};

use jellymoon_app::read::MAX_PAGE_BYTES;
use jellymoon_app::today::Today;
use sticker_kit::{Note, Scene};
use support::day::*;

#[test]
fn a_kept_page_comes_out_when_the_logs_cannot_be_read_with_a_note_that_says_so() {
    let world = world();
    let first = notes(&shown(&world, &[]));
    assert!(called(&world));
    let nowhere = world.dir.path().join("nowhere");
    let placed = shown(&world, &["--logs", nowhere.to_str().unwrap()]);
    assert_eq!(labels(&placed), ["Fail fast", "Retry uploads"]);
    let said = notes(&placed);
    assert!(
        said[0].starts_with("This page is from an earlier reading. No Claude Code session logs were found"),
        "{said:?}"
    );
    assert_eq!(said[1..3], first[..2], "the page under the note; its last note no longer fits");
    assert!(!called(&world));
}

// The order the window follows, step by step: the kept page before any log is opened, then the count, then the call.
// These steps run in this process, whose time zone a test cannot set. So the day's lines are stamped at noon of the
// day in this system's zone, whichever it is, and the program that keeps the first page is run in that zone too
// and not in the world's: at noon UTC the test read nothing from twelve hours ahead of UTC on.
#[test]
fn a_kept_page_is_shown_before_the_logs_are_opened_and_stays_with_a_note_while_the_day_is_read_again() {
    let mut world = world();
    world.vars.retain(|(name, _)| *name != "TZ");
    world.vars.extend(env::var_os("TZ").map(|zone| ("TZ", zone)));
    let noon = |second: i8, project: &str, text: &str| {
        let day: jiff::civil::Date = DAY.parse().unwrap();
        let at = day.at(12, 0, second, 0).to_zoned(jiff::tz::TimeZone::system()).unwrap().timestamp();
        said(&at.to_string(), &format!("/work/{project}"), text)
    };
    write_log(&world, "-work-alpha/s1.jsonl", &[noon(0, "alpha", FIRST), noon(1, "alpha", SECOND)]);
    write_log(&world, "-work-private-notes/s2.jsonl", &[noon(2, "private-notes", "dear diary")]);
    shown(&world, &[]);
    assert!(called(&world));
    let logs = world.dir.path().join("logs");
    let today = Today {
        date: DAY.to_string(),
        logs: world.dir.path().join("nowhere"),
        skip: Vec::new(),
        model: "sonnet".to_string(),
        home: world.home.clone(),
        harness: Some(world.rec.join(format!("claude{}", env::consts::EXE_SUFFIX))),
        fresh: true,
        settings: None,
        bin: None,
    };
    let title = |scene: &Scene| match &scene.notes[0] {
        Note::Plain { title, .. } => title.clone(),
        _ => "not a plain note".to_string(),
    };
    let (first, kept) = today.first();
    assert!(kept.is_some() && title(&first) != "Reading your day…", "the kept page, with no log to read");

    let counted = Today { logs, ..today }.later(kept)().expect("asked to read again");
    assert_eq!(title(&counted.scene), "Reading your day again…");
    assert_eq!(counted.scene.notes[1..], first.notes[..], "the kept page stays on show under the note");
    assert!(!called(&world), "counting the turns makes no call");
    let read = counted.later.expect("the reading call is the next step")().expect("a page, or why there is none");
    assert!(read.later.is_none() && title(&read.scene) != "Reading your day again…");
    assert!(called(&world), "the day was read again");
}

#[test]
fn a_kept_file_that_is_not_a_page_or_is_too_large_is_read_again_and_never_drawn() {
    let world = world();
    shown(&world, &[]);
    assert!(called(&world));
    let good = fs::read_to_string(kept(&world)).unwrap();
    // The second is page data that saw every turn, behind more blank space than a page may have.
    for bad in ["{\"day\": \"2026-03-10\"".to_string(), " ".repeat(MAX_PAGE_BYTES) + &good] {
        fs::write(kept(&world), &bad).unwrap();
        assert_eq!(labels(&shown(&world, &[])), ["Fail fast", "Retry uploads"]);
        assert!(called(&world), "a kept file of {} bytes was not read again", bad.len());
        assert_eq!(fs::read_to_string(kept(&world)).unwrap(), good, "the new reading replaced it");
    }
    assert_eq!(names(&world.home.join("pages")), [format!("{DAY}.json")], "nothing is left beside the kept page");
}

#[cfg(unix)]
#[test]
fn the_kept_pages_are_for_this_user_only() {
    use std::os::unix::fs::PermissionsExt;
    let world = world();
    shown(&world, &[]);
    let mode = |path: &std::path::Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!((mode(&world.home.join("pages")), mode(&kept(&world))), (0o700, 0o600));
}

#[test]
fn a_home_that_is_not_an_absolute_path_is_refused() {
    let world = world();
    let user = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    for vars in [&[("JELLYMOON_HOME", "jm")][..], &[("JELLYMOON_HOME", ""), (user, "user")]] {
        let out = run(&world, &[], vars);
        let said = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{vars:?}: {said}");
        assert!(said.starts_with("jellymoon-app: no home directory: set JELLYMOON_HOME"), "{vars:?}: {said}");
        assert!(out.stdout.is_empty() && !called(&world));
    }
}
