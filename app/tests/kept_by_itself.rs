//! F92 on the page: a finished change is kept by itself, and its note has one choice, which undoes it (ADR 0028;
//! `docs/development/app.md`, "A change kept by itself"). The rules and the words are core's, proved at
//! `jellymoon run` and `run undo` (`crates/jellymoon/tests/f92.rs`). Here is the app's part: what `layout` prints
//! of such a change, and the tap, carried as the window carries it by a made-up window in a process of its own
//! (`support/tapper.rs`, `support/worked.rs`). Every other test's world says that every change waits, as before
//! F92; these say that a finished change is kept by itself, as it is outside the tests. Nothing here starts the
//! real Claude Code, and no window is opened.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::fs;

use sticker_kit::{Note, Scene};
use support::Piece;
use support::wished::{SAID, World};
use support::worked::landed;

const HAS: &str = "Your folder has this change now.";
const COUNTED: &str = "1 new file, 1 changed, 1 removed.";
const UNDO: [&str; 1] = ["Undo it"];
const WAITS: &str = "Your folder is not changed yet.";
const READY: &str = "It waits for your yes: Keep it puts it in; Toss it throws it away.";
const AS_BEFORE: &str = "Your folder is as it was before the change, and the change waits for your yes.";
const NEXT: &str = "Open the wish again to see where the change stands. Here is why it was not undone:";
const EDITS: &str = "a file of this change has unsaved or staged edits, so it was not undone, and nothing was changed.";

/// One of the notes of an opened wish: its id, title, body and options.
type Text<'a> = (&'a str, &'a str, Vec<&'a str>, Vec<&'a str>);

fn texts(scene: &Option<Scene>) -> Vec<Text<'_>> {
    fn strs(all: &[String]) -> Vec<&str> {
        all.iter().map(String::as_str).collect()
    }
    fn text(note: &Note) -> Option<Text<'_>> {
        match note {
            Note::Text { id, title, body, options, .. } => {
                Some((id.as_str(), title.as_str(), strs(body), strs(options)))
            }
            _ => None,
        }
    }
    scene.as_ref().expect("a scene was shown").notes.iter().filter_map(text).collect()
}

fn of<'a>(placed: &'a [Piece], kind: &str, id: &str) -> Vec<&'a Piece> {
    placed.iter().filter(|piece| piece.kind() == kind && piece.id() == id).collect()
}

#[test]
fn layout_prints_the_note_of_a_change_kept_by_itself_with_its_count_and_one_choice() {
    let (w, repo) = landed("edits");
    // The change is in the person's folder, with no tap.
    assert_eq!(fs::read_to_string(repo.join("n.txt")).unwrap(), "new from the run\n");
    let before = w.everything(&repo);
    let placed = w.placed();
    let [note] = of(&placed, "note", "kept-m1")[..] else { panic!("one note of the change: {placed:?}") };
    let said = note.said();
    assert!(said.contains(HAS) && said.contains(COUNTED), "{said:?}");
    assert!(!said.contains(".txt") && !said.contains(w.kept()["run"].as_str().unwrap()), "{said:?}");
    let options: Vec<String> = of(&placed, "option", "kept-m1").iter().map(|option| option.said()).collect();
    assert_eq!(options, UNDO);
    assert!(placed.iter().all(|piece| !piece.id().starts_with("work-")), "no work waits: {placed:?}");
    assert_eq!(w.everything(&repo), before, "a layout wrote something");
}

#[test]
fn undo_it_shows_core_s_answer_and_then_the_work_s_note_which_waits_for_a_yes() {
    let (w, repo) = landed("edits");
    let showed = w.taps("logs", &["poke:m1", "choose:kept-m1:0", "run"]).showed();
    let [opened, at_once, undone] = &showed[..] else { panic!("three acts") };
    assert_eq!(texts(opened), [("kept-m1", HAS, vec![COUNTED], UNDO.to_vec())]);
    assert_eq!(texts(at_once), [("busy-m1", "That change is being undone.", vec![], vec![])]);
    let said = format!("“{SAID}”");
    let work = ("work-m1", WAITS, vec![READY, said.as_str(), COUNTED], vec!["Show me", "Keep it", "Toss it"]);
    assert_eq!(texts(undone), [("said-m1", "The change was undone.", vec![AS_BEFORE], vec![]), work]);

    // The folder is as it was before the change, and nothing in it is unsaved.
    let holds = |file: &str| fs::read_to_string(repo.join(file)).ok();
    assert_eq!(
        (holds("a.txt"), holds("b.txt"), holds("n.txt")),
        (Some("a.txt as it was\n".into()), Some("b.txt as it was\n".into()), None)
    );
    assert_eq!(w.told(&repo, &["status", "--porcelain"]), "");
    // The page has the work's note from now on: it waits as any change that was not kept by itself.
    let placed = w.placed();
    assert!(placed.iter().all(|piece| !piece.id().starts_with("kept-")), "{placed:?}");
    assert_eq!(of(&placed, "note", "work-m1").len(), 1, "{placed:?}");
}

#[test]
fn an_undo_core_refuses_says_so_in_core_s_sentence_and_the_change_stays() {
    let (w, repo) = landed("edits");
    fs::write(repo.join("a.txt"), "half an edit\n").unwrap();
    let before = w.everything(&repo);
    let showed = w.taps("logs", &["poke:m1", "choose:kept-m1:0", "run"]).showed();
    let [_, _, refused] = &showed[..] else { panic!("three acts") };
    let kept = ("kept-m1", HAS, vec![COUNTED], UNDO.to_vec());
    assert_eq!(texts(refused), [("said-m1", "The change was not undone.", vec![NEXT, EDITS], vec![]), kept]);
    assert_eq!(w.everything(&repo), before, "a refused undo changed something");
}

#[test]
fn a_change_of_a_file_the_person_s_tools_obey_still_waits_for_a_yes() {
    let (w, repo) = landed("rules");
    assert!(!repo.join("CLAUDE.md").exists(), "the change was kept by itself");
    let placed = w.placed();
    let [note] = of(&placed, "note", "work-m1")[..] else { panic!("one note of the work: {placed:?}") };
    let said = note.said();
    assert!(said.contains(WAITS) && said.contains(READY), "{said:?}");
    assert!(placed.iter().all(|piece| !piece.id().starts_with("kept-")), "{placed:?}");
}

#[test]
fn a_kept_change_the_folder_is_past_or_away_from_or_that_cannot_be_read_has_no_undo_on_a_page() {
    const PAST: &str = "It cannot be undone here any more: the branch has a commit since.";
    const EARLIER: &str = "This change was put in your folder earlier.";
    const ELSEWHERE: &str = "Your folder stands somewhere else now, so the change is not undone from here.";
    const UNREAD: &str = "Its details cannot be read now.";
    let opened = |w: &World| w.taps("logs", &["poke:m1"]).showed().remove(0);
    // The person committed on top of the change: the folder has it, and it is not undone from a page.
    let (w, repo) = landed("edits");
    fs::write(repo.join("mine.txt"), "the person's own\n").unwrap();
    w.told(&repo, &["add", "mine.txt"]);
    w.told(&repo, &["-c", "user.name=t", "-c", "user.email=t@example.invalid", "commit", "-q", "-m", "own"]);
    assert_eq!(texts(&opened(&w)), [("kept-m1", HAS, vec![COUNTED, PAST], vec![])]);
    // The folder stands on another branch: nothing is claimed of it now.
    w.told(&repo, &["checkout", "-q", "-b", "other", "HEAD~2"]);
    assert_eq!(texts(&opened(&w)), [("kept-m1", EARLIER, vec![COUNTED, ELSEWHERE], vec![])]);
    assert!(w.placed().iter().all(|piece| piece.kind() != "option" || piece.id() != "kept-m1"));

    // The run's branch is gone: the change is where it was, and its files cannot be counted.
    let (w, repo) = landed("edits");
    let run = w.kept()["run"].as_str().unwrap().to_string();
    w.told(&repo, &["branch", "-q", "-D", &format!("jellymoon/run-{run}")]);
    assert_eq!(texts(&opened(&w)), [("kept-m1", HAS, vec![UNREAD], vec![])]);
}
