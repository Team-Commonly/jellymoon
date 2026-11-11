//! F37: a typed wish is kept on the day's page in the person's exact words (ADR 0014). `jellymoon wish` is run as a
//! black box in a folder of the test's own, and `jellymoon page` shows what it kept. `f37/refused.rs` holds what is
//! refused and what the day's lock does. Every word here is made up; the harness is `support/fake_claude.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "f37/refused.rs"]
mod refused;
#[path = "f37/still.rs"]
mod still;
mod support;

use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

use serde_json::{Value, json};
use support::{A, Ran, ZONE, hub_key, install, said, scratch, write_log};
use tempfile::TempDir;

const DAY: &str = "2026-03-10";
const SAID: &str = "make the uploader retry twice";
const WHY: &str = "nothing was started: `jellymoon wish` keeps a wish and starts no run";

/// One test's folder: `jm` is Jellymoon's home, `user` the person's, `logs` their session logs, with no session.
pub struct World {
    pub dir: TempDir,
    pub vars: Vec<(&'static str, OsString)>,
}

pub fn world() -> World {
    let dir = scratch();
    let at = |name: &str| dir.path().join(name);
    for folder in ["user", "logs", "bin"] {
        fs::create_dir(at(folder)).unwrap();
    }
    let vars = vec![
        ("TZ", ZONE.into()),
        ("HOME", at("user").into()),
        ("USERPROFILE", at("user").into()),
        ("JELLYMOON_HOME", at("jm").into()),
        ("PATH", at("bin").into()),
        ("JELLYMOON_SYSTEM_BIN", OsString::new()), // no folder of this machine is looked in
    ];
    World { dir, vars }
}

/// A made wish as it is printed and kept.
pub fn made(id: &str, said: &str, project: &str) -> Value {
    json!({"id": id, "said": said, "project": project, "state": "working", "stars": []})
}

impl World {
    pub fn pages(&self) -> PathBuf {
        self.dir.path().join("jm").join("pages")
    }

    pub fn wishes(&self) -> PathBuf {
        self.pages().join(format!("{DAY}.wishes.json"))
    }

    pub fn run(&self, args: &[&str]) -> Ran {
        let vars: Vec<_> = self.vars.iter().map(|(name, value)| (*name, value.as_os_str())).collect();
        support::jellymoon(&vars, args)
    }

    pub fn say(&self, words: &str, project: &str) -> Ran {
        self.run(&["wish", "--date", DAY, "--say", words, "--project", project])
    }

    /// A wish that was kept: the one line it printed.
    pub fn wish(&self, words: &str, project: &str) -> Value {
        let ran = self.say(words, project);
        assert_eq!((ran.code, ran.stderr.as_str()), (0, ""), "wish {words:?}");
        assert!(ran.stdout.ends_with("}\n") && ran.stdout.matches('\n').count() == 1, "{:?}", ran.stdout);
        serde_json::from_str(&ran.stdout).unwrap()
    }

    pub fn kept(&self) -> Value {
        serde_json::from_str(&fs::read_to_string(self.wishes()).unwrap()).unwrap()
    }

    /// The wishes of `jellymoon page` for the day.
    pub fn page(&self) -> Vec<Value> {
        let logs = self.dir.path().join("logs");
        let ran = self.run(&["page", "--date", DAY, "--logs", logs.to_str().unwrap()]);
        assert_eq!((ran.code, ran.stderr.as_str()), (0, ""), "jellymoon page");
        let page: Value = serde_json::from_str(&ran.stdout).unwrap();
        page["wishes"].as_array().unwrap().clone()
    }
}

#[test]
fn a_typed_wish_is_kept_in_the_working_state_and_says_why_nothing_was_started() {
    let w = world();
    let first = w.wish(SAID, "alpha");
    assert_eq!(first, json!({"wish": made("m1", SAID, "alpha"), "run": null, "why": WHY}));
    assert_eq!(w.kept(), json!({"day": DAY, "wishes": [made("m1", SAID, "alpha")]}));
    let second = w.wish("and say so in the log", "beta");
    assert_eq!(second["wish"], made("m2", "and say so in the log", "beta"));
    let both = json!([made("m1", SAID, "alpha"), made("m2", "and say so in the log", "beta")]);
    assert_eq!(w.kept(), json!({"day": DAY, "wishes": both}));
    let names: Vec<_> = fs::read_dir(w.pages()).unwrap().map(|entry| entry.unwrap().file_name()).collect();
    assert_eq!(names, [format!("{DAY}.wishes.json").as_str()], "one file, and no lock is left");
    #[cfg(unix)]
    {
        let mode = |path: &PathBuf| std::os::unix::fs::PermissionsExt::mode(&fs::metadata(path).unwrap().permissions());
        assert_eq!((mode(&w.wishes()) & 0o777, mode(&w.pages()) & 0o777), (0o600, 0o700), "the person's alone");
    }
}

// What is kept is what is drawn: one line, nothing that cannot be seen, no credential, as a clash's statement is.
#[test]
fn a_wish_is_kept_as_it_is_drawn_and_never_holds_a_credential() {
    let w = world();
    let raw = format!("  deploy  with {}\nfr\u{2065}om \"now\" on ", hub_key());
    let clean = "deploy with [redacted] from 'now' on";
    assert_eq!(w.wish(&raw, "alpha")["wish"]["said"], clean);
    let text = fs::read_to_string(w.wishes()).unwrap();
    assert!(!text.contains(&hub_key()) && !text.contains('\u{2065}'), "{text}");
    assert_eq!(w.page()[0]["said"], clean);
}

#[test]
fn a_wish_with_no_date_is_kept_for_today() {
    let w = world();
    let ran = w.run(&["wish", "--say", SAID, "--project", "alpha"]);
    assert_eq!((ran.code, ran.stderr.as_str()), (0, ""));
    let names: Vec<String> =
        fs::read_dir(w.pages()).unwrap().map(|entry| entry.unwrap().file_name().into_string().unwrap()).collect();
    let [name] = names.as_slice() else { panic!("one file is kept: {names:?}") };
    let day = name.strip_suffix(".wishes.json").expect("the made wishes of one day");
    let shaped = day.len() == 10 && day.bytes().all(|byte| byte == b'-' || byte.is_ascii_digit());
    assert!(shaped && day > DAY, "today, not the fixture day: {day}");
    let kept: Value = serde_json::from_str(&fs::read_to_string(w.pages().join(name)).unwrap()).unwrap();
    assert_eq!(kept["day"], day);
}

// No session was typed that day, so no reading call is made: there is no `claude` to call in this world.
#[test]
fn the_page_shows_a_made_wish_with_no_reading() {
    let w = world();
    assert_eq!(w.page(), Vec::<Value>::new());
    w.wish(SAID, "alpha");
    let label = "make the uploader retry twice";
    let drawn = json!({"id": "m1", "label": label, "said": SAID, "project": "alpha", "state": "working", "stars": []});
    assert_eq!(w.page(), [drawn]);
    assert!(!w.dir.path().join("jm").join("reading").exists(), "no reading call was made");
}

// A reading gives its wishes the ids w1 to w8 whatever the model called them, so a made wish's id is never one.
#[test]
fn a_made_wish_stands_after_the_reading_s_wishes_under_an_id_the_reading_never_gives() {
    let w = world();
    w.wish(SAID, "alpha");
    let typed = "add a retry to the uploader";
    write_log(&w.dir.path().join("logs"), "-work-alpha/s1.jsonl", &[said("2026-03-10T16:00:00.000Z", A, typed)]);
    install(&w.dir.path().join("bin"), "claude");
    let wish =
        |id: &str| json!({"id": id, "label": "Retry", "said": typed, "project": "alpha", "state": "done", "stars": []});
    let reading = json!({"wishes": [wish("m1"), wish("m2")], "clashes": []}).to_string();
    let reply = json!({"type": "result", "is_error": false, "result": reading}).to_string();
    fs::write(w.dir.path().join("bin").join("reply"), reply).unwrap();
    fs::write(w.dir.path().join("bin").join("exit"), "0").unwrap();
    let page = w.page();
    let ids: Vec<&str> = page.iter().map(|wish| wish["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["w1", "w2", "m1"]);
    assert_eq!((page[0]["said"].as_str(), page[2]["said"].as_str()), (Some(typed), Some(SAID)));
}

// The app keeps a reading in `<day>.json` and writes it again at each reading. The made wishes are in another file.
#[test]
fn a_kept_page_written_again_does_not_lose_the_made_wishes() {
    let w = world();
    w.wish(SAID, "alpha");
    let before = fs::read(w.wishes()).unwrap();
    let page = json!({"day": DAY, "wishes": [], "clashes": [], "read": {"turns": 0, "projects": []}}).to_string();
    for _ in 0..2 {
        fs::write(w.pages().join(format!("{DAY}.json")), &page).unwrap();
        assert_eq!(fs::read(w.wishes()).unwrap(), before);
        assert_eq!(w.page().len(), 1, "and the page still shows it");
    }
    assert_eq!(w.wish("one more", "alpha")["wish"]["id"], "m2");
}

// Eight at once, which is a day's fill, twelve times over: a write outside the lock would lose one of them. Twelve
// because Windows refuses a step on a file another process is using at that moment, which three rounds showed in
// one CI run of three: each of the eight must still exit 0 and say nothing on stderr.
#[test]
fn wishes_made_at_once_are_all_kept_each_under_its_own_id() {
    for round in 0..12 {
        let (w, start) = (world(), std::sync::Barrier::new(8));
        std::thread::scope(|scope| {
            for n in 1..=8 {
                let (w, start) = (&w, &start);
                scope.spawn(move || {
                    start.wait();
                    w.wish(&format!("wish number {n}"), "alpha")
                });
            }
        });
        let kept = w.kept();
        let wishes = kept["wishes"].as_array().unwrap();
        let ids: Vec<String> = wishes.iter().map(|wish| wish["id"].as_str().unwrap().to_string()).collect();
        assert_eq!(ids, (1..=8).map(|n| format!("m{n}")).collect::<Vec<_>>(), "round {round}");
        let mut words: Vec<String> = wishes.iter().map(|wish| wish["said"].as_str().unwrap().to_string()).collect();
        words.sort_unstable();
        assert_eq!(words, (1..=8).map(|n| format!("wish number {n}")).collect::<Vec<_>>(), "round {round}");
    }
}
