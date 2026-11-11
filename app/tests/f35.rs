//! F35: a tap on a clash settles it, and the ruling reaches the person's running sessions. The rules of the tap are
//! core's and are proved at `jellymoon settle` (`crates/jellymoon/tests/f35.rs`, F36). Here is the app's part: the
//! page draws what core says stands, in the words a ruling carries, and a tap in the window, followed with no window
//! through `OnShow`, is carried to core and drawn again. The settings file is in the test's own folder, and the
//! `jellymoon` command is a stand-in program that nothing here runs.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::path::{Path, PathBuf};
use std::{env, fs};

use jellymoon_app::taps::OnShow;
use jellymoon_app::today::Today;
use serde_json::{Value, json};
use sticker_kit::{Note, Scene};
use support::Piece;
use support::day::*;

const SECOND_QUOTE: &str = "never retry uploads";

/// The day with its clash still open, read and kept.
fn open_clash() -> World {
    let world = world();
    answer(&world, &result(&reading().replace("\"settled\":true", "\"settled\":false")), 0);
    shown(&world, &[]);
    assert!(called(&world));
    world
}

fn user(world: &World) -> PathBuf {
    world.dir.path().join("user")
}

fn settings(world: &World) -> PathBuf {
    user(world).join(".claude").join("settings.json")
}

/// The day, with Claude Code's settings at `settings`.
fn today(world: &World, settings: PathBuf) -> Today {
    Today {
        date: DAY.to_string(),
        logs: world.dir.path().join("logs"),
        skip: Vec::new(),
        model: "sonnet".to_string(),
        home: world.home.clone(),
        harness: None,
        fresh: false,
        settings: Some(settings),
        bin: Some(user(world).join(".local").join("bin")),
    }
}

/// The day as the window holds it, with its kept page on show.
fn on_show(world: &World, settings: PathBuf) -> OnShow {
    let today = today(world, settings);
    let page = today.kept().expect("the day's page is kept");
    let scene = today.draw(&page);
    OnShow { today, page: Some(page), scene }
}

/// Puts a stand-in for the installed `jellymoon` command where `install.sh` puts the real one: a real program.
fn command(world: &World) -> PathBuf {
    let bin = user(world).join(".local").join("bin");
    fs::create_dir_all(&bin).unwrap();
    let command = bin.join(format!("jellymoon{}", env::consts::EXE_SUFFIX));
    fs::copy(env!("CARGO_BIN_EXE_f34-claude"), &command).unwrap();
    #[cfg(unix)]
    fs::set_permissions(&command, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    command
}

/// The choice note with this id as placed: its title and footer, then each option in the order drawn, and the role
/// each option is drawn in.
fn note(placed: &[Piece], id: &str) -> Option<(String, Vec<String>)> {
    let sheet = placed.iter().find(|piece| piece.kind() == "note" && piece.id() == id)?;
    let mut options: Vec<&Piece> = placed.iter().filter(|piece| piece.kind() == "option" && piece.id() == id).collect();
    options.sort_by_key(|option| option.0["index"].as_u64());
    let said: Vec<String> = std::iter::once(sheet.said()).chain(options.iter().map(|option| option.said())).collect();
    Some((said.join(" | "), options.iter().map(|option| option.roles()[0].clone()).collect()))
}

/// A scene's plain notes and choices in words: a plain note's title, a choice's id, title, options and whether the first is struck.
fn said(scene: &Scene) -> Vec<String> {
    let one = |note: &Note| match note {
        Note::Plain { title, .. } => Some(title.clone()),
        Note::Choice { id, title, options, struck_first, .. } => {
            let struck = if *struck_first { " (first struck)" } else { "" };
            Some(format!("{id}: {title}: {}{struck}", options.join(" | ")))
        }
        _ => None,
    };
    scene.notes.iter().filter_map(one).collect()
}

/// The rulings that wait.
fn rulings(world: &World) -> usize {
    let dir = world.home.join("rulings");
    if dir.exists() { names(&dir).iter().filter(|name| name.ends_with(".json")).count() } else { 0 }
}

/// Every file under the test's folder that a tap could write: jellymoon's home, by name, and the settings.
fn written(world: &World) -> (Vec<String>, Vec<String>, bool) {
    let pages = world.home.join("pages");
    (names(&world.home), if pages.exists() { names(&pages) } else { Vec::new() }, settings(world).exists())
}

/// What one step of a session is told by the hook: the context, or "" when it is told nothing.
fn told(world: &World, session: &str) -> String {
    let step = json!({"session_id": session, "hook_event_name": "PostToolUse", "tool_name": "Read"}).to_string();
    let Some(said) = jellymoon_core::deliver(&world.home, step.as_bytes()) else { return String::new() };
    let said: Value = serde_json::from_str(&said).unwrap();
    said["hookSpecificOutput"]["additionalContext"].as_str().unwrap().to_string()
}

fn body(scene: &Scene, at: usize) -> String {
    match &scene.notes[at] {
        Note::Plain { title, body } => format!("{title} {body}"),
        _ => "not a plain note".to_string(),
    }
}

#[test]
fn the_page_shows_what_core_says_stands_and_showing_it_makes_no_ruling() {
    let world = open_clash();
    let (open, roles) = note(&shown(&world, &[]), "clash-1").unwrap();
    assert_eq!(open, format!("these two clash tap the one that stands | “{FIRST}” | “{SECOND_QUOTE}”"));
    assert!(!roles.contains(&"option_struck".to_string()));
    assert_eq!(rulings(&world), 0, "showing a page makes no ruling");

    // The tap itself is core's: the function `jellymoon settle` runs.
    let settled = jellymoon_core::settle_kept(&world.home, DAY, 1, 2).unwrap();
    assert_eq!((settled.stands.as_str(), settled.unkept), (SECOND_QUOTE, None));
    let (said, roles) = note(&shown(&world, &[]), "clash-1").unwrap();
    assert_eq!(said, format!("you chose tap the other to change it | “{FIRST}” | “{SECOND_QUOTE}”"));
    assert_eq!(roles, ["option_struck", "option"], "the statement that was replaced is struck");
    jellymoon_core::settle_kept(&world.home, DAY, 1, 1).unwrap();
    let (said, _) = note(&shown(&world, &[]), "clash-1").unwrap();
    assert_eq!(said, format!("you chose tap the other to change it | “{SECOND_QUOTE}” | “{FIRST}”"));
    assert!(!called(&world), "no reading is needed to show it");
}

/// The options of a clash as `layout` placed them, each with its lines joined by `between`.
fn options(placed: &[Piece], id: &str, between: &str) -> Vec<String> {
    let of_clash = placed.iter().filter(|piece| piece.kind() == "option" && piece.id() == id);
    let line = |run: &Value| run["text"].as_str().unwrap().to_string();
    of_clash.map(|option| option.runs().iter().map(line).collect::<Vec<_>>().join(between)).collect()
}

fn rewrite(world: &World, change: impl Fn(&mut Value)) {
    let mut page: Value = serde_json::from_str(&fs::read_to_string(kept(world)).unwrap()).unwrap();
    change(&mut page);
    fs::write(kept(world), page.to_string()).unwrap();
}

// Every word a ruling sends is on the page: the option `layout` prints is the ruling's `stands`, character for
// character, for a statement as long as a ruling carries, in words and in a script with no spaces, and for one that
// is longer and so cut by core.
#[test]
fn the_option_the_page_draws_is_the_ruling_s_statement_character_for_character() {
    let words = "keep every upload in the queue until the receiver has answered twice and say so in the log ";
    let in_words = |length: usize| words.repeat(4).chars().take(length - 1).chain(['x']).collect::<String>();
    let cases = [
        (in_words(200), " ", "1280x800"),
        (in_words(200), " ", "960x640"),
        (in_words(320), " ", "960x640"),
        ("请".repeat(200), "", "960x640"),
        ("请".repeat(200), "", "1440x900"),
    ];
    for (long, between, size) in cases {
        let world = open_clash();
        rewrite(&world, |page| page["clashes"][0]["quote_b"] = json!(long));
        let at = format!("{} characters at {size}", long.chars().count());
        let drawn = options(&shown(&world, &["--size", size]), "clash-1", between);
        assert_eq!(drawn.len(), 2, "{at}: the clash is drawn");
        let ruling = jellymoon_core::settle_kept(&world.home, DAY, 1, 2).unwrap().ruling.unwrap();
        assert_eq!(drawn, [format!("“{}”", ruling.over), format!("“{}”", ruling.stands)], "{at}");
        assert_eq!(ruling.stands.chars().count(), 200, "a ruling carries 200 characters");
        assert!(told(&world, "s").contains(&format!("What stands: \"{}\"", ruling.stands)));
        assert!(!called(&world));
    }
}

// Two clashes too long to be drawn together are never cut to fit: the one not chosen on yet is drawn whole and the
// other is counted, so each can be tapped in its turn. When not even one fits beside the other notes, none is drawn.
#[test]
fn a_clash_that_does_not_fit_is_counted_and_the_open_one_comes_first() {
    let long = |word: &str| format!("{word} ").repeat(200).chars().take(200).collect::<String>();
    let two = |world: &World, [a, b, c, d]: [String; 4]| {
        rewrite(world, |page| {
            let second = json!({"a": "a", "b": "b", "quote_a": c, "quote_b": d, "settled": false});
            (page["clashes"][0]["quote_a"], page["clashes"][0]["quote_b"]) = (json!(a), json!(b));
            page["clashes"].as_array_mut().unwrap().truncate(1);
            page["clashes"].as_array_mut().unwrap().push(second);
        });
    };
    let whole = |word: &str| format!("“{}”", long(word).trim_end());
    let world = open_clash();
    two(&world, ["retry", "never", "friday", "ready"].map(long));
    let placed = shown(&world, &["--size", "960x640"]);
    assert_eq!(options(&placed, "clash-1", " "), [whole("retry"), whole("never")]);
    assert_eq!(options(&placed, "clash-2", " "), [""; 0], "not drawn in part");
    assert!(notes(&placed).iter().any(|note| note.starts_with("+1 more of")), "{:?}", notes(&placed));

    jellymoon_core::settle_kept(&world.home, DAY, 1, 2).unwrap();
    let placed = shown(&world, &["--size", "960x640"]);
    assert_eq!(options(&placed, "clash-2", " "), [whole("friday"), whole("ready")], "the open clash is drawn now");
    assert_eq!(options(&placed, "clash-1", " "), [""; 0]);

    // Four statements of 200 of the widest characters: neither clash fits beside the page's other notes.
    two(&world, ["请", "问", "山", "水"].map(|sign| sign.repeat(200)));
    let placed = shown(&world, &[]);
    assert!(!placed.iter().any(|piece| piece.kind() == "option" && piece.id().starts_with("clash-")));
    assert!(notes(&placed).iter().any(|note| note.starts_with("+2 more of")), "{:?}", notes(&placed));
}

#[test]
fn a_tap_in_the_window_is_carried_to_core_and_the_page_is_drawn_from_what_core_says() {
    let world = open_clash();
    let mut shown = on_show(&world, settings(&world));
    let open = format!("clash-1: these two clash: “{FIRST}” | “{SECOND_QUOTE}”");
    assert_eq!(said(&shown.scene)[1..], *std::slice::from_ref(&open), "after the note that delivery is off");

    // An id that is not exactly a clash's, and an option that is not there, do nothing and write nothing.
    let before = written(&world);
    for (id, index) in [("clash-01", 0), ("clash-+1", 0), ("clash-2", 0), ("clash-1", 2), ("clash-", 0), ("", 0)] {
        let after = shown.tap(id, index).unwrap();
        assert_eq!(said(&after)[0], "That tap did nothing.", "{id} {index}");
        assert_eq!(said(&after)[2..], *std::slice::from_ref(&open), "{id} {index}");
    }
    assert_eq!((written(&world), rulings(&world)), (before, 0));

    let after = shown.tap("clash-1", 1).unwrap();
    let chose = format!("clash-1: you chose: “{FIRST}” | “{SECOND_QUOTE}” (first struck)");
    assert_eq!(said(&after)[1..], *std::slice::from_ref(&chose));
    assert_eq!(said(&shown.scene), said(&after), "the scene on show is the one a next tap acts on");
    assert_eq!(rulings(&world), 1);
    assert_eq!(said(&shown.tap("clash-1", 1).unwrap())[1..], [chose], "the one that stands, tapped again");
    assert_eq!(rulings(&world), 1, "makes no second ruling");
    // The struck one is drawn first, so the first option is the tap the other way.
    let other = shown.tap("clash-1", 0).unwrap();
    assert_eq!(said(&other)[1..], [format!("clash-1: you chose: “{SECOND_QUOTE}” | “{FIRST}” (first struck)")]);
    assert_eq!(rulings(&world), 2);
    assert!(told(&world, "s-1").ends_with(&format!("What stands: \"{FIRST}\". It replaces: \"{SECOND_QUOTE}\".")));
    assert!(!settings(&world).exists(), "a tap on a clash leaves Claude Code's settings alone");
}

// A tap sends the statement whose drawn words were tapped. When the page under the scene is no longer the one that
// was drawn, nothing is sent.
#[test]
fn a_tap_on_a_scene_the_page_no_longer_matches_sends_nothing() {
    let world = open_clash();
    let mut shown = on_show(&world, settings(&world));
    let page = shown.page.as_mut().unwrap();
    page.data.clashes[0].quote_b = "retry forever".to_string();
    let after = shown.tap("clash-1", 1).unwrap();
    assert_eq!(body(&after, 0), "That tap did nothing. the page has changed since it was drawn, so nothing was sent.");
    assert_eq!(rulings(&world), 0);
    // The scene is drawn again from the page as it is, so the next tap is on words the person saw.
    assert!(said(&after)[2].ends_with("“retry forever”"), "{:?}", said(&after));
    shown.tap("clash-1", 1).unwrap();
    assert_eq!((rulings(&world), told(&world, "s").contains("What stands: \"retry forever\"")), (1, true));
}

#[test]
fn a_ruling_the_page_could_not_keep_is_drawn_with_a_warning_and_still_shown_as_chosen() {
    let world = open_clash();
    fs::create_dir(world.home.join("pages").join(format!("{DAY}.ruled.json"))).unwrap();
    let mut shown = on_show(&world, settings(&world));
    let after = shown.tap("clash-1", 1).unwrap();
    let warning = body(&after, 0);
    assert!(warning.starts_with("This page could not keep your choice. Your choice stands and waits"), "{warning}");
    assert_eq!(said(&after)[2], format!("clash-1: you chose: “{FIRST}” | “{SECOND_QUOTE}” (first struck)"));
    assert_eq!(rulings(&world), 1, "the ruling was made");
    let again = shown.tap("clash-1", 1).unwrap();
    assert_eq!((rulings(&world), body(&again, 0) == warning), (1, true), "and is not made twice");
}

#[test]
fn delivery_is_offered_only_for_a_command_core_takes_and_is_turned_on_by_its_own_tap() {
    let world = open_clash();
    let offer = "delivery: your sessions are not told yet: Turn on delivery";
    let plain = "Your sessions are not told yet.";
    assert_eq!(said(&on_show(&world, settings(&world)).scene)[0], plain, "with no jellymoon command");
    let by_hand = notes(&shown(&world, &[])).swap_remove(1);
    assert!(by_hand.starts_with(plain) && by_hand.contains("`jellymoon hook install`"), "{by_hand}");

    let command = command(&world);
    fs::create_dir_all(settings(&world).parent().unwrap()).unwrap(); // Claude Code's own folder is there
    let relative = Path::new("user").join(".claude").join("settings.json");
    assert_eq!(said(&on_show(&world, relative).scene)[0], plain, "a settings file with no full path");
    #[cfg(unix)]
    {
        // A file that is not the person's own program is not offered: the hook would run it at every step.
        fs::set_permissions(&command, std::os::unix::fs::PermissionsExt::from_mode(0o777)).unwrap();
        assert_eq!(said(&on_show(&world, settings(&world)).scene)[0], plain, "a command anyone can write to");
        fs::set_permissions(&command, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    }
    let mut shown = on_show(&world, settings(&world));
    assert_eq!(said(&shown.scene)[0], offer);

    let after = shown.tap("delivery", 0).unwrap();
    assert!(!said(&after).iter().any(|note| note.contains("not told yet")), "{:?}", said(&after));
    let now: Value = serde_json::from_str(&fs::read_to_string(settings(&world)).unwrap()).unwrap();
    let entry = json!({"type": "command", "command": command.to_str().unwrap(), "args": ["hook"]});
    assert_eq!(now, json!({"hooks": {"PostToolUse": [{"hooks": [entry]}]}}));
    assert_eq!(rulings(&world), 0, "turning delivery on makes no ruling");
    assert_eq!(said(&shown.tap("delivery", 0).unwrap())[0], "That tap did nothing.", "the note is gone");
}

// The kept page comes through core's door: a file that is another day's page is not this day's.
#[test]
fn a_kept_page_of_another_day_is_not_shown_or_ruled_on() {
    let world = open_clash();
    let other = fs::read_to_string(kept(&world)).unwrap().replace(DAY, "2026-03-09");
    fs::write(kept(&world), other).unwrap();
    assert!(today(&world, settings(&world)).kept().is_none());
    shown(&world, &[]);
    assert!(called(&world), "so the day is read again");
}
