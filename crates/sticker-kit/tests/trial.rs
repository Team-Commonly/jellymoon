//! A picture is tried before it is trusted: the trial itself, at the kit's door. Every child here is the tests'
//! helper (`support/trial_child.rs`). No test rests on the clock, but for "ended within the limit", which has an
//! outer bound a slow machine does not reach.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use sticker_kit::Theme;
use sticker_kit::theme::{TRIAL_LIMIT, art_cost};
use support::{Folder, HONEST, OUTER, given, sized, trial, trial_as};

/// The first line of a kept answer: what it is for besides the picture.
const KEY: &str = if cfg!(debug_assertions) {
    "sticker-kit 0.1.0, usvg 0.46.0, read and drawn at twice its size, rules 2, debug, for tests"
} else {
    "sticker-kit 0.1.0, usvg 0.46.0, read and drawn at twice its size, rules 2, release, for tests"
};

/// The pictures four reviews found that a door of written rules took and the renderer choked on. Each is typed
/// here as its review gave it.
fn choking() -> [(&'static str, String); 4] {
    let cubics = "c1e6 0 0 1e6 1e6 1e6".repeat(60);
    let stroker = format!(
        r##"<g transform="scale(1e-6)"><path d="M0 0{cubics}" fill="none" stroke="#123456" stroke-width="2e9"/></g>"##
    );
    let percent = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8" viewBox="0 0 1e36 1e36">{}</svg>"#,
        r#"<ellipse rx="100%" ry="100%"/>"#.repeat(40)
    );
    let closes = format!(r#"<path d="M0 0L1 1{}"/>"#, "z".repeat(60_000));
    let ragged: String = (0..192).map(|i| format!("{:.2},{} ", i as f32 * 256.0 / 192.0, (i % 2) * 256)).collect();
    let paint = r#"fill="url(#g)" fill-opacity="0.5" fill-rule="evenodd" stroke="url(#g)" stroke-opacity="0.5" stroke-width="0.52""#;
    let gradient = r##"<linearGradient id="g" x2="0.02" gradientTransform="rotate(33)"><stop offset="0" stop-color="#f00"/><stop offset="0.5" stop-color="#0f0"/><stop offset="1" stop-color="#00f"/></linearGradient>"##;
    let zigzag = format!("{gradient}{}", format!(r#"<polyline points="{ragged}" {paint}/>"#).repeat(8));
    [
        ("a stroke thousands of times wider than its curve", sized(64, &stroker)),
        ("a percent of a huge viewBox", percent),
        ("a run of closes", sized(64, &closes)),
        ("ragged fills as tall as the picture", sized(256, &zigzag)),
    ]
}

#[test]
fn an_honest_picture_is_accepted_and_the_text_tried_is_the_text_given() {
    let folder = Folder::new("honest");
    assert_eq!(folder.star(HONEST, &trial()), Ok(HONEST.to_string()));
    // The child is given the picture and nothing else of the caller's: this process has a `PATH`, and it none.
    assert!(std::env::var_os("PATH").is_some());
    assert_eq!(folder.star(HONEST, &trial_as(&["bare"])), Ok(HONEST.to_string()));
    // A scalloped badge, as a review typed it: forty arcs in a row, each of them drawn.
    let badge = format!(r#"<path d="M20 2{}z"/>"#, "a1.5 1.5 0 0 1 2 1".repeat(40));
    assert_eq!(folder.star(&sized(128, &badge), &trial()), Ok(sized(128, &badge)));
    assert!(TRIAL_LIMIT >= Duration::from_secs(1), "the limit is many times what an honest picture takes");
}

// The trial is the proof and the rules are not: so each picture is given to a child that holds it to no rule, and
// to the kit's own, which refuses it by a rule before it is drawn.
#[test]
fn a_picture_the_renderer_chokes_on_costs_one_ended_child_whatever_the_rules_let_through() {
    let folder = Folder::new("choking");
    // The helper is started once before the clock matters: a system may look a new program over the first time.
    assert_eq!(folder.star(HONEST, &trial_as(&["unruled"])), Ok(HONEST.to_string()));
    let mut unruled = trial_as(&["unruled"]);
    unruled.limit = Duration::from_millis(1500);
    for (name, svg) in choking() {
        let started = Instant::now();
        let said = folder.star(&svg, &unruled);
        let took = started.elapsed();
        eprintln!("{name}, held to no rule: {said:?} in {took:?}");
        assert!(took < unruled.limit + OUTER, "{name}: the trial was ended within its limit, and took {took:?}");
        let ended = |said: &String| said.starts_with("its trial took longer") || said.starts_with("its trial ended");
        // A build with no debug assertions may draw one of these in time, on a machine fast enough.
        assert!(said.as_ref().is_err_and(ended) || !cfg!(debug_assertions), "{name}: {said:?}");

        let started = Instant::now();
        let ruled = folder.star(&svg, &trial()).expect_err(name);
        eprintln!("{name}, held to the rules: {ruled} in {:?}", started.elapsed());
        assert!(!ended(&ruled), "{name} is refused by a rule, with its line: {ruled}");
    }
    assert_eq!(folder.star(HONEST, &trial()), Ok(HONEST.to_string()), "and the caller is as it was");
}

#[test]
fn a_child_that_overflows_fails_says_what_is_no_answer_or_cannot_be_started_refuses_the_picture() {
    let folder = Folder::new("children");
    let said = |which: &str| folder.star(HONEST, &trial_as(&[which])).expect_err(which);
    let overflowed = said("overflows");
    assert!(overflowed.starts_with("its trial ended badly ("), "{overflowed}");
    let failed = said("fails");
    assert!(
        failed.starts_with("its trial ended badly (") && failed.contains('3'),
        "a good answer and a code: {failed}"
    );
    assert_eq!(said("garbage"), "its trial did not answer as a trial does");
    let coded = said("no-such-way");
    assert!(coded.starts_with("its trial ended badly (") && coded.contains('2'), "a code and no answer: {coded}");

    let mut nowhere = trial();
    nowhere.program = folder.0.join("no-such-program");
    let unstarted = folder.star(HONEST, &nowhere).expect_err("no program");
    assert!(unstarted.starts_with("its trial could not be started: "), "{unstarted}");
    nowhere.program = PathBuf::new();
    assert!(folder.star(HONEST, &nowhere).is_err(), "a caller that does not know its own program draws no art");
}

// A child that never ends is ended, and waited for: it holds a lock on a file for as long as it lives, and the
// lock is free the moment the trial has answered.
#[test]
fn a_child_that_never_ends_or_never_stops_writing_is_ended_at_the_limit_and_nothing_is_left_running() {
    let folder = Folder::new("never");
    assert_eq!(folder.star(HONEST, &trial()), Ok(HONEST.to_string()), "the helper has been started once");
    let lock = folder.0.join("held");
    for which in ["never", "floods"] {
        let mut slow = trial_as(&[which, lock.to_str().unwrap()]);
        // A third of a second, and twice that for as long as a busy machine had not let the child get to its
        // lock within the limit: the limit is not what is tested, the end is.
        for ms in [300, 600, 1200, 2400, 4800, 9600] {
            let _ = fs::remove_file(lock.with_extension("at"));
            slow.limit = Duration::from_millis(ms);
            let started = Instant::now();
            let said = folder.star(HONEST, &slow).expect_err(which);
            let took = started.elapsed();
            let ended =
                format!("its trial took longer than {ms} ms and was ended, and is tried again at the next start");
            assert_eq!(said, ended, "{which}");
            assert!(took >= slow.limit && took < slow.limit + OUTER, "{which}: {took:?}");
            let free = fs::File::open(&lock).and_then(|held| Ok(held.try_lock()?));
            assert!(free.is_ok(), "{which}: the child is gone when the answer is given: {free:?}");
            if lock.with_extension("at").is_file() {
                break;
            }
            assert!(ms < 9600, "{which}: the child never got as far as holding its lock");
        }
    }
}

/// How often the counting child was started.
fn starts(count: &std::path::Path) -> usize {
    fs::read(count).map_or(0, |bytes| bytes.len())
}

#[test]
fn a_kept_answer_is_given_with_no_child_started_and_only_for_its_own_picture_and_kit() {
    let (folder, kept) = (Folder::new("kept"), Folder::new("kept-answers"));
    let count = folder.0.join("count");
    let mut counted = trial_as(&["counted", count.to_str().unwrap()]);
    counted.kept = Some(kept.0.join("made-when-needed"));
    let kept_files = || -> Vec<PathBuf> {
        let folder = counted.kept.as_ref().unwrap();
        fs::read_dir(folder).map_or(Vec::new(), |entries| entries.flatten().map(|entry| entry.path()).collect())
    };

    assert_eq!(folder.star(HONEST, &counted), Ok(HONEST.to_string()));
    assert_eq!((starts(&count), kept_files().len()), (1, 1), "the first time a picture is seen it is tried");
    assert_eq!(folder.star(HONEST, &counted), Ok(HONEST.to_string()));
    assert_eq!(starts(&count), 1, "and never again: the answer was kept");

    // Another picture is another answer, and a refusal is kept as an acceptance is.
    let scripted = HONEST.replace("<circle", "<script/><circle");
    let refused = Err("it contains the element `<script`".to_string());
    assert_eq!(folder.star(&scripted, &counted), refused);
    assert_eq!(folder.star(&scripted, &counted), refused);
    assert_eq!((starts(&count), kept_files().len()), (2, 2));

    // A kept answer that is not for this kit, is cut short, is not an answer, or answers for another picture is
    // none: the picture is tried again, and the answer kept afresh.
    let file = kept_files().into_iter().find(|file| fs::read_to_string(file).unwrap().ends_with(HONEST)).unwrap();
    let good = fs::read_to_string(&file).unwrap();
    // The answer's line: what the rules count, the pixels drawn, and what the draw cost, which is not the same twice.
    let line = good.lines().nth(1).unwrap().to_string();
    let steady = |kept: &str| {
        let mut lines: Vec<String> = kept.lines().map(str::to_string).collect();
        lines[1] = lines[1].split(' ').take(4).collect::<Vec<_>>().join(" ");
        lines.join("\n")
    };
    assert_eq!(good.lines().next(), Some(KEY), "{good}");
    assert!(line.split(' ').count() == 6 && steady(&good).contains("\naccepted 26 356 400\n"), "{good}");
    let with = |answer: &str| good.replacen(&line, answer, 1);
    let spoiled = [
        good.replacen("sticker-kit 0.", "sticker-kit 9.", 1),
        good.replacen("usvg 0.46.0", "usvg 0.47.0", 1),
        good[..good.len() - 1].to_string(),
        with(&line.replacen("accepted 26 ", "accepted many ", 1)),
        with("refused: "),
        with(&format!("{line} 7")),
        good.replace(HONEST, &scripted),
        format!("{good}<!-- more -->"),
        with(&line.replacen(" 356 400 ", " 356 0 ", 1)),
        with(&line.replacen(" 356 400 ", " 356 ", 1)),
        good.replacen(if cfg!(debug_assertions) { ", debug, " } else { ", release, " }, ", another build, ", 1),
        good.replacen(", for tests\n", ", for another caller\n", 1),
        String::new(),
    ];
    for (nth, spoiled) in spoiled.iter().enumerate() {
        assert_ne!(*spoiled, good, "case {nth} is a change");
        fs::write(&file, spoiled).unwrap();
        assert_eq!(folder.star(HONEST, &counted), Ok(HONEST.to_string()), "case {nth}");
        assert_eq!(starts(&count), 3 + nth, "case {nth}: tried again");
        assert_eq!(steady(&fs::read_to_string(&file).unwrap()), steady(&good), "case {nth}: kept afresh");
    }
    // What the kept answer says is what is given, with nothing started: so it is as trusted as the theme's own
    // folder, and whoever can write one can write the other.
    let (key, picture) = (good.lines().next().unwrap(), good.split_once("\n<svg").unwrap().1);
    fs::write(&file, format!("{key}\nrefused: said by the file\n<svg{picture}")).unwrap();
    let before = starts(&count);
    assert_eq!(folder.star(HONEST, &counted), Err("said by the file".to_string()));
    assert_eq!(starts(&count), before);
}

#[test]
fn a_trial_that_ran_over_or_could_not_start_is_not_kept_and_no_folder_keeps_nothing() {
    let (folder, kept) = (Folder::new("unkept"), Folder::new("unkept-answers"));
    let mut never = trial_as(&["never", folder.0.join("held").to_str().unwrap()]);
    (never.kept, never.limit) = (Some(kept.0.clone()), Duration::from_millis(300));
    assert!(folder.star(HONEST, &never).is_err());
    let mut nowhere = never.clone();
    nowhere.program = folder.0.join("no-such-program");
    assert!(folder.star(HONEST, &nowhere).is_err());
    assert_eq!(fs::read_dir(&kept.0).unwrap().count(), 0, "a busy machine's answer may be another next time");

    let count = folder.0.join("count");
    let unkept = trial_as(&["counted", count.to_str().unwrap()]);
    assert_eq!(unkept.kept, None);
    for tried in 1..=2 {
        assert_eq!(folder.star(HONEST, &unkept), Ok(HONEST.to_string()));
        assert_eq!(starts(&count), tried);
    }
}

#[test]
fn each_picture_of_a_theme_has_its_own_answer_and_the_refused_ones_are_named() {
    let folder = Folder::new("several");
    let scripted = HONEST.replace("<circle", "<script/><circle");
    for (name, svg) in [("hero", HONEST), ("star", &scripted), ("badge", "not a picture")] {
        fs::write(folder.0.join(format!("{name}.svg")), svg).unwrap();
    }
    let theme = "[art]\nhero = \"hero.svg\"\nstar = \"star.svg\"\nbubble = \"gone.svg\"\nbadge = \"badge.svg\"\n";
    fs::write(folder.0.join("theme.toml"), theme).unwrap();
    let loaded = Theme::load(&folder.0.join("theme.toml")).expect("a theme loads whatever its pictures hold");
    assert_eq!(loaded.warnings, ["art.bubble: `gone.svg` was not found; the built-in art is used"]);
    let tried = loaded.theme.try_art(&trial());
    assert_eq!(given(&tried.accepted), [("hero", HONEST.to_string())]);
    let said: Vec<String> = tried.refused.iter().map(ToString::to_string).collect();
    assert_eq!(said.len(), 2, "{said:?}");
    assert_eq!(said[0], "the SVG for art.star is refused: it contains the element `<script`");
    assert!(said[1].starts_with("the SVG for art.badge is refused: it is not XML the renderer reads: "), "{said:?}");
    // What the child says of a picture is what the rules say of it in this process.
    assert_eq!(art_cost(&scripted), Err("it contains the element `<script`".to_string()));
    assert_eq!(Theme::sticker().try_art(&trial()), Default::default(), "a built-in theme has no folder and no art");
}

/// Every `.rs` file under `folder`, with its text.
fn sources(folder: &std::path::Path, found: &mut Vec<(String, String)>) {
    for entry in fs::read_dir(folder).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            sources(&path, found);
        } else if path.extension().is_some_and(|ending| ending == "rs") {
            found.push((path.file_name().unwrap().to_string_lossy().into_owned(), fs::read_to_string(&path).unwrap()));
        }
    }
}

// There is no side door: in what ships, a picture's text meets the parsers in the rules' own file, which only the
// child calls, the renderer in the child, and GPUI's image in the one function that is given accepted pictures and
// the kit's own drawings. A line added anywhere else fails here, and is looked at.
#[test]
fn no_shipped_code_reads_or_draws_a_picture_but_the_child_and_the_one_place_given_accepted_ones() {
    let kit = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut found = Vec::new();
    for folder in [kit.join("src"), kit.join("examples"), kit.join("..").join("..").join("app").join("src")] {
        sources(&folder, &mut found);
    }
    assert!(found.len() > 30, "the kit's and the app's sources were read: {}", found.len());
    let only = [
        ("art_cost(", &["svg_check.rs", "stand.rs"][..]),
        ("svg_refusal(", &["svg_check.rs"]),
        ("usvg::", &["svg_check.rs"]),
        ("roxmltree", &["svg_check.rs"]),
        ("resvg", &[]),
        ("SvgRenderer", &["stand.rs"]),
        ("ImageFormat::Svg", &["sprites.rs"]),
        ("svg_image(", &["sprites.rs"]),
        ("Accepted {", &["load.rs"]),
    ];
    for (name, text) in &found {
        let code = text.lines().filter(|line| !line.trim_start().starts_with("//"));
        let code: Vec<&str> = code.take_while(|line| !line.starts_with("mod tests")).collect();
        for (what, files) in only {
            let written = code.iter().filter(|line| line.contains(what)).count();
            assert!(written == 0 || files.contains(&name.as_str()), "{name} writes `{what}`");
        }
        let (images, kept) = (text.matches("svg_image(").count(), text.matches("Accepted {").count());
        assert!(name != "sprites.rs" || images == 3, "one function makes an image, called for two kinds: {images}");
        assert!(
            name != "load.rs" || kept == 3,
            "an accepted picture is named where it is made and nowhere else: {kept}"
        );
    }
}
