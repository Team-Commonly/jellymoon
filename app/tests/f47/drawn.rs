//! What the second gate on F47 found. A page keeps a file the person's tools obey only when that file's name was
//! drawn, whole, on the note the yes was given under. These are asserted on what `layout` places, at the window's
//! size, and not on the scene: a yes is placed only under every name drawn to its last character, a tap finds
//! only a placed option, and work with more such files than a note names, or with one whose name is not printed as
//! it is, has no yes on a page.

use std::fs;
use std::path::Path;

use serde_json::Value;
use sticker_kit::{Note, Scene};

use super::named::{AGAIN, ASKS, TOLD, left};
use super::{CHOICES, KEPT, WORK, holds, texts};
use crate::support::Piece;
use crate::support::day::DAY;
use crate::support::wished::World;
use crate::support::worked::finished;

const OVER: &str = "A note cannot name every file of this work that your tools obey, each whole and as it is, so a \
                    page does not keep it. It is kept from the command line, with `jellymoon run keep`.";
const SIZES: [&str; 2] = ["size:1440x900", "size:960x640"];

/// A text with no space in it: a text wrapped onto several lines is compared so, as a line ends where a space
/// was or inside a long word.
fn bare(text: &str) -> String {
    text.replace(' ', "")
}

/// What `layout` placed of one note: the texts of its sheet, [`bare`], and the words of each placed option of it.
fn placed(line: &str, note: &str) -> (String, Vec<String>) {
    let pieces: Vec<Piece> = serde_json::from_str::<Vec<Value>>(line).unwrap().into_iter().map(Piece).collect();
    let of = |kind: &'static str| pieces.iter().filter(move |piece| piece.kind() == kind && piece.id() == note);
    let texts = |piece: &Piece| piece.runs().iter().map(|run| bare(run["text"].as_str().unwrap())).collect::<String>();
    (of("note").map(texts).collect(), of("option").map(|option| option.said()).collect())
}

fn keep(w: &World, repo: &Path, more: &[&str]) -> Value {
    let (logs, repo) = (w.at("logs"), repo.to_str().unwrap());
    let args = ["run", "keep", "--wish", "m1", "--date", DAY, "--repo", repo, "--logs", logs.to_str().unwrap()];
    w.jellymoon(&[&args[..], more].concat())
}

/// What `run keep --ask` names of the work: every file of it the person's tools obey.
pub fn asked(w: &World, repo: &Path) -> Vec<String> {
    let asked = keep(w, repo, &["--ask"]);
    asked["settings"].as_array().unwrap().iter().map(|name| name.as_str().unwrap().to_string()).collect()
}

/// A clash's choice, as a day's page has one: the note after the opened wish's on a crowded page.
fn crowd(w: &World) -> String {
    let options = ["keep the first one", "keep the second one", "neither"].map(str::to_string).to_vec();
    let (title, footer) = ("Two things you said clash.".to_string(), "You can say later.".to_string());
    let note = Note::Choice { id: "clash-1".into(), title, options, footer, struck_first: false, whole: true };
    let file = w.at("crowd.json");
    fs::write(&file, serde_json::to_string(&note).unwrap()).unwrap();
    format!("left:{}", file.display())
}

/// The work of `script` asked about at each size, on a page with a clash's choice under the note when `crowded`:
/// whether the yes was placed. It is placed only under every name the work has, each drawn whole and none
/// counted, and then the tap keeps those files; where it is not placed, a tap on its place is nothing.
fn yes_under_every_name(script: &str, crowded: bool) -> Vec<bool> {
    let at = |size: &str| {
        let (w, repo) = finished(script);
        let names = asked(&w, &repo);
        assert!(names.len() >= 3 && names.iter().all(|name| !repo.join(name).exists()), "{names:?}");
        let before = w.everything(&repo);
        let first = if crowded { vec![crowd(&w)] } else { Vec::new() };
        let mut acts: Vec<&str> = first.iter().map(String::as_str).collect();
        acts.extend([size, "poke:m1", "choose:work-m1:1", "run", "placed", "choose:sure-m1:1"]);
        let lines = w.taps("logs", &acts).ended();
        let [.., drawn, tapped] = &lines[..] else { panic!("{lines:?}") };
        assert_eq!(w.everything(&repo), before, "{script} at {size}: something was changed before a keep was run");
        let (said, options) = placed(drawn, "sure-m1");
        assert!(said.starts_with(&bare(ASKS)), "{said}");
        if options.is_empty() {
            assert_eq!(tapped, "null", "a tap on a yes that was not placed was carried: {said}");
            return false;
        }
        let whole = names.iter().all(|name| said.contains(&bare(name)));
        assert!(whole && !said.contains("more") && said.ends_with(&bare(TOLD)), "a yes under {said:?}");
        assert_eq!(options, AGAIN);
        // The same taps again, and the keep run: what lands is what was named.
        acts.push("run");
        let lines = w.taps("logs", &acts).ended();
        let kept: Scene = serde_json::from_str(lines.last().unwrap()).unwrap();
        assert_eq!(texts(&kept)[0].1, KEPT);
        assert!(names.iter().all(|name| repo.join(name).is_file()), "{names:?}");
        true
    };
    SIZES.iter().map(|size| at(size)).collect()
}

#[test]
fn on_a_crowded_page_the_yes_is_placed_only_under_every_name_drawn_whole_and_keeps_those() {
    // Three files, of which the reviewer's page drew one, counted two and kept all three.
    assert_eq!(yes_under_every_name("crowd", true), [true, true]);
    assert_eq!(yes_under_every_name("six", false), [true, true], "six names of a line each have room");
}

// A card is as high as what it holds, so names that took more lines than the old strip of notes had are drawn
// whole, with their yes. A yes is still placed only under every name drawn whole (`yes_under_every_name` asserts
// it of each page); a body that is cut or counted has none, which the kit's `tests/text_note.rs` holds.
#[test]
fn names_of_many_lines_are_drawn_whole_with_their_yes_and_keep_those() {
    assert_eq!(yes_under_every_name("long", false), [true, true]);
    assert_eq!(yes_under_every_name("longs", false), [true, true]);
}

#[test]
fn work_of_more_such_files_than_a_note_names_or_of_a_name_not_printed_as_it_is_has_no_yes_on_a_page() {
    // The last such file of each work. `masked` has one, in a name with the shape of a credential: it is printed
    // as another word, so a yes under it would be a yes to a file the person was not told of.
    let masked = format!(".claude/AK{}{}.md", "IA", "Q7".repeat(8));
    // The others are printed whole, and a person does not read them as they are: `blank` has a letter that
    // draws as a blank in each of its two, `joined` a joiner between two letters, `counted` reads like a count.
    let works = [
        ("seven", 7, "r6/CLAUDE.md"),
        ("eight", 8, "r7/CLAUDE.md"),
        ("masked", 1, masked.as_str()),
        ("blank", 2, "\u{3164}/CLAUDE.md"),
        ("joined", 2, "CLAUDE.md"),
        ("counted", 2, ".claude/+5 more"),
    ];
    for (script, count, last) in works {
        let (w, repo) = finished(script);
        let names = asked(&w, &repo);
        assert_eq!((names.len(), names[count - 1] == last), (count, script != "masked"), "{names:?}");
        assert!(script != "masked" || names[0].contains("[redacted]"), "{names:?}");
        let before = w.everything(&repo);
        // The note says so and has no keep. A page left on show with one, by whatever made it: a tap on `Keep
        // it`, and on the yes under words that name every file and under the note's own, is answered alike.
        let forged: Vec<String> = names.iter().cloned().chain([TOLD.to_string()]).collect();
        let over = vec![OVER.to_string(), "Nothing was changed.".to_string()];
        let (id, options) = ("work-m1".to_string(), CHOICES.map(str::to_string).to_vec());
        let stale = Note::Text { id, title: WORK.into(), body: vec![], options, quoted: String::new(), field: None };
        fs::write(w.at("stale.json"), serde_json::to_string(&stale).unwrap()).unwrap();
        let stale = format!("left:{}", w.at("stale.json").display());
        let taps: [(String, &[&str]); 4] = [
            (String::new(), &["poke:m1", "placed", "choose:work-m1:2"]),
            (stale, &["choose:work-m1:1", "run", "placed"]),
            (left(&w, &forged), &["choose:sure-m1:1", "run", "placed"]),
            (left(&w, &over), &["choose:sure-m1:1", "run", "placed"]),
        ];
        for (first, rest) in taps {
            let acts: Vec<&str> =
                [first.as_str()].into_iter().filter(|act| !act.is_empty()).chain(rest.iter().copied()).collect();
            let lines = w.taps("logs", &acts).ended();
            let drawn = lines.iter().find(|line| line.starts_with('[')).unwrap();
            let note = if first.is_empty() { "work-m1" } else { "sure-m1" };
            let (said, options) = placed(drawn, note);
            // The work's own note counts its files and names none before `Show me`; the one that asks names none.
            assert!(said.ends_with(&bare(if first.is_empty() { OVER } else { "Nothing was changed." })), "{said}");
            assert!(said.contains(&bare(OVER)) && (first.is_empty() || !said.contains("CLAUDE.md")), "{acts:?}");
            // The work's note says what its one choice that changes something does, and of no keep.
            let tossed = "Itwaitsforyouryes:Tossitthrowsitaway.";
            assert!(!first.is_empty() || (said.contains(tossed) && !said.contains("Keepitputs")), "{said}");
            assert_eq!(options, ["Show me", "Toss it"], "{acts:?}");
            // Where the keep was, in the third place, nothing is placed, and a tap there is nothing.
            assert_eq!(lines.last().map(String::as_str) == Some("null"), first.is_empty(), "{lines:?}");
            assert_eq!(w.everything(&repo), before, "{acts:?} changed something");
        }
        // `jellymoon-app layout` places the same: the sentence, and no keep.
        let printed = w.placed();
        let of =
            |kind: &'static str| printed.iter().filter(move |piece| piece.kind() == kind && piece.id() == "work-m1");
        let options: Vec<String> = of("option").map(|option| option.said()).collect();
        assert_eq!(options, ["Show me", "Toss it"]);
        assert!(of("note").all(|note| bare(&note.said()).ends_with(&bare(OVER))) && of("note").count() == 1);
        // From the command line the work is kept, with the word for those files.
        assert_eq!(keep(&w, &repo, &["--with-settings"])["settings"].as_array().unwrap().len(), count);
        assert_eq!(holds(&repo, last), Some("rules from the run\n".into()));
    }
}
