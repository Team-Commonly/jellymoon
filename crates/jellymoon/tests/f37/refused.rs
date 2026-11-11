//! What `jellymoon wish` refuses, each in one line with nothing written, and what the day's lock does to it.

use std::fs;
use std::time::{Duration, Instant, SystemTime};

use serde_json::{Value, json};

use super::{DAY, SAID, World, made, world};
use crate::support::{hub_key, tree};

const NAME: &str = "jellymoon: --project takes one folder's name of at most 100 characters: no path, nothing that \
                    cannot be seen and nothing shaped like a credential\n";
const EMPTY: &str = "jellymoon: a wish needs words, and nothing is left of these once cleaned\n";
const LONG: &str = "jellymoon: a wish is at most 200 characters, and this one is longer\n";

impl World {
    fn refused(&self, case: &str, ran: &crate::Ran, line: &str, before: &[(std::path::PathBuf, Vec<u8>)]) {
        assert_eq!((ran.code, ran.stdout.as_str(), ran.stderr.as_str()), (1, "", line), "{case}");
        assert!(tree(self.dir.path()) == before, "{case}: nothing was written");
    }

    /// The day's lock, which is the tap's.
    fn lock(&self) -> std::path::PathBuf {
        self.pages().join(format!("{DAY}.ruled.lock"))
    }

    fn leave_lock(&self, at: SystemTime) {
        fs::create_dir_all(self.pages()).unwrap();
        fs::write(self.lock(), "").unwrap();
        fs::File::options().write(true).open(self.lock()).unwrap().set_modified(at).unwrap();
    }
}

#[test]
fn a_wish_with_no_words_too_many_words_or_no_plain_project_is_refused_and_writes_nothing() {
    let (fits, key_name) = ("word ".repeat(39) + "words", hub_key());
    let long = format!("{fits}x");
    let many = "é".repeat(5000);
    let cases: [(&str, &str, &str, &str); 12] = [
        ("no words", "", "alpha", EMPTY),
        ("only spaces", " \n\t ", "alpha", EMPTY),
        ("only what cannot be seen", "\u{2065}\u{200b}\u{feff}", "alpha", EMPTY),
        ("201 characters", &long, "alpha", LONG),
        ("far too long", &many, "alpha", LONG),
        ("no project", SAID, "", NAME),
        ("a path", SAID, "work/alpha", NAME),
        ("a Windows path", SAID, "work\\alpha", NAME),
        ("the folder above", SAID, "..", NAME),
        ("a name with a character that cannot be seen", SAID, "al\u{2065}pha", NAME),
        ("a name with a line break", SAID, "alpha\nbeta", NAME),
        ("a name shaped like a credential", SAID, &key_name, NAME),
    ];
    let w = world();
    w.wish("the one before", "alpha");
    let before = tree(w.dir.path());
    for (case, words, project, line) in cases {
        w.refused(case, &w.say(words, project), line, &before);
    }
    let ran = w.run(&["wish", "--date", "2026-3-10", "--say", SAID, "--project", "alpha"]);
    w.refused("a day that is not one", &ran, "jellymoon: --date must be a day as YYYY-MM-DD\n", &before);
    for args in [&["wish", "--say", SAID][..], &["wish", "--project", "alpha"], &["wish", "alpha"]] {
        let ran = w.run(args);
        assert!(ran.code == 2 && ran.stdout.is_empty() && ran.stderr.contains("usage:"), "{args:?}");
        assert!(tree(w.dir.path()) == before, "{args:?}");
    }
    assert_eq!(w.wish(&fits, &"p".repeat(39))["wish"]["id"], "m2", "200 characters are a wish");
}

#[test]
fn a_day_holds_eight_made_wishes_and_the_ninth_is_refused() {
    let w = world();
    for n in 1..=8 {
        assert_eq!(w.wish(&format!("wish number {n}"), "alpha")["wish"]["id"], format!("m{n}"));
    }
    // A folder the person opened to others stays as they left it: a refusal changes no mode either.
    #[cfg(unix)]
    fs::set_permissions(w.pages(), std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    let before = tree(w.dir.path());
    let line = "jellymoon: the page of 2026-03-10 has its 8 made wishes already, so this one was not kept\n";
    w.refused("the ninth", &w.say("one too many", "alpha"), line, &before);
    #[cfg(unix)]
    {
        let mode = std::os::unix::fs::PermissionsExt::mode(&fs::metadata(w.pages()).unwrap().permissions());
        assert_eq!(mode & 0o777, 0o755, "the folder's mode is as it was");
    }
    assert_eq!(w.page().len(), 8);
}

/// What a test leaves where the made wishes are kept.
type Spoil = Box<dyn Fn(&World)>;

/// Leaves this text there, for its owner alone to write, whatever the umask the tests run under.
fn text(text: String) -> Spoil {
    Box::new(move |w| {
        fs::write(w.wishes(), &text).unwrap();
        #[cfg(unix)]
        fs::set_permissions(w.wishes(), std::os::unix::fs::PermissionsExt::from_mode(0o644)).unwrap();
    })
}

fn wishes(day: &str, wishes: Value) -> Spoil {
    text(json!({"day": day, "wishes": wishes}).to_string())
}

#[cfg(unix)]
fn link_it(w: &World) {
    let elsewhere = w.dir.path().join("elsewhere.json");
    fs::rename(w.wishes(), &elsewhere).unwrap();
    std::os::unix::fs::symlink(&elsewhere, w.wishes()).unwrap();
}

#[cfg(unix)]
fn open_it(w: &World) {
    fs::set_permissions(w.wishes(), std::os::unix::fs::PermissionsExt::from_mode(0o620)).unwrap();
}

fn folder(w: &World) {
    fs::remove_file(w.wishes()).unwrap();
    fs::create_dir(w.wishes()).unwrap();
}

fn spoiled() -> Vec<(&'static str, Spoil, &'static str)> {
    let shape = "they are not the made wishes of that day";
    let cases = vec![
        ("not JSON", text("{\"day\"".to_string()), shape),
        ("of another day", wishes("2026-03-11", json!([])), shape),
        ("a field nobody knows", text(json!({"day": DAY, "wishes": [], "more": 1}).to_string()), shape),
        ("an id of the reading's", wishes(DAY, json!([made("w1", SAID, "alpha")])), shape),
        ("an id out of its place", wishes(DAY, json!([made("m2", SAID, "alpha")])), shape),
        ("words that are not clean", wishes(DAY, json!([made("m1", "two\nlines", "alpha")])), shape),
        ("a project that is a path", wishes(DAY, json!([made("m1", SAID, "../alpha")])), shape),
        (
            "nine of them",
            wishes(DAY, Value::Array((1..=9).map(|n| made(&format!("m{n}"), SAID, "alpha")).collect())),
            shape,
        ),
        ("over 64 KiB", text(" ".repeat(64 * 1024 + 1)), "it is too large"),
        ("a folder", Box::new(folder) as Spoil, "it is not a plain file"),
    ];
    #[cfg(unix)]
    let cases = cases
        .into_iter()
        .chain([
            ("a link", Box::new(link_it) as Spoil, "it is not a plain file"),
            ("a file its group can write to", Box::new(open_it) as Spoil, "others can write to it"),
        ])
        .collect();
    cases
}

// A file that is not Jellymoon's own made wishes is never drawn and never written over.
#[test]
fn made_wishes_that_cannot_be_used_are_not_drawn_and_a_new_wish_is_refused() {
    for (case, spoil, why) in spoiled() {
        let w = world();
        w.wish(SAID, "alpha");
        spoil(&w);
        let before = tree(w.dir.path());
        assert_eq!(w.page(), Vec::<Value>::new(), "{case}: nothing of it is drawn");
        let line = format!("jellymoon: the made wishes of 2026-03-10 are not used: {why}; this wish was not kept\n");
        w.refused(case, &w.say("one more", "alpha"), &line, &before);
    }
}

#[test]
fn a_lock_left_behind_long_ago_is_taken_over_and_a_fresh_one_refuses_the_wish_after_three_seconds() {
    let w = world();
    w.leave_lock(SystemTime::now() - Duration::from_secs(60));
    let started = Instant::now();
    assert_eq!(w.wish(SAID, "alpha")["wish"]["id"], "m1");
    assert!(started.elapsed() < Duration::from_secs(2) && !w.lock().exists(), "taken with no wait, and removed");

    w.leave_lock(SystemTime::now());
    let (before, started) = (tree(w.dir.path()), Instant::now());
    let ran = w.say("one more", "alpha");
    let waited = started.elapsed();
    let line = "jellymoon: another change to the page of 2026-03-10 has not finished, so this wish was not kept\n";
    w.refused("a fresh lock", &ran, line, &before);
    assert!(waited >= Duration::from_secs(3) && waited < Duration::from_secs(8), "waited {waited:?}");
}
