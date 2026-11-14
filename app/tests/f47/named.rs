//! What the gate on F47 found. A keep's one more yes is a yes to the files the note named: every file of the whole
//! work the person's tools obey is named or counted before anything is kept, past what a list of the work holds
//! too; a yes to a note that names other files is refused; and a keep that was never asked once more lands no
//! such file, where only the merge brings one. And the guards of `Show me` that had no test.

use std::fs;

use sticker_kit::Note;

use super::{CHOICES, WORK, branches, holds, persons, quoted, texts};
use crate::support::wished::{SAID, World};
use crate::support::worked::finished;

const ASKS: &str = "This work touches files your tools obey.";
const TOLD: &str = "Nothing was changed. Kept, they are in your checkout, where your tools read them.";
const AGAIN: [&str; 3] = ["Show me", "Keep it, with those files", "Toss it"];
const NOT_KEPT: &str = "That work was not kept.";

fn run(w: &World) -> String {
    w.kept()["run"].as_str().unwrap().to_string()
}

/// A file that holds the note that asks once more, with `named` as the files it names: a page left on show.
fn left(w: &World, named: &str) -> String {
    let options = AGAIN.map(str::to_string).to_vec();
    let body = vec![named.to_string(), TOLD.to_string()];
    let note = Note::Text { id: "sure-m1".into(), title: ASKS.into(), body, options };
    let file = w.at("left.json");
    fs::write(&file, serde_json::to_string(&note).unwrap()).unwrap();
    format!("left:{}", file.display())
}

#[test]
fn every_file_the_person_s_tools_obey_in_work_of_more_files_than_are_listed_is_named_before_any_is_kept() {
    let (w, repo) = finished("many");
    let before = w.everything(&repo);
    let go = w.at("go");
    let hold = format!("hold:{}", go.display());
    let mut window = w.taps("logs", &["poke:m1", "choose:work-m1:1", "run", &hold, "choose:sure-m1:1", "run"]);
    window.printed(3);
    assert_eq!(w.everything(&repo), before, "the first tap kept something");
    fs::write(&go, "").unwrap();
    let showed = window.showed();
    let [opened, _, asks, _, kept] = &showed[..] else { panic!("five lines") };

    // 236 files, of which a list holds 200: the counts are of those, six are named, and the rest is counted.
    let (said, run) = (quoted(SAID), run(&w));
    let why = format!("the work of run {run} waits: files made 198, changed 1, removed 1 of its first 200.");
    let names = "CLAUDE.md, a.txt, b.txt, f000.txt, f001.txt, f002.txt, and 230 more";
    let note = ("work-m1", WORK, vec![said.as_str(), why.as_str(), names], CHOICES.to_vec());
    assert_eq!(texts(opened.as_ref().unwrap()), [note]);
    // Two of the three are past the list, and all three are named.
    let obeyed = "CLAUDE.md, zz/.claude/settings.json, zz/.envrc";
    assert_eq!(texts(asks.as_ref().unwrap()), [("sure-m1", ASKS, vec![obeyed, TOLD], AGAIN.to_vec())]);
    let kept = texts(kept.as_ref().unwrap()).iter().map(|note| (note.0, note.1)).collect::<Vec<_>>();
    assert_eq!(kept, [("said-m1", "That work was kept.")]);
    let below = repo.join("zz");
    assert!(repo.join("CLAUDE.md").is_file() && below.join(".envrc").is_file());
    assert!(below.join(".claude").join("settings.json").is_file());
}

#[test]
fn work_whose_only_file_the_person_s_tools_obey_is_past_the_list_is_asked_about_and_kept_at_the_second_yes() {
    let (w, repo) = finished("past");
    let before = w.everything(&repo);
    let go = w.at("go");
    let hold = format!("hold:{}", go.display());
    let mut window = w.taps("logs", &["poke:m1", "choose:work-m1:1", "run", &hold, "choose:sure-m1:1", "run"]);
    window.printed(3);
    assert_eq!(w.everything(&repo), before, "the first tap kept something");
    fs::write(&go, "").unwrap();
    let showed = window.showed();
    let [_, _, asks, _, kept] = &showed[..] else { panic!("five lines") };
    assert_eq!(texts(asks.as_ref().unwrap()), [("sure-m1", ASKS, vec!["zz/.envrc", TOLD], AGAIN.to_vec())]);
    assert_eq!(texts(kept.as_ref().unwrap())[0].1, "That work was kept.");
    assert_eq!(holds(&repo.join("zz"), ".envrc"), Some("PATH_add bin\n".into()));
}

#[test]
fn a_yes_to_a_note_that_names_other_files_than_the_work_s_is_refused_and_nothing_is_changed() {
    let (w, repo) = finished("many");
    let before = w.everything(&repo);
    let why = format!(
        "the files of the work of run {} that your tools obey are not the ones that note named, so it was not kept, \
         and nothing was changed.",
        run(&w)
    );
    // A note that names the first of the three alone, one that names one more than there are, and one that names
    // none: a yes to any of them is a yes to other files than the work's.
    for named in ["CLAUDE.md", "CLAUDE.md, zz/.claude/settings.json, zz/.envrc, .envrc", ""] {
        let showed = w.taps("logs", &[&left(&w, named), "choose:sure-m1:1", "run"]).showed();
        let [at_once, refused] = &showed[..] else { panic!("two lines") };
        assert_eq!(texts(at_once.as_ref().unwrap()), [("busy-m1", "That work is being kept.", vec![], vec![])]);
        assert_eq!(texts(refused.as_ref().unwrap()), [("said-m1", NOT_KEPT, vec![why.as_str()], vec![])]);
        assert_eq!(w.everything(&repo), before, "a yes to {named:?} changed something");
    }
    // The same note with the words core would draw now is the yes the work takes.
    let named = "CLAUDE.md, zz/.claude/settings.json, zz/.envrc";
    let showed = w.taps("logs", &[&left(&w, named), "choose:sure-m1:1", "run"]).showed();
    assert_eq!(texts(showed[1].as_ref().unwrap())[0].1, "That work was kept.");
    assert!(repo.join("CLAUDE.md").is_file());
}

#[test]
fn a_keep_that_was_not_asked_once_more_lands_no_file_the_person_s_tools_obey_that_only_the_merge_brings() {
    let (w, repo) = finished("edits");
    // The person gives `a.txt`, which the work edits, a name their tools obey. No file of the work has such a
    // name, so nothing is asked once more; the merge follows the new name, and would write the edit there.
    w.told(&repo, &["mv", "a.txt", "CLAUDE.md"]);
    let who = ["-c", "user.name=A Person", "-c", "user.email=person@example.test"];
    w.told(&repo, &[&who[..], &["commit", "-q", "-m", "a new name"]].concat());
    let before = (persons(&w, &repo), branches(&w, &repo));
    let rules = holds(&repo, "CLAUDE.md");
    let showed = w.taps("logs", &["poke:m1", "choose:work-m1:1", "run"]).showed();
    let [opened, _, refused] = &showed[..] else { panic!("three lines") };
    assert_eq!(texts(opened.as_ref().unwrap())[0].3, CHOICES.to_vec(), "nothing of the work itself is obeyed");
    let why = format!(
        "the work of run {} would put a file your tools obey in your checkout that no note named, so it was not \
         kept, and nothing was changed.",
        run(&w)
    );
    assert_eq!(texts(refused.as_ref().unwrap()), [("said-m1", NOT_KEPT, vec![why.as_str()], vec![])]);
    assert_eq!((persons(&w, &repo), branches(&w, &repo)), before, "the keep changed something");
    assert_eq!(holds(&repo, "CLAUDE.md"), rules);
}

#[test]
fn show_me_passes_over_a_removed_file_asks_for_eight_files_at_most_and_goes_on_from_the_first_past_the_last() {
    // `a.txt`, `b.txt` which the work removed, seven files that are text and cannot be shown, and `n.txt`.
    let (w, _) = finished("walls");
    let show = ["choose:work-m1:0", "run"];
    let acts = [&["poke:m1"][..], &show, &show, &show].concat();
    let showed = w.taps("logs", &acts).showed();
    let file = |at: usize| {
        let notes = texts(showed[at].as_ref().unwrap());
        (notes[1].0, notes[1].1)
    };
    let files = [file(2), file(4), file(6)];
    // After file 1 the eighth file asked for is `n.txt`, as the removed one is not asked for; after it, the first.
    let first = ("file-1-m1", "File 1 of the work: a.txt");
    assert_eq!(files, [first, ("file-10-m1", "File 10 of the work: n.txt"), first]);
}
