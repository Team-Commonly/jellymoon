//! Which project a heard wish is for, and what core refuses when the window has passed: nothing is kept, and the
//! line carries core's reason.

use std::fs;
use std::time::{Duration, Instant, SystemTime};

use serde_json::json;

use super::*;
use crate::support::{hub_key, shows};

/// A hold whose heard wish has had its four seconds.
fn held_and_waited() -> Vec<String> {
    let mut events = hold(0, SAID);
    events.push(tick(5000));
    events
}

impl World {
    /// Makes a wish for `project` on the page's entry, as the person does by hand.
    fn typed_wish(&self, words: &str, project: &str) {
        let made = printed(&self.run(&["wish", "--date", DAY, "--say", words, "--project", project]));
        assert_eq!(made["wish"]["project"], project);
    }
}

#[test]
fn with_one_project_offered_it_is_the_project_and_with_none_nothing_is_kept() {
    let w = heard_world();
    // No session log at all, then a logs folder with nothing typed on the day.
    for logs in [false, true] {
        if logs {
            fs::create_dir_all(w.logs()).unwrap();
        }
        let before = w.home();
        assert_eq!(w.hear(&[], &held_and_waited()), [quiet(), listening(""), cannot(NO_PROJECT)], "logs: {logs}");
        assert!(w.home() == before && !w.at("jm").exists());
    }
    w.typed("alpha");
    assert_eq!(w.hear(&[], &hold(0, SAID)).last(), Some(&heard(SAID, "alpha", 4000)));
}

#[test]
fn with_more_than_one_offered_it_is_the_day_s_last_made_wish_s() {
    let w = heard_world();
    for name in ["alpha", "beta", "gamma"] {
        w.typed(name);
    }
    // Nothing to choose by yet.
    assert_eq!(w.hear(&[], &held_and_waited()), [quiet(), listening(""), cannot(NO_PROJECT)]);
    assert!(!w.at("jm").exists());
    w.typed_wish("the first one", "gamma");
    w.typed_wish("the second one", "beta");
    let printed = w.hear(&[], &held_and_waited());
    assert_eq!(printed[2..], [heard(SAID, "beta", 4000), with(quiet(), "made", json!("m3"))]);
    assert_eq!(w.made_wishes().unwrap()[2], made("m3", SAID, "beta"));
    // It is asked at each heard wish: a wish made on the page since is the day's last.
    w.typed_wish("the fourth one", "alpha");
    assert_eq!(w.hear(&[], &hold(0, SAID)).last(), Some(&heard(SAID, "alpha", 4000)));
}

#[test]
fn a_last_made_wish_s_project_that_is_no_longer_offered_is_not_the_project() {
    let w = heard_world();
    w.typed("alpha");
    w.typed("beta");
    w.typed_wish("the first one", "beta");
    // Switched off by name: the only one left is the project.
    assert_eq!(w.hear(&["--skip", "beta"], &hold(0, SAID)).last(), Some(&heard(SAID, "alpha", 4000)));
    // Never typed in: with two others offered there is nothing to choose by.
    w.typed("gamma");
    w.typed_wish("the second one", "delta");
    assert_eq!(w.hear(&[], &hold(0, SAID)).last(), Some(&cannot(NO_PROJECT)));
    // Every folder switched off: none.
    assert_eq!(w.hear(&["--skip", "alpha,beta,gamma"], &hold(0, SAID)).last(), Some(&cannot(NO_PROJECT)));
}

// A folder's name is a session log's text. One that could not be a wish's project is never offered or printed.
#[test]
fn a_folder_whose_name_is_no_project_s_is_not_offered() {
    let w = heard_world();
    let key = hub_key();
    for (n, name) in [key.as_str(), "al\u{2065}pha", "be\nta", &"p".repeat(101)].into_iter().enumerate() {
        w.typed_at(&format!("-odd-{n}"), &w.at("work").join(name));
    }
    let ran = w.hear_ran(&[], &held_and_waited());
    assert_eq!(lines(&ran).last(), Some(&cannot(NO_PROJECT)));
    w.typed("alpha");
    // The same folder typed in under two sessions is one project.
    w.typed_at("-work-alpha-again", &w.at("work").join("alpha"));
    let ran = w.hear_ran(&[], &held_and_waited());
    assert_eq!(lines(&ran)[2..], [heard(SAID, "alpha", 4000), with(quiet(), "made", json!("m1"))]);
    assert!(!shows(&ran.stdout, &key));
}

#[test]
fn a_ninth_wish_is_not_kept_and_the_line_says_why() {
    let w = alpha_world();
    for n in 1..=8 {
        w.typed_wish(&format!("wish number {n}"), "alpha");
    }
    let before = w.home();
    let full = "the page of 2026-03-10 has its 8 made wishes already, so this one was not kept";
    let want = [quiet(), listening(""), heard(SAID, "alpha", 4000), cannot(full)];
    assert_eq!(w.hear(&["--run"], &held_and_waited()), want);
    assert!(w.home() == before && w.starts() == 0, "nothing was kept and nothing was started");
}

#[test]
fn on_a_day_that_is_locked_nothing_is_kept_and_the_line_says_why() {
    let w = alpha_world();
    w.typed_wish("the one before", "alpha");
    let lock = w.jm("pages").join(format!("{DAY}.ruled.lock"));
    fs::write(&lock, "").unwrap();
    fs::File::options().write(true).open(&lock).unwrap().set_modified(SystemTime::now()).unwrap();
    let (before, started) = (w.home(), Instant::now());
    let locked = "another change to the page of 2026-03-10 has not finished, so this wish was not kept";
    let want = [quiet(), listening(""), heard(SAID, "alpha", 4000), cannot(locked)];
    assert_eq!(w.hear(&[], &held_and_waited()), want);
    assert!(started.elapsed() >= Duration::from_secs(3), "the lock was not waited for");
    assert!(w.home() == before, "nothing was kept");
}

#[test]
fn made_wishes_that_cannot_be_used_are_not_added_to() {
    let w = alpha_world();
    w.typed_wish("the one before", "alpha");
    let kept = w.jm("pages").join(format!("{DAY}.wishes.json"));
    fs::write(&kept, json!({"day": DAY, "wishes": [], "more": 1}).to_string()).unwrap();
    let before = w.home();
    let unused = "the made wishes of 2026-03-10 are not used: they are not the made wishes of that day; this wish \
                  was not kept";
    assert_eq!(w.hear(&[], &held_and_waited()).last(), Some(&cannot(unused)));
    assert!(w.home() == before);
}

#[test]
fn a_day_that_is_none_and_a_home_that_is_no_full_path_begin_no_hearing() {
    let w = alpha_world();
    let logs = w.logs();
    let ran = w.run(&["hear", "--date", "2026-3-10", "--logs", logs.to_str().unwrap()]);
    assert_eq!(refused(&ran), "jellymoon: --date must be a day as YYYY-MM-DD\n");
    let ran =
        w.run_with(&[("JELLYMOON_HOME", "jm".into())], &["hear", "--date", DAY, "--logs", logs.to_str().unwrap()]);
    assert_eq!((ran.code, ran.stdout.as_str()), (1, ""), "{}", ran.stderr);
    for args in [&["hear", "--wish", "m1"][..], &["hear", "now"], &["hear", "--logs", " "]] {
        let ran = w.run(args);
        assert!(ran.code == 2 && ran.stdout.is_empty() && ran.stderr.contains("usage:"), "{args:?}");
    }
}
