//! F34: opening the app shows today's page. The built `jellymoon-app` is run as a black box with `layout` and no
//! `--page`, which prints what the window would show once the day is read. The made-up day and harness are in
//! `support/day.rs`. Every log line and every reply is made up. Nothing here runs the real Claude Code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::{env, fs};

use serde_json::{Value, json};
use support::Piece;
use support::day::*;

#[test]
fn the_waiting_state_is_a_page_with_one_note_and_the_day_s_page_follows_it() {
    let world = world();
    let waiting = shown(&world, &["--waiting"]);
    assert_eq!(
        notes(&waiting),
        ["Reading your day… This takes a few seconds: your own Claude Code is reading what you typed today."]
    );
    assert!(waiting.iter().any(|piece| piece.kind() == "paper") && labels(&waiting).is_empty());
    assert!(!called(&world) && !world.home.exists(), "the waiting state reads nothing and keeps nothing");

    let page = shown(&world, &[]);
    assert_eq!(labels(&page), ["Fail fast", "Retry uploads"], "the let-go wish is placed last");
    let said = notes(&page);
    assert!(
        said[0].starts_with("came true today ★ no retry left") && said[2].starts_with("you changed your mind"),
        "{said:?}"
    );
    assert!(called(&world));
}

#[test]
fn a_kept_page_is_shown_without_a_call_until_the_day_has_turns_it_did_not_see() {
    let world = world();
    let first = notes(&shown(&world, &[]));
    assert!(called(&world));
    assert_eq!(notes(&shown(&world, &[])), first);
    assert!(!called(&world), "the kept page saw every turn of the day");

    let more = [said(&noon(3), &work("alpha"), "and log every failure")];
    write_log(&world, "-work-alpha/s3.jsonl", &more);
    shown(&world, &[]);
    assert!(called(&world), "a turn the kept page did not see");
    let page: Value = serde_json::from_slice(&fs::read(kept(&world)).unwrap()).unwrap();
    assert_eq!(page["read"]["turns"], 4);
    shown(&world, &[]);
    assert!(!called(&world));
    shown(&world, &["--fresh"]);
    assert!(called(&world), "asked to read again");

    let other = shown(&world, &["--date", "2026-03-09"]);
    assert_eq!(
        notes(&other)[0],
        "Nothing wished for yet today. Choose the folder you want to work in, or type a name for a new project in the \
         field at the top."
    );
    assert!(!called(&world), "a day with nothing typed makes no call");
    assert_eq!(other.iter().find(|piece| piece.kind() == "tape").unwrap().said(), "Mon · Mar 9");
}

/// The one note of a page that shows no page and keeps none. Its title is `lines` lines at this size: "Today
/// could not be read." is one, "There is no page for this day yet." is two.
fn one_note(world: &World, args: &[&str], more: &[(&str, &str)], lines: usize) -> String {
    let out = run(world, args, more);
    assert!(out.status.success() && out.stderr.is_empty(), "layout {args:?}");
    let placed = String::from_utf8(out.stdout).unwrap();
    let placed: Vec<Piece> = placed.lines().map(|line| Piece(serde_json::from_str(line).unwrap())).collect();
    let plain: Vec<&Piece> = placed.iter().filter(|piece| piece.kind() == "note").collect();
    assert_eq!(plain.len(), 1, "{args:?}");
    let roles = plain[0].roles();
    let titles = roles.iter().take_while(|role| *role == "note_title").count();
    assert_eq!(titles, lines, "the title's lines: {roles:?}");
    assert!(roles[titles..].iter().all(|role| role == "body"), "a plain note: {roles:?}");
    assert!(labels(&placed).is_empty() && !kept(world).exists(), "no page is shown and none is kept");
    plain[0].said()
}

#[test]
fn a_reading_that_cannot_run_is_one_plain_note_that_says_why() {
    let world = world();
    let one = |world: &World, args: &[&str], lines: usize| one_note(world, args, &[], lines);
    answer(&world, "", 1);
    let failed = one(&world, &[], 1);
    assert!(
        failed.starts_with(
            "Today could not be read. Claude Code could not do the reading. Check that `claude` answers in a terminal"
        ),
        "{failed}"
    );
    answer(&world, "not a result", 0);
    let odd = one(&world, &[], 1);
    assert!(odd.starts_with("Today could not be read. Claude Code could not do the reading."), "{odd}");
    // A look at a call's record removes it, so the look after the next case is at that case's call alone.
    assert!(called(&world), "the two calls before were made");
    // The harness's own result says the call was an error.
    answer(&world, &json!({"type": "result", "is_error": true, "result": "made-up trouble"}).to_string(), 0);
    let harness = one(&world, &[], 1);
    assert!(harness.starts_with("Today could not be read. Claude Code could not do the reading."), "{harness}");
    assert!(called(&world), "the call was made");
    answer(&world, &result("Sorry, I cannot help with that."), 0);
    assert_eq!(
        one(&world, &[], 1),
        "Today could not be read. The model's answer was not a page, so nothing from it is shown. Open Jellymoon again to try once more."
    );
    let nowhere = world.dir.path().join("nowhere");
    let none = one(&world, &["--logs", nowhere.to_str().unwrap()], 2);
    assert!(
        none.starts_with("There is no page for this day yet. No Claude Code session logs were found on this computer"),
        "{none}"
    );

    fs::remove_file(world.rec.join(format!("claude{}", env::consts::EXE_SUFFIX))).unwrap();
    let missing = one(&world, &[], 2);
    assert!(
        missing.starts_with("There is no page for this day yet. Claude Code was not found on this computer."),
        "{missing}"
    );
    assert!(missing.ends_with("It was looked for in PATH, ~/.local/bin, ~/.claude/local."), "{missing}");
}

// The review of the second app notes, note 2. Core's line names what could not be made or started, and only a
// `claude` that was found and did not start is one a terminal would show the trouble of.
#[test]
fn a_call_that_could_not_be_started_says_what_could_not_be_made_or_started_and_no_call_is_made() {
    let world = world();
    let start = "There is no page for this day yet. Claude Code could not be started, so nothing was read. ";
    let again = "Open Jellymoon again to try once more. What happened: the reading call could not run: ";
    // A file where the reading folder would be: the folder cannot be made, and `claude` is not the trouble.
    fs::create_dir_all(&world.home).unwrap();
    fs::write(world.home.join("reading"), "").unwrap();
    let unmade = one_note(&world, &[], &[], 2);
    let folder =
        format!("{start}{again}the reading folder in Jellymoon's own folder could not be made or written to: ");
    assert!(unmade.starts_with(&folder), "{unmade}");
    fs::remove_file(world.home.join("reading")).unwrap();
    // A program named that is not there: nothing is started, as with no `claude` found at all.
    let gone = world.dir.path().join("gone").join("claude");
    let unstarted = one_note(&world, &[], &[("JELLYMOON_CLAUDE", gone.to_str().unwrap())], 2);
    let named = format!("{start}{again}JELLYMOON_CLAUDE names no program that could be started: ");
    assert!(unstarted.starts_with(&named) && !unstarted.contains("gone"), "{unstarted}");
    // A `claude` on `PATH` whose own program is not there cannot be started. Nothing of it runs.
    #[cfg(unix)]
    {
        let broken = world.dir.path().join("broken");
        fs::create_dir(&broken).unwrap();
        fs::write(broken.join("claude"), "#!/no/such/program\n").unwrap();
        fs::set_permissions(broken.join("claude"), std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
        let found = one_note(&world, &[], &[("PATH", broken.to_str().unwrap())], 2);
        let advice = "Check that `claude` answers in a terminal. ";
        let unstartable = format!("{start}{advice}{again}the `claude` that was found could not be started: ");
        assert!(found.starts_with(&unstartable), "{found}");
    }
    assert!(!called(&world), "nothing was called");
}

#[test]
fn a_kept_page_stays_on_show_when_a_later_reading_fails_and_the_first_note_says_so() {
    let world = world();
    shown(&world, &[]);
    answer(&world, "", 1);
    let placed = shown(&world, &["--fresh"]);
    assert_eq!(labels(&placed), ["Fail fast", "Retry uploads"]);
    assert!(notes(&placed)[0].starts_with("This page is from an earlier reading. Claude Code could not do"));
}

#[test]
fn only_the_day_goes_to_the_call_with_every_tool_off_and_only_the_checked_page_is_kept() {
    let world = world();
    fs::create_dir_all(&world.home).unwrap();
    fs::write(world.home.join("skip.txt"), "private-notes\n").unwrap();
    shown(&world, &[]);

    let rec = |name: &str| fs::read_to_string(world.rec.join(name)).unwrap();
    let day: Value = serde_json::from_str(&rec("stdin")).unwrap();
    let typed: Vec<&str> = day["turns"].as_array().unwrap().iter().map(|turn| turn["said"].as_str().unwrap()).collect();
    assert_eq!((day["day"].as_str(), typed), (Some(DAY), vec![FIRST, SECOND]), "the day, less the folder switched off");
    assert_eq!(day.as_object().unwrap().len(), 2);
    assert_eq!((rec("arg3"), rec("arg4")), ("--tools".to_string(), String::new()));

    let page: Value = serde_json::from_str(&fs::read_to_string(kept(&world)).unwrap()).unwrap();
    let wish = |id: &str, label: &str, said: &str, state: &str, stars: Value| json!({"id": id, "label": label, "said": said, "project": "alpha", "state": state, "stars": stars});
    let checked = json!({
        "day": DAY,
        "wishes": [wish("w1", "Retry uploads", FIRST, "let_go", json!([])), wish("w2", "Fail fast", SECOND, "done", json!(["no retry left"]))],
        "clashes": [{"a": "w1", "b": "w2", "quote_a": FIRST, "quote_b": "never retry uploads", "settled": true}],
        "read": {"turns": 2, "projects": ["alpha"]},
    });
    assert_eq!(page, checked);
    assert_eq!(names(&world.home), ["pages", "reading", "skip.txt"]);
    assert_eq!(names(&world.home.join("pages")), [format!("{DAY}.json")]);
    assert!(names(&world.home.join("reading")).is_empty(), "the call's folder is gone");
}

// A list of switched-off folders that cannot be read is never an empty list: nothing goes to the call.
#[test]
fn a_skip_file_that_cannot_be_read_stops_the_reading_and_one_with_a_byte_order_mark_is_honoured() {
    let world = world();
    fs::create_dir_all(&world.home).unwrap();
    fs::write(world.home.join("skip.txt"), [b'p', 0xff, 0xfe, b'\n']).unwrap();
    let said = notes(&shown(&world, &[]));
    assert_eq!(said.len(), 1);
    assert!(
        said[0].starts_with("There is no page for this day yet. The list of folders you switched off (skip.txt"),
        "{said:?}"
    );
    assert!(said[0].ends_with("it is not UTF-8 text."), "{said:?}");
    assert!(!called(&world) && !kept(&world).exists(), "no call and no kept page");

    fs::write(world.home.join("skip.txt"), "\u{feff}private-notes\r\n").unwrap();
    shown(&world, &[]);
    let day: Value = serde_json::from_str(&fs::read_to_string(world.rec.join("stdin")).unwrap()).unwrap();
    assert_eq!(day["turns"].as_array().unwrap().len(), 2, "the first name is the folder's, not a mark and a name");
}

// With an empty environment the system could still say where the user's home is. The app does not ask it, so a run
// that names no folder stops here and never reads a real person's logs or opens a window.
#[test]
fn with_no_home_in_the_environment_the_app_stops_before_it_reads_anything() {
    for args in [&[][..], &["layout"], &["layout", "--fresh"]] {
        let out = support::run(args, None);
        let said = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {said}");
        assert!(said.starts_with("jellymoon-app: finding the session logs: no home directory"), "{args:?}: {said}");
        assert!(out.stdout.is_empty());
    }
    for logs in ["", "  "] {
        let out = support::run(&["layout", "--logs", logs], None);
        assert_eq!(out.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&out.stderr).starts_with("jellymoon-app: --logs needs a folder"));
    }
    // Core's one rule for Jellymoon's home is the app's too: a relative one is refused before anything is read.
    let world = world();
    let out = run(&world, &[], &[("JELLYMOON_HOME", "rel")]);
    let said = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(2), "{said}");
    assert!(said.starts_with("jellymoon-app: no home directory: set JELLYMOON_HOME to an absolute path"), "{said}");
    assert!(out.stdout.is_empty() && !called(&world) && !world.dir.path().join("rel").exists());
    // A `--skip` name with a character that cannot be seen is refused by core's own check, by its place.
    let out = run(&world, &["--skip", "alpha,private-\u{200d}notes"], &[]);
    let said = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(2), "{said}");
    let line = "jellymoon-app: --skip: name 2 holds a character that cannot be seen; name a part of the folder's \
                name without it\n";
    assert!(said.starts_with(line), "{said}");
    assert!(!said.contains("private") && out.stdout.is_empty() && !called(&world), "{said}");
}

#[test]
fn a_date_that_is_not_a_day_is_a_wrong_command_line() {
    for date in ["tomorrow", "2026-3-10", "../2026-03-10", "2026-02-30"] {
        let out = support::run(&["layout", "--date", date], None);
        let said = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{date}: {said}");
        assert!(
            said.starts_with("jellymoon-app: --date must be a day as YYYY-MM-DD") && out.stdout.is_empty(),
            "{said}"
        );
    }
}
