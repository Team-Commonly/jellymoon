//! F47: a tap on a little one opens its wish, and work that waits is shown, kept or tossed there. The rules and
//! the words are core's, proved at `jellymoon run waiting`, `run show`, `run keep` and `run toss`
//! (`crates/jellymoon/tests/f47.rs`, `f44.rs`, `f45.rs`, `f46.rs`). Here is the app's part: what `layout` prints
//! of work that waits, and the taps, carried as the window carries them by a made-up window in a process of its
//! own (`support/tapper.rs`, `support/worked.rs`). The wish is run by the `jellymoon` command with the made-up
//! agent of F39's tests: nothing here starts the real Claude Code, and no window is opened.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "f47/asked.rs"]
mod asked;
#[path = "f47/gone.rs"]
mod gone;
#[path = "f47/named.rs"]
mod named;
mod support;

use std::fs;
use std::path::Path;

use sticker_kit::{Note, Scene, State};
use support::Piece;
use support::day::DAY;
use support::wished::{SAID, World};
use support::worked::{finished, tree};

pub const WORK: &str = "Work waits for this wish.";
pub const CHOICES: [&str; 3] = ["Show me", "Keep it", "Toss it"];

/// One of the notes of an opened wish: its id, title, body and options.
pub type Text<'a> = (&'a str, &'a str, Vec<&'a str>, Vec<&'a str>);

/// The notes of an opened wish on a scene, in their order.
pub fn texts(scene: &Scene) -> Vec<Text<'_>> {
    fn strs(all: &[String]) -> Vec<&str> {
        all.iter().map(String::as_str).collect()
    }
    fn text(note: &Note) -> Option<Text<'_>> {
        match note {
            Note::Text { id, title, body, options } => Some((id, title, strs(body), strs(options))),
            _ => None,
        }
    }
    scene.notes.iter().filter_map(text).collect()
}

/// The state and the stars of a wish's character.
pub fn drawn(scene: &Scene, id: &str) -> Option<(State, u8)> {
    scene.characters.iter().find(|one| one.id == id).map(|one| (one.state, one.stars))
}

pub fn quoted(said: &str) -> String {
    format!("“{said}”")
}

/// The work of the first made wish, as `jellymoon run waiting` lists it, when it waits.
pub fn waits(w: &World, repo: &Path) -> Option<serde_json::Value> {
    let (logs, repo) = (w.at("logs"), repo.to_str().unwrap());
    let listed = w.jellymoon(&["run", "waiting", "--date", DAY, "--repo", repo, "--logs", logs.to_str().unwrap()]);
    listed["waits"].as_array().unwrap().first().cloned()
}

/// What the note of work that waits says under its title, in core's words.
pub fn lines(w: &World, repo: &Path) -> Vec<String> {
    let waits = waits(w, repo).expect("the work waits");
    let names: Vec<&str> = waits["names"].as_array().unwrap().iter().map(|name| name.as_str().unwrap()).collect();
    vec![quoted(SAID), format!("{}.", waits["why"].as_str().unwrap()), names.join(", ")]
}

fn of<'a>(placed: &'a [Piece], kind: &str, id: &str) -> Vec<&'a Piece> {
    placed.iter().filter(|piece| piece.kind() == kind && piece.id() == id).collect()
}

#[test]
fn layout_prints_one_note_for_work_that_waits_with_the_wish_s_words_its_files_and_three_choices() {
    let (w, repo) = finished("edits");
    let before = w.everything(&repo);
    let placed = w.placed();
    let [note] = of(&placed, "note", "work-m1")[..] else { panic!("one note of the work: {placed:?}") };
    let said = note.said();
    let run = w.kept()["run"].as_str().unwrap().to_string();
    for part in [
        WORK.to_string(),
        quoted(SAID),
        format!("the work of run {run} waits: files made 1, changed 1, removed 1."),
        "a.txt, b.txt, n.txt".to_string(),
    ] {
        assert!(said.contains(&part), "{part:?} is not in {said:?}");
    }
    let options: Vec<String> = of(&placed, "option", "work-m1").iter().map(|option| option.said()).collect();
    assert_eq!(options, CHOICES);
    assert_eq!(w.everything(&repo), before, "a layout wrote something");

    // A day whose made wish has no work waiting prints no such note.
    w.jellymoon(&[
        "run",
        "toss",
        "--wish",
        "m1",
        "--date",
        DAY,
        "--repo",
        repo.to_str().unwrap(),
        "--logs",
        w.at("logs").to_str().unwrap(),
    ]);
    assert!(w.placed().iter().all(|piece| !piece.id().starts_with("work-")));
}

#[test]
fn show_me_draws_one_file_of_the_work_as_text_and_the_next_tap_the_next_file_and_writes_nothing() {
    let (w, repo) = finished("rules");
    let before = w.everything(&repo);
    let (logs, named) = (w.at("logs"), repo.to_str().unwrap());
    let file = |n: &str| {
        let args = ["run", "show", "--wish", "m1", "--date", DAY, "--file", n, "--repo", named, "--logs"];
        w.jellymoon(&[&args[..], &[logs.to_str().unwrap()]].concat())["text"].as_str().unwrap().to_string()
    };
    let (first, second) = (file("1"), file("2"));
    let acts = ["poke:m1", "choose:work-m1:0", "run", "choose:work-m1:0", "run", "poke:m1"];
    let showed = w.taps("logs", &acts).showed();
    let [opened, reading, one, _, two, closed] = &showed[..] else { panic!("six acts") };

    let work = lines(&w, &repo);
    let work: Vec<&str> = work.iter().map(String::as_str).collect();
    let note = ("work-m1", WORK, work, CHOICES.to_vec());
    assert_eq!(texts(opened.as_ref().unwrap()), std::slice::from_ref(&note));
    assert_eq!(texts(reading.as_ref().unwrap()), [("busy-m1", "That work is being read.", vec![], vec![])]);
    // The file's text is the note's body, a line of it each, cleaned as `run show` cleans it: nothing in it is a
    // page, a link, a colour or a command to anything here.
    let body: Vec<&str> = first.lines().collect();
    assert_eq!(body[0], "<script>open('file:///')</script>");
    assert_eq!((body[1].contains("[tap](https://example.test)"), body[2]), (true, "$(touch ran)"));
    assert!(!first.contains('\u{1b}'), "{first:?}");
    let shown = [note.clone(), ("file-1-m1", "File 1 of the work: 0.md", body, vec![])];
    assert_eq!(texts(one.as_ref().unwrap()), shown);
    let shown = [note.clone(), ("file-2-m1", "File 2 of the work: CLAUDE.md", second.lines().collect(), vec![])];
    assert_eq!(texts(two.as_ref().unwrap()), shown);
    // A tap on the little one again shows the work alone.
    assert_eq!(texts(closed.as_ref().unwrap()), [note]);

    assert_eq!(w.everything(&repo), before, "a show wrote something");
    let ran =
        |files: support::worked::Files| files.keys().any(|path| path.file_name().is_some_and(|name| name == "ran"));
    assert!(!ran(tree(w.day.dir.path())), "a line of the file was run");
}

#[test]
fn the_shell_starts_no_program_and_hands_nothing_to_the_system_s_opener() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut read = 0;
    for (path, bytes) in tree(&src) {
        let text = String::from_utf8(bytes).unwrap();
        for word in ["Command::new", "process::Command", "open_url", "open_with", "open_path", "reveal_path"] {
            assert!(!text.contains(word), "{} names {word}", path.display());
        }
        read += 1;
    }
    assert!(read >= 12, "the shell's own files were to be read: {read}");
}

#[test]
fn a_tap_on_any_other_little_one_shows_its_wish_s_own_words_and_how_it_stands_and_changes_nothing() {
    let (w, repo) = finished("edits");
    w.jellymoon(&["wish", "--date", DAY, "--say", "tidy the notes", "--project", "alpha"]);
    let before = w.everything(&repo);
    let acts = ["poke:m2", "poke:m2", "poke:m1", "poke:m2", "poke:me", "poke:m9", "poke:", "choose:wish-m2:0"];
    let showed = w.taps("logs", &acts).showed();
    let [bare, again, work, back, rest @ ..] = &showed[..] else { panic!("eight acts") };

    let made = [("wish-m2", "This wish is being worked on.", vec!["“tidy the notes”"], vec![])];
    let bare = bare.as_ref().unwrap();
    assert_eq!(texts(bare), made);
    assert!(again.is_none(), "the note is on show already");
    // One wish is open at a time: the next tap's note takes the place of this one.
    let ids = |scene: &Option<Scene>| {
        texts(scene.as_ref().unwrap()).iter().map(|note| note.0.to_string()).collect::<Vec<_>>()
    };
    assert_eq!((ids(work), ids(back)), (vec!["work-m1".to_string()], vec!["wish-m2".to_string()]));
    assert!(rest.iter().all(Option::is_none), "a tap on no wish, and on a note with no choice, show nothing");
    // The page under the note is the page as it was.
    let under = back.as_ref().unwrap();
    assert_eq!((&under.characters, &under.notes), (&bare.characters, &bare.notes));
    assert_eq!((drawn(bare, "m1"), drawn(bare, "m2")), (Some((State::Idle, 1)), Some((State::Busy, 0))));

    assert_eq!(w.everything(&repo), before, "a tap wrote something");
}

/// The files of the checkout that are the person's, and the refs that are not a run's branch.
pub fn persons(w: &World, repo: &Path) -> (support::worked::Files, Vec<String>) {
    let inside = |path: &Path| path.components().any(|part| part.as_os_str() == ".git");
    let mut files = tree(repo);
    let (index, head) = (repo.join(".git").join("index"), repo.join(".git").join("HEAD"));
    files.retain(|path, _| !inside(path) || *path == index || *path == head);
    let refs = w.told(repo, &["for-each-ref", "--format=%(refname) %(objectname)"]);
    (files, refs.lines().filter(|line| !line.starts_with("refs/heads/jellymoon/")).map(str::to_string).collect())
}

/// The run branches of a repo.
pub fn branches(w: &World, repo: &Path) -> Vec<String> {
    let listed = w.told(repo, &["for-each-ref", "--format=%(refname:short)", "refs/heads/jellymoon"]);
    listed.lines().map(str::to_string).collect()
}

pub fn holds(repo: &Path, file: &str) -> Option<String> {
    fs::read_to_string(repo.join(file)).ok()
}
