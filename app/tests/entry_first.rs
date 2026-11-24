//! The entry is the page's main thing: what a person who has never seen Jellymoon finds where they start. The
//! field is wide and tall, with `Start` beside it, and nothing covers it; with no project on offer the page says
//! to choose a folder first, or to name a new project, and words entered there are a name and never a wish; the
//! hint says what the chips are; a page with nothing read and no wish has no footer. Proved at
//! `jellymoon-app layout` and through the made-up window of F42's tests (`support/wisher.rs`), and on the Mac
//! in the page's window on GPUI's test platform (`entry_first/window.rs`). No window is opened on a screen.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;
#[cfg(all(target_os = "macos", feature = "window"))]
#[path = "entry_first/window.rs"]
mod window;

use std::fs;

use jellymoon_app::starting::UNMADE;
use serde_json::json;
use sticker_kit::{Note, Scene};
use support::day::{DAY, called, notes};
use support::wished::{SAID, World, quiet, showed};
use support::{Piece, pieces};

/// The two sizes a first-run page is looked at: the design's, and the smallest the window opens at.
const SIZES: [&str; 2] = ["1440x900", "960x640"];

/// The least share of the page's width the field takes.
const SHARE: f64 = 0.4;

/// The least height of the field, in the design's pixels.
const TALL: f64 = 40.0;

const START: &str = "Start";
const NEW_HINT: &str = "Choose a folder below, or type a name for a new project";
const HINT_TAB: &str = "What do you want done? Choose its project below.";
const FOLDER: &str = "choose the folder you want to work in";
const NAME: &str = "type a name for a new project";
const UNREADY: &str = "Nothing to start yet.";

fn of<'a>(placed: &'a [Piece], kind: &str) -> Vec<&'a Piece> {
    placed.iter().filter(|piece| piece.kind() == kind).collect()
}

fn words(placed: &[Piece], kind: &str) -> Vec<String> {
    of(placed, kind).iter().map(|piece| piece.said()).collect()
}

/// A world with nothing typed and nothing listed; with `logs` false, no session logs at all: a first run.
fn first_run(logs: bool) -> World {
    let w = quiet();
    if !logs {
        fs::remove_dir(w.at("logs")).unwrap();
    }
    w
}

fn add(w: &World, name: &str) {
    fs::create_dir_all(w.at("work").join(name).join(".git")).unwrap();
    w.jellymoon(&["project", "add", "--repo", w.at("work").join(name).to_str().unwrap()]);
}

fn overlaps(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3]
}

/// The field is wider than [`SHARE`] of the page and at least [`TALL`] high, `Start` stands beside it on its
/// row, in the band at the window's bottom, and no other piece of that band covers either. The column's pieces
/// are in the column's own places, which the view shows over the band only (`one_column.rs`).
fn the_field_comes_first(placed: &[Piece], size: &str) {
    let width: f64 = size.split('x').next().unwrap().parse().unwrap();
    let k = width / 1440.0;
    let (field, go) = (of(placed, "entry")[0].rect(), of(placed, "go"));
    assert!(field[2] > SHARE * width, "{size}: the field is {} of {width} wide", field[2]);
    assert!(field[3] >= TALL * k - 0.01, "{size}: the field is {} high", field[3]);
    let [go] = go[..] else { panic!("{size}: one Start: {go:?}") };
    assert_eq!(go.said(), START, "{size}");
    let go = go.rect();
    assert!(field[0] + field[2] < go[0], "{size}: Start is after the field: {field:?} {go:?}");
    assert!((field[1] - go[1]).abs() < 0.01 && (field[3] - go[3]).abs() < 0.01, "{size}: on its row: {go:?}");
    let height: f64 = size.split('x').nth(1).unwrap().parse().unwrap();
    assert!(field[1] > 0.8 * height && field[1] + field[3] < height, "{size}: at the window's bottom: {field:?}");
    for piece in placed.iter().filter(|piece| ["chip", "more", "action"].contains(&piece.kind())) {
        for mine in [field, go] {
            assert!(!overlaps(mine, piece.rect()), "{size}: {piece:?} covers {mine:?}");
        }
    }
}

#[test]
fn with_no_project_the_page_says_to_choose_a_folder_first_and_the_field_is_the_main_thing() {
    for logs in [false, true] {
        let w = first_run(logs);
        for size in SIZES {
            let placed = w.placed_with(&["--size", size]);
            let said = notes(&placed).join(" ").to_lowercase();
            let (folder, name) = (said.find(FOLDER), said.find(NAME));
            assert!(folder.is_some() && folder < name, "logs {logs} at {size}: the folder first: {said}");
            assert!(!said.contains("type what you want done"), "logs {logs} at {size}: no wish is asked for: {said}");
            assert_eq!(words(&placed, "entry"), [NEW_HINT], "logs {logs} at {size}: the field takes a name");
            // The hint says what the note says, in the same order: the folder, then the name.
            let hint = NEW_HINT.to_lowercase();
            assert!(hint.find("folder") < hint.find("name") && said.find("folder") < said.find("name"));
            the_field_comes_first(&placed, size);
            // Nothing read and nothing made: no line about a reading.
            assert!(of(&placed, "footer").is_empty(), "logs {logs} at {size}");
        }
        assert!(!called(&w.day), "logs {logs}: a reading call was made");
    }
}

#[test]
fn with_two_projects_the_hint_says_what_the_chips_are_and_start_is_beside_the_field() {
    let w = first_run(false);
    add(&w, "alpha");
    add(&w, "beta");
    for (theme, size) in ["sticker", "plain"].iter().flat_map(|theme| SIZES.map(|size| (theme, size))) {
        let placed = w.placed_with(&["--theme", theme, "--size", size]);
        assert_eq!(words(&placed, "entry"), [HINT_TAB], "{theme} at {size}: the hint, whole");
        assert_eq!(words(&placed, "chip"), ["alpha", "beta"], "{theme} at {size}");
        let field = of(&placed, "entry")[0].rect();
        assert!(of(&placed, "chip").iter().all(|chip| chip.rect()[1] > field[1] + field[3]), "the chips are below");
        the_field_comes_first(&placed, size);
        let said = notes(&placed).join(" ");
        assert!(said.contains("Type what you want done") && !said.to_lowercase().contains(FOLDER), "{said}");
    }
}

fn first(scene: &Scene) -> Option<(&str, &str)> {
    match scene.notes.first()? {
        Note::Plain { title, body } => Some((title, body)),
        _ => None,
    }
}

/// Whether a wish was kept for the day, or a run asked for.
fn wished(w: &World) -> bool {
    w.at("jm").join("pages").join(format!("{DAY}.wishes.json")).exists() || w.starts() > 0
}

#[test]
fn with_no_project_the_words_entered_are_a_new_project_s_name_and_never_a_wish() {
    let w = first_run(false);
    let scene = showed(w.enters_in("new", SAID));
    let (title, body) = first(&scene[0]).expect("one note");
    let made = format!("“{SAID}” is ready: type what you want done.");
    assert_eq!((title, body), ("Your new project is ready.", made.as_str()), "a project of that name");
    assert!(!wished(&w), "the words were taken as a wish");
}

#[test]
fn start_with_nothing_typed_starts_nothing_and_says_what_to_type_first() {
    let w = first_run(false);
    let scene = showed(w.starts_empty("new"));
    let (title, body) = first(&scene[0]).expect("one note");
    let lower = body.to_lowercase();
    assert!(title == UNREADY && lower.find(FOLDER) < lower.find(NAME) && lower.contains(NAME), "{title}: {body}");
    add(&w, "alpha");
    let scene = showed(w.starts_empty("wish"));
    let (title, body) = first(&scene[0]).expect("one note");
    assert_eq!((title, body), (UNREADY, "Type what you want done in the field first."));
    let list = w.jellymoon(&["project", "list"]);
    assert_eq!(list["projects"].as_array().unwrap().len(), 1, "only the project added: {list}");
    assert!(!wished(&w), "a wish was kept, or a run asked for");
}

/// A page of projects on offer, with nothing read and no wish made, in the page data a reading would keep.
fn nothing_read() -> serde_json::Value {
    let read = json!({"turns": 0, "projects": ["alpha", "beta"]});
    json!({"day": DAY, "wishes": [], "clashes": [], "read": read})
}

#[test]
fn projects_on_offer_with_nothing_read_and_no_wish_have_no_footer() {
    let placed = pieces(&nothing_read(), &[]);
    assert!(of(&placed, "footer").is_empty(), "{placed:?}");
}

#[test]
fn a_made_wish_with_no_turn_read_says_nothing_of_a_reading() {
    let w = first_run(true);
    add(&w, "alpha");
    w.jellymoon(&["wish", "--date", DAY, "--say", "tidy the uploader", "--project", "alpha"]);
    let placed = w.placed();
    // Every wish on the page is the person's own, and nothing was read: a footer would have nothing to say.
    assert_eq!(words(&placed, "footer"), Vec::<String>::new());
}

#[test]
fn start_with_nothing_typed_keeps_the_notes_standing_and_puts_its_own_first() {
    let w = first_run(false);
    let mut on_show = w.on_show();
    let standing = Note::Plain { title: UNMADE.to_string(), body: "That name is taken.".to_string() };
    on_show.scene.notes.insert(0, standing.clone());
    for id in ["new", "wish", "new"] {
        let scene = on_show.unready(id);
        assert_eq!(first(&scene).map(|(title, _)| title), Some(UNREADY), "{id}: first");
        let kept = scene.notes.iter().filter(|note| **note == standing).count();
        let nothing = scene.notes.iter().filter(|note| matches!(note, Note::Plain { title, .. } if title == UNREADY));
        assert_eq!((kept, nothing.count()), (1, 1), "{id}: the standing note stays, and one of ours");
    }
}
