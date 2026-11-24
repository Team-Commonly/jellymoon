//! A newer build is installed (`docs/development/app.md`, "A newer build is installed"): the version in the
//! bundle's `Info.plist` is read when the window opens and at most once a minute after, and when the one on disk
//! is higher the page has one plain note, the last of its column. Every bundle here is a folder of the test's own,
//! made by hand in the shape `app/macos/bundle.sh` writes: no installed app is read, and nothing is started. The
//! day is kept by core with one made wish, as in F43's tests; the looks are made through `Showing` with the
//! test's clock, their work run on this thread.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use jellymoon_app::newer::{BODY, EVERY, MOST_BYTES, TITLE, Watch, bundle_of, read, version};
use jellymoon_app::showing::Showing;
use jellymoon_app::taps::OnShow;
use jellymoon_app::today::Today;
use sticker_kit::{Note, Scene};
use support::day::{self, DAY, shown};

/// The property list `app/macos/bundle.sh` writes, stamped with `version`.
fn plist(version: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Jellymoon</string>
  <key>CFBundleDisplayName</key><string>Jellymoon</string>
  <key>CFBundleIdentifier</key><string>me.commonly.jellymoon</string>
  <key>CFBundleExecutable</key><string>jellymoon-app</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundleShortVersionString</key><string>{version}</string>
  <key>CFBundleVersion</key><string>{version}</string>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
"#
    )
}

fn plist_of(bundle: &Path) -> PathBuf {
    bundle.join("Contents").join("Info.plist")
}

/// A bundle under `dir`, stamped with `version` when one is given.
fn bundle(dir: &Path, version: Option<&str>) -> PathBuf {
    let bundle = dir.join("Jellymoon.app");
    fs::create_dir_all(bundle.join("Contents").join("MacOS")).unwrap();
    version.into_iter().for_each(|version| fs::write(plist_of(&bundle), plist(version)).unwrap());
    bundle
}

/// The installer replaced the bundle: its property list is the one of `version`.
fn installed(bundle: &Path, version: &str) {
    fs::write(plist_of(bundle), plist(version)).unwrap();
}

fn note() -> Note {
    Note::Plain { title: TITLE.to_string(), body: BODY.to_string() }
}

#[test]
fn a_version_is_higher_by_its_numbers_as_numbers_and_then_by_having_no_suffix() {
    let v = |text: &str| version(text).unwrap_or_else(|| panic!("{text} is a version"));
    assert!(v("0.2.9") < v("0.2.10") && v("0.2.10") == v("0.2.10") && v("0.3.0-rc1") < v("0.3.0"));
    assert!(v("0.2.10") < v("0.3.0-rc1") && v("0.3") == v("0.3.0") && v("0.9.9") < v("1"));
    for none in ["", "garbage", "0.0.0", "0", "0.0.0-rc1", "0.2.x", "0.2.", ".2", "0.3.0-", "+1.2", "1.-2", " 1.2"] {
        assert_eq!(version(none), None, "{none:?}");
    }
    assert_eq!(version("99999999999999999999.1"), None, "a number too large is no number");
}

#[test]
fn the_version_is_read_from_the_file_the_bundle_script_writes_and_from_nothing_else() {
    let dir = support::scratch();
    let file = dir.path().join("Info.plist");
    let reads = |text: &[u8]| {
        fs::write(&file, text).unwrap();
        read(&file)
    };
    assert_eq!(reads(plist("0.2.10").as_bytes()), version("0.2.10"));
    assert_eq!(reads(plist("0.3.0-rc1").as_bytes()), version("0.3.0-rc1"));
    // The key missing: the other version of the list is not taken for it.
    let missing = plist("0.2.10").replace("CFBundleShortVersionString", "CFBundleLongVersionString");
    assert_eq!(reads(missing.as_bytes()), None);
    let unended = plist("0.2.10").replace("<string>0.2.10</string>\n  <key>CFBundleVersion", "<string>0.2.10");
    assert_eq!((reads(unended.as_bytes()), reads(plist("soon").as_bytes()), reads(b"")), (None, None, None));
    // A binary property list, which names the key and is no text.
    let binary = [b"bplist00\xd1\x01\x02".as_slice(), plist("0.2.10").as_bytes(), b"\xff\xfe\x00"].concat();
    assert_eq!(reads(&binary), None);
    // A file of the most bytes is read; one byte more and it is not.
    let padded = |bytes: u64| {
        let text = plist("0.2.10");
        format!("{text}{}", " ".repeat(usize::try_from(bytes).unwrap() - text.len()))
    };
    assert_eq!(reads(padded(MOST_BYTES).as_bytes()), version("0.2.10"));
    assert_eq!(reads(padded(MOST_BYTES + 1).as_bytes()), None);
    assert_eq!((read(dir.path()), read(&dir.path().join("none"))), (None, None), "a folder, and no file");
}

#[test]
fn a_program_is_in_a_bundle_only_under_its_contents_and_macos() {
    let bundle = Path::new("Apps").join("Jellymoon.app");
    assert_eq!(bundle_of(&bundle.join("Contents").join("MacOS").join("jellymoon-app")), Some(bundle.as_path()));
    let built = Path::new("target").join("debug").join("jellymoon-app");
    assert_eq!((bundle_of(&built), bundle_of("jellymoon-app".as_ref()), bundle_of("".as_ref())), (None, None, None));
    // This test's own program is in none, so a window opened by a test watches nothing.
    assert!(Watch::running().check(Instant::now()).is_none());
}

#[test]
fn the_version_on_disk_is_read_at_most_once_a_minute_and_only_a_higher_one_is_newer() {
    let dir = support::scratch();
    let bundle = bundle(dir.path(), Some("0.2.9"));
    let (watch, open) = (Watch::of(Some(&bundle)), Instant::now());
    let at = |seconds: u64| open + Duration::from_secs(seconds);
    let checked = |seconds: u64| watch.check(at(seconds)).map(|check| check());
    assert_eq!(EVERY, Duration::from_secs(60));
    // The first look reads; the looks of the minute after it read nothing, whatever is on disk by then.
    assert_eq!(checked(1), Some(false), "the version it was opened with");
    installed(&bundle, "0.2.10");
    assert_eq!((checked(2), checked(60)), (None, None));
    // An older build, the same one, a file that is gone and one that is not a version are all silence.
    for (minute, lower) in (0..).zip(["0.2.8", "0.2.9", "0.2.9-rc1", "soon"]) {
        installed(&bundle, lower);
        assert_eq!(checked(61 + 60 * minute), Some(false), "{lower}");
    }
    fs::remove_file(plist_of(&bundle)).unwrap();
    assert_eq!((checked(301), watch.noted(Vec::new())), (Some(false), Vec::new()));
    installed(&bundle, "0.2.10");
    assert_eq!((checked(360), checked(361)), (None, Some(true)), "0.2.10 is higher than 0.2.9");
    // Found, and the page changed under the look that found it: the next look says so again, reading nothing,
    // until a column was given with the note. Then nothing is read for as long as the window is open.
    installed(&bundle, "0.2.9");
    assert_eq!(checked(362), Some(true));
    assert_eq!(watch.noted(Vec::new()), [note()]);
    assert_eq!((checked(363), checked(3600)), (None, None));
}

#[test]
fn with_no_bundle_or_no_version_at_the_open_nothing_is_ever_read_or_shown() {
    let dir = support::scratch();
    let bare = bundle(dir.path(), None);
    fs::create_dir(dir.path().join("zero")).unwrap();
    fs::create_dir(dir.path().join("odd")).unwrap();
    let (zero, odd) = (bundle(&dir.path().join("zero"), Some("0.0.0")), bundle(&dir.path().join("odd"), Some("soon")));
    let watches = [Watch::of(None), Watch::of(Some(&bare)), Watch::of(Some(&zero)), Watch::of(Some(&odd))];
    // A release is installed over each: there was no version to be higher than, so there is no read.
    [&bare, &zero, &odd].into_iter().for_each(|bundle| installed(bundle, "0.2.10"));
    let open = Instant::now();
    for watch in &watches {
        assert!(watch.check(open).is_none() && watch.check(open + EVERY).is_none());
        assert_eq!(watch.noted(Vec::new()), Vec::new());
    }
}

/// A day read and kept with one made wish, as a window opened now holds it. No run is ever asked for.
fn wished() -> (day::World, OnShow) {
    let world = day::world();
    shown(&world, &[]);
    jellymoon_core::wish(&world.home, Some(DAY), "make the uploader retry twice", "alpha").unwrap();
    let today = Today {
        date: DAY.to_string(),
        logs: world.dir.path().join("logs"),
        skip: Vec::new(),
        model: "sonnet".to_string(),
        home: world.home.clone(),
        harness: Some(world.claude()),
        fresh: false,
        settings: None,
        bin: None,
    };
    let (scene, page) = today.first();
    (world, OnShow { today, page, scene })
}

/// The same day in another window.
fn again(on: &OnShow) -> OnShow {
    OnShow { today: on.today.clone(), page: on.page.clone(), scene: on.scene.clone() }
}

#[test]
fn the_look_that_finds_a_newer_build_shows_its_note_last_in_the_column_and_changes_nothing_else() {
    let (_world, on) = wished();
    let dir = support::scratch();
    let bundle = bundle(dir.path(), Some("0.2.9"));
    let showing = Showing::of(Some(again(&on))).bundled(Some(&bundle));
    let (open, scene) = (Instant::now(), showing.scene().unwrap());
    let before = showing.column(&scene);
    assert!(before.notes.iter().any(|note| matches!(note, Note::Said { id, .. } if id == "typed-m1")), "{before:?}");
    assert!(!before.notes.contains(&note()));
    assert!(showing.look_at(open).unwrap()().is_none(), "the build it was opened with: what is shown is kept");

    // The installer replaces the bundle under the open window. No look of the same minute reads it.
    installed(&bundle, "0.2.10");
    assert!(showing.look_at(open + Duration::from_secs(59)).unwrap()().is_none());
    let found = showing.look_at(open + EVERY).unwrap()().expect("the scene on show, to be drawn again");
    assert_eq!(showing.seen(found), Some(scene.clone()), "the scene that is kept has no note of it");
    let column = showing.column(&scene);
    let (last, above) = column.notes.split_last().unwrap();
    assert_eq!((last, above), (&note(), before.notes.as_slice()), "the last, with every note above it as it was");
    assert!(showing.look_at(open + EVERY * 2).unwrap()().is_none(), "found once: no look shows it again");

    // It is on every page of the window from then on, under a wish that is not made yet too, and on no kept scene.
    let text = |id: &str| Note::Text {
        id: id.to_string(),
        title: String::new(),
        body: Vec::new(),
        options: Vec::new(),
        quoted: String::new(),
        field: None,
    };
    let first_run = Scene { notes: vec![text("shape-said"), text("shape")], ..Scene::default() };
    assert_eq!(showing.column(&first_run).notes, [text("shape-said"), text("shape"), note()]);
    assert_eq!(showing.scene(), Some(scene.clone()));

    // An older build installed over a newer one, and a window in no bundle: no note, whatever the looks.
    let older = Showing::of(Some(again(&on))).bundled(Some(&bundle));
    installed(&bundle, "0.2.8");
    let quiet = [older, Showing::of(Some(again(&on))).bundled(None), Showing::of(Some(on))];
    for showing in quiet {
        assert!(showing.look_at(open).unwrap()().is_none() && showing.look_at(open + EVERY).unwrap()().is_none());
        assert_eq!(showing.column(&scene), before);
    }
}
