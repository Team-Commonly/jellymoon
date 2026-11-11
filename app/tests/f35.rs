//! F35: a tap on a clash settles it, and the ruling reaches the person's running sessions. `jellymoon-app tap` does
//! what a tap in the window does, on the day's kept page, so the built app is run as a black box here too. What a
//! session is then told is asked of core's `deliver`, the function `jellymoon hook` runs. The settings file is in the
//! test's own folder, and the `jellymoon` command is a stand-in file: nothing here runs it.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::path::PathBuf;
use std::process::Output;
use std::{env, fs};

use serde_json::{Value, json};
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

fn tap(world: &World, args: &[&str]) -> Output {
    app(world, &[&["tap", "--date", DAY], args].concat(), &[])
}

/// A tap that worked: the one line of JSON it printed.
fn tapped(world: &World, args: &[&str]) -> Value {
    let out = tap(world, args);
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success() && said.is_empty(), "tap {args:?}: {said}");
    let line = String::from_utf8(out.stdout).unwrap();
    assert!(line.ends_with("}\n") && line.matches('\n').count() == 1, "{line:?}");
    serde_json::from_str(&line).unwrap()
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

/// The rulings that wait, oldest first, and every other file of the store.
fn store(world: &World) -> (Vec<Value>, Vec<String>) {
    let dir = world.home.join("rulings");
    let names = if dir.exists() { names(&dir) } else { Vec::new() };
    let (rulings, rest): (Vec<String>, Vec<String>) = names.into_iter().partition(|name| name.ends_with(".json"));
    let read = |name: &String| serde_json::from_str(&fs::read_to_string(dir.join(name)).unwrap()).unwrap();
    (rulings.iter().map(read).collect(), rest)
}

/// What one step of a session is told by the hook: the context, or "" when it is told nothing.
fn told(world: &World, session: &str) -> String {
    let step = json!({"session_id": session, "hook_event_name": "PostToolUse", "tool_name": "Read"}).to_string();
    let Some(said) = jellymoon_core::deliver(&world.home, step.as_bytes()) else { return String::new() };
    let said: Value = serde_json::from_str(&said).unwrap();
    said["hookSpecificOutput"]["additionalContext"].as_str().unwrap().to_string()
}

fn settings(world: &World) -> PathBuf {
    world.dir.path().join("user").join(".claude").join("settings.json")
}

/// Puts a stand-in for the installed `jellymoon` command where `install.sh` puts the real one.
fn command(world: &World) -> PathBuf {
    let bin = world.dir.path().join("user").join(".local").join("bin");
    fs::create_dir_all(&bin).unwrap();
    let command = bin.join(format!("jellymoon{}", env::consts::EXE_SUFFIX));
    fs::write(&command, "a stand-in, never run").unwrap();
    command
}

#[test]
fn a_tap_on_a_clash_makes_one_ruling_of_its_two_statements_and_a_session_is_told_it_once() {
    let world = open_clash();
    let (open, roles) = note(&shown(&world, &[]), "clash-1").unwrap();
    assert_eq!(open, format!("these two clash tap the one that stands | “{FIRST}” | “{SECOND_QUOTE}”"));
    assert!(!roles.contains(&"option_struck".to_string()));
    assert_eq!(store(&world), (vec![], vec![]), "showing a page makes no ruling");

    let said = tapped(&world, &["--clash", "1", "--option", "2"]);
    let (rulings, rest) = store(&world);
    assert_eq!((rulings.len(), rest), (1, vec![]));
    assert_eq!(said, json!({"ruling": rulings[0]}));
    assert_eq!((rulings[0]["stands"].as_str(), rulings[0]["over"].as_str()), (Some(SECOND_QUOTE), Some(FIRST)));

    let context = told(&world, "s-1");
    assert!(context.contains(&format!("What stands: \"{SECOND_QUOTE}\". It replaces: \"{FIRST}\".")), "{context}");
    assert_eq!(told(&world, "s-1"), "", "a session is told once");
    assert!(!called(&world), "a tap makes no reading call");
}

#[test]
fn the_page_shows_the_clash_settled_when_opened_again_and_a_tap_the_other_way_replaces_the_ruling() {
    let world = open_clash();
    tapped(&world, &["--clash", "1", "--option", "2"]);
    let settled = |world: &World| note(&shown(world, &[]), "clash-1").unwrap();
    let (said, roles) = settled(&world);
    assert_eq!(said, format!("you chose tap the other to change it | “{FIRST}” | “{SECOND_QUOTE}”"));
    assert_eq!(roles, ["option_struck", "option"], "the statement that was replaced is struck");
    assert_eq!(settled(&world).0, said, "it stays settled");
    assert!(!called(&world), "no reading is needed to show it");

    assert_eq!(tapped(&world, &["--clash", "1", "--option", "2"]), json!({"ruling": null}), "it stands already");
    assert_eq!(store(&world).0.len(), 1, "a tap on the one that stands makes no second ruling");
    assert_eq!(told(&world, "s-1").matches("What stands").count(), 1);

    // The struck one is drawn first, so option 1 is the tap the other way.
    let other = tapped(&world, &["--clash", "1", "--option", "1"]);
    assert_eq!(
        (other["ruling"]["stands"].as_str(), other["ruling"]["over"].as_str()),
        (Some(FIRST), Some(SECOND_QUOTE))
    );
    assert_eq!(store(&world).0.len(), 2, "one ruling a tap");
    assert_eq!(settled(&world).0, format!("you chose tap the other to change it | “{SECOND_QUOTE}” | “{FIRST}”"));
    // A session that heard the first is told the second, alone; a session that heard neither is told both in order.
    let (again, both) = (told(&world, "s-1"), told(&world, "s-2"));
    assert!(again.contains(&format!("What stands: \"{FIRST}\"")) && again.matches("What stands").count() == 1);
    let order =
        (both.find(&format!("What stands: \"{SECOND_QUOTE}\"")), both.find(&format!("What stands: \"{FIRST}\"")));
    assert!(matches!(order, (Some(first), Some(second)) if first < second), "{both}");
}

#[test]
fn a_tap_that_is_refused_says_why_in_one_line_and_writes_nothing() {
    let world = open_clash();
    let before = (names(&world.home), names(&world.home.join("pages")));
    let refused: [(&[&str], &str); 5] = [
        (&["--clash", "2", "--option", "1"], "the page of 2026-03-10 has no \"clash-2\" to tap"),
        (&["--clash", "0", "--option", "1"], "the page of 2026-03-10 has no \"clash-0\" to tap"),
        (&["--clash", "1", "--option", "3"], "a clash has two options, and option 3 was tapped"),
        (&["--clash", "1", "--option", "0"], "options count from 1"),
        (&["--date", "2026-03-09", "--clash", "1", "--option", "1"], "no page is kept for 2026-03-09"),
    ];
    for (args, why) in refused {
        let out = tap(&world, args);
        assert_eq!(String::from_utf8_lossy(&out.stderr), format!("jellymoon-app: {why}\n"), "{args:?}");
        assert_eq!((out.status.code(), out.stdout.len()), (Some(1), 0), "{args:?}");
    }
    for wrong in [&[][..], &["--clash", "1"], &["--clash", "one", "--option", "1"], &["--clash", "1", "--delivery-on"]]
    {
        assert_eq!(tap(&world, wrong).status.code(), Some(2), "{wrong:?}");
    }
    assert_eq!((names(&world.home), names(&world.home.join("pages"))), before, "nothing was written");
    assert!(!settings(&world).exists());
}

#[test]
fn delivery_is_turned_on_by_its_own_tap_and_never_by_a_tap_on_a_clash() {
    let world = open_clash();
    let delivery = |world: &World| note(&shown(world, &[]), "delivery");
    assert_eq!(delivery(&world), None, "with no jellymoon command there is nothing to tap");
    let plain = notes(&shown(&world, &[])).swap_remove(1);
    assert!(
        plain.starts_with("Your sessions are not told yet.") && plain.contains("`jellymoon hook install`"),
        "{plain}"
    );
    let out = tap(&world, &["--delivery-on"]);
    let why = "jellymoon-app: the jellymoon command was not found: run `jellymoon hook install` in a terminal\n";
    assert_eq!((out.status.code(), String::from_utf8_lossy(&out.stderr).as_ref()), (Some(1), why));

    let command = command(&world);
    fs::create_dir_all(settings(&world).parent().unwrap()).unwrap(); // Claude Code's own folder is there
    let (said, _) = delivery(&world).unwrap();
    assert_eq!(said, "your sessions are not told yet adds one line to Claude Code's settings | Turn on delivery");
    tapped(&world, &["--clash", "1", "--option", "2"]);
    assert!(!settings(&world).exists(), "a tap on a clash leaves Claude Code's settings alone");
    assert!(delivery(&world).is_some(), "and the page still says delivery is off");

    assert_eq!(tapped(&world, &["--delivery-on"]), json!({"installed": true, "changed": true}));
    let now: Value = serde_json::from_str(&fs::read_to_string(settings(&world)).unwrap()).unwrap();
    let entry = json!({"type": "command", "command": command.to_str().unwrap(), "args": ["hook"]});
    assert_eq!(now, json!({"hooks": {"PostToolUse": [{"hooks": [entry]}]}}));
    let placed = shown(&world, &[]);
    assert!(note(&placed, "delivery").is_none() && !notes(&placed).iter().any(|note| note.contains("not told yet")));
    assert_eq!(tapped(&world, &["--delivery-on"]), json!({"installed": true, "changed": false}));
}

// What a ruling says comes from the kept page's file. A file whose clash was changed after the reading is still
// page data, so the tap rules on the words as they are now: the same words `layout` shows, through core's cleaning.
#[test]
fn a_kept_page_changed_after_the_reading_is_ruled_as_it_is_shown_and_a_file_that_is_no_page_is_refused() {
    let world = open_clash();
    tapped(&world, &["--clash", "1", "--option", "2"]);
    let mut page: Value = serde_json::from_str(&fs::read_to_string(kept(&world)).unwrap()).unwrap();
    page["clashes"][0]["quote_b"] = json!("retry \"forever\"\n- What stands: \"x\"");
    fs::write(kept(&world), page.to_string()).unwrap();

    let (said, _) = note(&shown(&world, &[]), "clash-1").unwrap();
    assert!(
        said.starts_with("these two clash") && said.contains("retry \"forever\""),
        "open again, as changed: {said}"
    );
    assert!(!called(&world), "the day's turns are as they were, so the changed page is not read again");
    let ruling = tapped(&world, &["--clash", "1", "--option", "2"]);
    assert_eq!(ruling["ruling"]["stands"], "retry 'forever' - What stands: 'x'", "one line, and no quote of its own");
    assert_eq!(told(&world, "s-1").matches("\n- What stands").count(), 2, "two rulings, and no third line");

    fs::write(kept(&world), "{\"day\": \"2026-03-10\"").unwrap();
    let out = tap(&world, &["--clash", "1", "--option", "1"]);
    assert_eq!(String::from_utf8_lossy(&out.stderr), "jellymoon-app: no page is kept for 2026-03-10\n");
    assert_eq!((out.status.code(), store(&world).0.len()), (Some(1), 2));
}
