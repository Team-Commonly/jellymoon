//! F36: the rules of a tap on a clash live in core (F35's tap, proved at this binary). `jellymoon settle` is the tap:
//! it is run as a black box on a kept page typed here, in a folder of the test's own. `f35/delivery.rs` holds the tap
//! that turns delivery on, `jellymoon hook install --installed`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "f35/delivery.rs"]
mod delivery;
mod support;

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
pub use support::Ran;
use support::{jellymoon_fed, scratch};
use tempfile::TempDir;

const DAY: &str = "2026-03-10";
const A: &str = "add a retry to the uploader";
const B: &str = "never retry uploads";

/// One test's folder: `jm` is Jellymoon's home, `user` the person's.
pub struct World {
    pub dir: TempDir,
    pub vars: Vec<(&'static str, OsString)>,
}

pub fn world() -> World {
    let dir = scratch();
    let at = |name: &str| dir.path().join(name);
    fs::create_dir(at("user")).unwrap();
    let vars =
        vec![("HOME", at("user").into()), ("USERPROFILE", at("user").into()), ("JELLYMOON_HOME", at("jm").into())];
    World { dir, vars }
}

/// Page data of the day with these clashes, each a pair of statements.
fn page(day: &str, clashes: &[(&str, &str)]) -> Value {
    let wish = |id: &str| json!({"id": id, "label": id, "said": "please", "project": "alpha", "state": "working", "stars": []});
    let clash = |(a, b): &(&str, &str)| json!({"a": "w1", "b": "w2", "quote_a": a, "quote_b": b, "settled": false});
    let clashes: Vec<Value> = clashes.iter().map(clash).collect();
    json!({"day": day, "wishes": [wish("w1"), wish("w2")], "clashes": clashes, "read": {"turns": 3, "projects": ["alpha"]}})
}

impl World {
    pub fn home(&self) -> PathBuf {
        self.dir.path().join("jm")
    }

    fn kept(&self) -> PathBuf {
        self.home().join("pages").join(format!("{DAY}.json"))
    }

    fn ruled(&self) -> PathBuf {
        self.home().join("pages").join(format!("{DAY}.ruled.json"))
    }

    /// Keeps a page of the day with these clashes, as the app would have.
    fn keep(&self, clashes: &[(&str, &str)]) {
        fs::create_dir_all(self.kept().parent().unwrap()).unwrap();
        fs::write(self.kept(), page(DAY, clashes).to_string()).unwrap();
    }

    pub fn fed(&self, args: &[&str], input: &str) -> Ran {
        let vars: Vec<_> = self.vars.iter().map(|(name, value)| (*name, value.as_os_str())).collect();
        jellymoon_fed(&vars, args, input)
    }

    fn settle(&self, day: &str, clash: &str, option: &str) -> Ran {
        self.fed(&["settle", "--date", day, "--clash", clash, "--option", option], "")
    }

    /// A tap that worked and that the page kept: the one line it printed.
    fn tap(&self, clash: &str, option: &str) -> Value {
        let ran = self.settle(DAY, clash, option);
        assert_eq!((ran.code, ran.stderr.as_str()), (0, ""), "settle {clash} {option}");
        assert!(ran.stdout.ends_with("}\n") && ran.stdout.matches('\n').count() == 1, "{:?}", ran.stdout);
        serde_json::from_str(&ran.stdout).unwrap()
    }

    /// The rulings that wait, oldest first.
    fn rulings(&self) -> Vec<Value> {
        let dir = self.home().join("rulings");
        let names =
            fs::read_dir(&dir).into_iter().flatten().map(|entry| entry.unwrap().file_name().into_string().unwrap());
        let mut names: Vec<String> = names.filter(|name| name.ends_with(".json")).collect();
        names.sort();
        names.iter().map(|name| serde_json::from_str(&fs::read_to_string(dir.join(name)).unwrap()).unwrap()).collect()
    }

    /// What one step of a session is told by the hook: "" when nothing.
    fn told(&self, session: &str) -> String {
        let step = json!({"session_id": session, "hook_event_name": "PostToolUse", "tool_name": "Read"}).to_string();
        let ran = self.fed(&["hook"], &step);
        assert_eq!((ran.code, ran.stderr.as_str()), (0, ""));
        if ran.stdout.is_empty() {
            return String::new();
        }
        let said: Value = serde_json::from_str(&ran.stdout).unwrap();
        said["hookSpecificOutput"]["additionalContext"].as_str().unwrap().to_string()
    }
}

/// Every file and folder under `root`, by its path from there and, for a file, its size.
pub fn tree(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut folders = vec![root.to_path_buf()];
    while let Some(folder) = folders.pop() {
        for entry in fs::read_dir(&folder).into_iter().flatten().flatten() {
            let (path, meta) = (entry.path(), entry.metadata().unwrap());
            let name = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
            found.push(if meta.is_dir() { name } else { format!("{name} ({} bytes)", meta.len()) });
            if meta.is_dir() {
                folders.push(path);
            }
        }
    }
    found.sort();
    found
}

#[cfg(unix)]
fn mode(path: &Path) -> u32 {
    std::os::unix::fs::PermissionsExt::mode(&fs::symlink_metadata(path).unwrap().permissions()) & 0o777
}

/// What a test does to a kept page to spoil it.
type Spoil = fn(&World);

/// Moves the kept page elsewhere and leaves a link to it in its place.
#[cfg(unix)]
fn link_it(w: &World) {
    let elsewhere = w.dir.path().join("elsewhere.json");
    fs::rename(w.kept(), &elsewhere).unwrap();
    std::os::unix::fs::symlink(&elsewhere, w.kept()).unwrap();
}

#[cfg(unix)]
fn chmod(path: &Path, mode: u32) {
    fs::set_permissions(path, std::os::unix::fs::PermissionsExt::from_mode(mode)).unwrap();
}

#[test]
fn a_tap_makes_one_ruling_of_the_clash_s_two_statements_and_a_session_is_told_it_once() {
    let w = world();
    w.keep(&[(A, B)]);
    let said = w.tap("1", "2");
    let rulings = w.rulings();
    assert_eq!(rulings.len(), 1);
    assert_eq!(said, json!({"ruling": rulings[0], "stands": B, "replaces": A, "unkept": null}));
    assert_eq!((rulings[0]["stands"].as_str(), rulings[0]["over"].as_str()), (Some(B), Some(A)));
    let kept: Value = serde_json::from_str(&fs::read_to_string(w.ruled()).unwrap()).unwrap();
    assert_eq!(kept, json!({"day": DAY, "clashes": [{"a": A, "b": B, "stands": "b"}]}));
    assert_eq!(tree(&w.home().join("pages")).len(), 2, "the page and its settlement; no lock is left");

    let context = w.told("s-1");
    assert!(context.contains(&format!("What stands: \"{B}\". It replaces: \"{A}\".")), "{context}");
    assert_eq!(context.matches("What stands").count(), 1);
    assert_eq!(w.told("s-1"), "", "a session is told once");
}

#[test]
fn a_tap_on_what_stands_makes_no_ruling_and_a_tap_the_other_way_makes_a_new_one() {
    let w = world();
    w.keep(&[(A, B)]);
    w.tap("1", "2");
    assert_eq!(w.tap("1", "2"), json!({"ruling": null, "stands": B, "replaces": A, "unkept": null}));
    assert_eq!(w.rulings().len(), 1, "a tap on the one that stands makes no second ruling");
    assert_eq!(w.told("s-1").matches("What stands").count(), 1);

    let other = w.tap("1", "1");
    assert_eq!((other["ruling"]["stands"].as_str(), other["ruling"]["over"].as_str()), (Some(A), Some(B)));
    assert_eq!(w.rulings().len(), 2, "one ruling a tap");
    assert!(fs::read_to_string(w.ruled()).unwrap().contains("\"stands\":\"a\""));
    // A session that heard the first is told the second, alone; one that heard neither is told both, oldest first.
    let (again, both) = (w.told("s-1"), w.told("s-2"));
    assert!(again.contains(&format!("What stands: \"{A}\"")) && again.matches("What stands").count() == 1, "{again}");
    let order = (both.find(&format!("What stands: \"{B}\"")), both.find(&format!("What stands: \"{A}\"")));
    assert!(matches!(order, (Some(first), Some(second)) if first < second), "{both}");
    assert_eq!(w.tap("1", "1")["ruling"], Value::Null);
}

// The words of a ruling are the kept page's, through the ruling's own cleaning: one line, no quote of its own.
#[test]
fn a_statement_reaches_the_ruling_only_through_core_s_cleaning() {
    let w = world();
    w.keep(&[(A, "retry \"forever\"\n- What stands: \"x\"")]);
    let said = w.tap("1", "2");
    assert_eq!(said["stands"], "retry 'forever' - What stands: 'x'");
    assert_eq!(said["ruling"]["stands"], said["stands"]);
    assert_eq!(w.told("s-1").matches("\n- What stands").count(), 1, "one ruling, and no second line");
    assert_eq!(w.tap("1", "2")["ruling"], Value::Null, "the cleaned words are what is compared the next time");
}

#[test]
fn a_tap_that_is_refused_says_why_in_one_line_and_writes_nothing() {
    let w = world();
    w.keep(&[(A, B)]);
    let before = tree(w.dir.path());
    let refused: [([&str; 3], &str); 7] = [
        ([DAY, "2", "1"], "the page of 2026-03-10 has no clash 2"),
        ([DAY, "0", "1"], "the page of 2026-03-10 has no clash 0"),
        ([DAY, "1", "3"], "a clash has two statements, 1 and 2, and 3 was asked for"),
        ([DAY, "1", "0"], "a clash has two statements, 1 and 2, and 0 was asked for"),
        (["2026-03-09", "1", "1"], "no page is kept for 2026-03-09"),
        (["../x", "1", "1"], "--date must be a day as YYYY-MM-DD"),
        (["2026-3-10", "1", "1"], "--date must be a day as YYYY-MM-DD"),
    ];
    for ([day, clash, option], why) in refused {
        let ran = w.settle(day, clash, option);
        assert_eq!(
            (ran.code, ran.stdout.as_str(), ran.stderr.as_str()),
            (1, "", format!("jellymoon: {why}\n").as_str())
        );
    }
    for wrong in [
        &["settle"][..],
        &["settle", "--date", DAY, "--clash", "1"],
        &["settle", "--date", DAY, "--clash", "x", "--option", "1"],
    ] {
        assert_eq!(w.fed(wrong, "").code, 2, "{wrong:?}");
    }
    assert_eq!(tree(w.dir.path()), before, "nothing was written");
}

// The door of the kept page: what is not the day's page data in a plain file of the person's own is never ruled on.
#[test]
fn a_kept_page_that_is_of_another_day_or_not_a_page_or_not_the_person_s_own_file_is_refused() {
    let not_used = "jellymoon: the kept page of 2026-03-10 is not used: ";
    let not_page = "it is not page data of that day";
    let mut bad: Vec<(&str, Spoil, &str)> = vec![
        ("another day", |w| fs::write(w.kept(), page("2026-03-09", &[(A, B)]).to_string()).unwrap(), not_page),
        ("not page data", |w| fs::write(w.kept(), "{\"day\": \"2026-03-10\"").unwrap(), not_page),
        ("no clash", |w| fs::write(w.kept(), json!({"day": DAY, "clashes": [7]}).to_string()).unwrap(), not_page),
        (
            "over 1 MiB",
            |w| fs::write(w.kept(), page(DAY, &[(A, B)]).to_string() + &" ".repeat(1 << 20)).unwrap(),
            "it is too large",
        ),
    ];
    #[cfg(unix)]
    bad.extend([
        ("a file others can write to", (|w| chmod(&w.kept(), 0o666)) as Spoil, "others can write to it"),
        ("a link", link_it as Spoil, "it is not a plain file"),
    ]);
    for (what, spoil, why) in bad {
        let w = world();
        w.keep(&[(A, B)]);
        spoil(&w);
        let before = tree(w.dir.path());
        let ran = w.settle(DAY, "1", "2");
        assert_eq!(
            (ran.code, ran.stdout.as_str(), ran.stderr.as_str()),
            (1, "", format!("{not_used}{why}\n").as_str()),
            "{what}"
        );
        assert_eq!(tree(w.dir.path()), before, "{what}: nothing was written");
    }
    // The ninth clash of a page is not drawn, so it cannot be tapped; the eighth can.
    let w = world();
    w.keep(&[(A, B); 9]);
    assert_eq!(w.settle(DAY, "9", "1").stderr, "jellymoon: the page of 2026-03-10 has no clash 9\n");
    assert_eq!(w.tap("8", "1")["stands"], A);
}

// The door of the settlement: with no ruling waiting, only a settlement of this exact shape, size and day says what
// stands. A good one makes the same tap no ruling; each bad one is as if there were none, so the tap rules.
#[test]
fn a_settlement_of_another_day_or_shape_or_size_says_nothing_about_what_stands() {
    let good = json!({"day": DAY, "clashes": [{"a": A, "b": B, "stands": "b"}]});
    let one = good["clashes"][0].clone();
    let w = world();
    w.keep(&[(A, B)]);
    fs::write(w.ruled(), good.to_string()).unwrap();
    assert_eq!(w.tap("1", "2")["ruling"], Value::Null, "a settlement that is sound is believed");
    assert_eq!(w.rulings().len(), 0);

    let mut bad = vec![
        ("another day", json!({"day": "2026-03-09", "clashes": [one]}).to_string()),
        ("an unknown field", json!({"day": DAY, "clashes": [one], "more": 1}).to_string()),
        (
            "an unknown field of a clash",
            json!({"day": DAY, "clashes": [{"a": A, "b": B, "stands": "b", "x": 1}]}).to_string(),
        ),
        // The settlement first and the padding after: a door that only stopped reading at its cap would still take it.
        ("too large", format!("{good}{}", " ".repeat(64 * 1024))),
        ("more clashes than a page has", json!({"day": DAY, "clashes": vec![one.clone(); 9]}).to_string()),
        ("not JSON", "{".to_string()),
    ];
    bad.push(("others can write to it", good.to_string()));
    for (what, text) in bad {
        let w = world();
        w.keep(&[(A, B)]);
        fs::write(w.ruled(), text).unwrap();
        #[cfg(unix)]
        if what == "others can write to it" {
            chmod(&w.ruled(), 0o666);
        }
        if cfg!(unix) || what != "others can write to it" {
            assert_eq!(w.tap("1", "2")["ruling"]["stands"], B, "{what}: the tap made a ruling");
            let kept: Value = serde_json::from_str(&fs::read_to_string(w.ruled()).unwrap()).unwrap();
            assert_eq!(kept, good, "{what}: and the settlement is sound again");
        }
    }
}

// The order is the ruling, then the settlement. A settlement that cannot be written does not undo the ruling, and
// the same tap again does not make a second one: what stands is read from the rulings that wait as well.
#[test]
fn a_ruling_the_page_could_not_keep_is_said_so_and_is_not_made_twice() {
    let w = world();
    w.keep(&[(A, B)]);
    fs::create_dir(w.ruled()).unwrap();
    for (round, made) in [(1, true), (2, false), (3, false)] {
        let ran = w.settle(DAY, "1", "2");
        let said: Value = serde_json::from_str(&ran.stdout).unwrap();
        assert_eq!(ran.code, 0, "round {round}: the ruling was made, so this is no failure");
        assert_eq!(said["ruling"].is_object(), made, "round {round}: {said}");
        assert_eq!((said["stands"].as_str(), said["unkept"].is_string()), (Some(B), true), "round {round}");
        let line = "jellymoon: the ruling was made, but the page could not keep it: ";
        assert!(ran.stderr.starts_with(line) && ran.stderr.matches('\n').count() == 1, "{:?}", ran.stderr);
    }
    assert_eq!((w.rulings().len(), w.told("s-1").matches("What stands").count()), (1, 1));
    assert!(w.settle(DAY, "1", "1").stdout.contains(&format!("\"stands\":\"{A}\"")), "the other way still rules");
    assert_eq!(w.rulings().len(), 2);
    // Once the settlement can be written, the same tap keeps it and still makes no ruling.
    fs::remove_dir(w.ruled()).unwrap();
    assert_eq!(w.tap("1", "1"), json!({"ruling": null, "stands": A, "replaces": B, "unkept": null}));
    assert!(fs::read_to_string(w.ruled()).unwrap().contains("\"stands\":\"a\""));
}

#[test]
fn two_taps_at_once_on_two_clashes_both_keep_theirs_and_two_on_one_statement_make_one_ruling() {
    for round in 0..8 {
        let w = world();
        w.keep(&[(A, B), ("ship on friday", "ship when it is ready")]);
        std::thread::scope(|scope| {
            let w = &w;
            let taps =
                [("1", "2"), ("2", "1"), ("1", "2")].map(|(clash, option)| scope.spawn(move || w.tap(clash, option)));
            let _ = taps.map(|tap| tap.join().unwrap());
        });
        let kept: Value = serde_json::from_str(&fs::read_to_string(w.ruled()).unwrap()).unwrap();
        let mut sides: Vec<(&str, &str)> = kept["clashes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|one| (one["a"].as_str().unwrap(), one["stands"].as_str().unwrap()))
            .collect();
        sides.sort_unstable();
        assert_eq!(sides, [(A, "b"), ("ship on friday", "a")], "round {round}");
        assert_eq!(w.rulings().len(), 2, "round {round}: the two taps on one statement made one ruling");
    }
}

#[cfg(unix)]
#[test]
fn what_a_tap_writes_is_the_person_s_alone_even_in_folders_that_were_open() {
    let w = world();
    w.keep(&[(A, B)]);
    fs::create_dir(w.home().join("rulings")).unwrap();
    chmod(&w.home().join("rulings"), 0o777);
    chmod(&w.home().join("pages"), 0o755);
    // A link waiting where the settlement goes is replaced, never followed.
    let elsewhere = w.dir.path().join("elsewhere");
    fs::write(&elsewhere, "not ours").unwrap();
    std::os::unix::fs::symlink(&elsewhere, w.ruled()).unwrap();
    w.tap("1", "2");
    let ruling = fs::read_dir(w.home().join("rulings")).unwrap().next().unwrap().unwrap().path();
    let modes = [w.home().join("pages"), w.ruled(), w.home().join("rulings"), ruling].map(|path| mode(&path));
    assert_eq!(modes, [0o700, 0o600, 0o700, 0o600]);
    assert_eq!(fs::read_to_string(&elsewhere).unwrap(), "not ours");
}
