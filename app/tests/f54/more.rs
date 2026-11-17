//! `More like this`, typed under work that waits or was kept, carried as the window carries it
//! (`support/tapper.rs`): core's `wish_after` makes the follow-up, and its run is asked for as a typed wish's is.
//! Whether one may be made, where it stands and every refusal are core's.

use std::fs;

use serde_json::json;

use super::support::day::DAY;
use super::support::worked::tree;
use super::{DONE, MORE, MORE_HINT, UNKEPT, UNSTARTED, WORK, World, acted, plain, quoted, ready, scene, text, typed};
use crate::support::wished::SAID;

const NEXT: &str = "and the downloader too";

fn acts(w: &World, acts: &[String]) -> Vec<serde_json::Value> {
    acted(w, &acts.iter().map(String::as_str).collect::<Vec<_>>())
}

/// The ids of the characters of the day's made wishes, in the order `layout` places them.
fn drawn(w: &World) -> Vec<String> {
    let placed = w.placed();
    let made = |id: &String| id.strip_prefix('m').is_some_and(|n| n.parse::<u8>().is_ok());
    let stands = |piece: &&super::support::Piece| ["hero", "character"].contains(&piece.kind());
    let ids = placed.iter().filter(stands).map(|piece| piece.id().to_string()).filter(|id| made(id));
    let mut ids: Vec<String> = ids.collect();
    ids.dedup();
    ids
}

#[test]
fn a_follow_up_typed_under_work_that_waits_is_made_by_core_after_its_wish_and_its_run_starts() {
    let w = ready();
    let first = w.ran("edits", 0, "completed");
    let other = w.wishes();
    // The made-up agent's turn for a follow-up: it works on in the copy the followed work is in.
    w.plays("follow", 0);
    let bin = w.install().parent().unwrap().display().to_string();
    let note = format!("work-{first}");
    let printed =
        acts(&w, &[format!("bin:{bin}"), format!("poke:{first}"), format!("type:{note}:{NEXT}"), "start".into()]);
    let first_shown = scene(&printed[0]);
    let opened = text(&first_shown, &note).map(|note| (note.0, note.3.len(), note.4));
    assert_eq!(opened, Some((WORK, 3, Some((MORE, MORE_HINT)))));
    let (kept, shown) = typed(&printed[1]);
    assert!(kept, "core kept the follow-up, so the field is emptied");
    assert!(plain(&shown).iter().all(|(title, _)| ![UNKEPT, UNSTARTED].contains(title)), "{:?}", plain(&shown));
    // The work still waits, with its choices and the offer: another follow-up may be made of it.
    assert_eq!(text(&shown, &note).map(|note| (note.0, note.3.len(), note.4)), opened);
    // Core made it: the words as typed, after the wish it follows, and for that wish's project.
    let made = w.wish("m3");
    assert_eq!((&made["said"], &made["after"], &made["project"]), (&json!(NEXT), &json!(first), &json!("alpha")));
    // Its run was started as a typed wish's is, a process of its own, and the asking had nothing to say.
    assert_eq!(printed[2], json!(null));
    w.run_ended("m3");
    assert_eq!(w.wish("m3")["state"], "done", "{}", w.wish("m3"));
    // The page draws the day's made wishes in core's order: the follow-up where core keeps it.
    let kept: Vec<String> =
        ["m1", "m2", "m3"].iter().map(|id| w.wish(id)["id"].as_str().unwrap().to_string()).collect();
    assert_eq!((drawn(&w), kept), (vec![first, other, "m3".to_string()], vec!["m1".into(), "m2".into(), "m3".into()]));

    // A follow-up of the follow-up: its work waits now, so the same is offered under it.
    let printed = acts(&w, &["poke:m3".into(), "type:work-m3:and once more".into(), "start".into()]);
    assert!(typed(&printed[1]).0);
    assert_eq!(w.wish("m4")["after"], "m3");
    // No command is installed for this window: core says why no run was started, under the app's title.
    let said = scene(&printed[2]);
    let (title, body) = plain(&said)[0];
    assert!(title == UNSTARTED && body.starts_with(&format!("{}: ", quoted("and once more"))), "{body}");
}

#[test]
fn a_follow_up_typed_under_work_that_was_kept_is_made_by_core() {
    let w = ready();
    let wish = w.ran("edits", 0, "completed");
    w.runs(&["keep"], &wish);
    let note = format!("wish-{wish}");
    let printed = acts(&w, &[format!("poke:{wish}"), format!("type:{note}:{NEXT}")]);
    let kept = (DONE, vec!["“make the uploader retry twice”"], "", vec![], Some((MORE, MORE_HINT)));
    assert_eq!((quoted(SAID).as_str(), text(&scene(&printed[0]), &note)), (kept.1[0], Some(kept.clone())));
    let (taken, shown) = typed(&printed[1]);
    assert!(taken);
    assert_eq!(text(&shown, &note), Some(kept));
    assert_eq!((&w.wish("m2")["said"], &w.wish("m2")["after"]), (&json!(NEXT), &json!(wish)));
}

#[test]
fn a_follow_up_core_refuses_is_drawn_in_core_s_words_and_the_field_keeps_what_was_typed() {
    let w = ready();
    let wish = w.ran("edits", 0, "completed");
    let note = format!("work-{wish}");
    // Words core does not keep as a wish.
    let long = "x".repeat(201);
    let before = tree(&w.at("jm"));
    let printed = acts(&w, &[format!("poke:{wish}"), format!("type:{note}:{long}")]);
    let (kept, shown) = typed(&printed[1]);
    let (title, body) = plain(&shown)[0];
    assert!(!kept && title == UNKEPT && body.contains("200 characters"), "{body}");
    assert!(text(&shown, &note).is_some_and(|note| note.4 == Some((MORE, MORE_HINT))));
    assert!(tree(&w.at("jm")) == before, "a refused follow-up wrote something");

    // The day's eight: with eight made wishes the ninth is refused, in core's words.
    for _ in 0..7 {
        w.wishes();
    }
    let before = tree(&w.at("jm"));
    let printed = acts(&w, &[format!("poke:{wish}"), format!("type:{note}:{NEXT}")]);
    let (kept, shown) = typed(&printed[1]);
    let why = format!("{}: the page of {DAY} has its 8 made wishes already, so this one was not kept.", quoted(NEXT));
    assert_eq!((kept, plain(&shown)[0]), (false, (UNKEPT, why.as_str())));
    assert!(text(&shown, &note).is_some_and(|note| note.4 == Some((MORE, MORE_HINT))));
    assert!(tree(&w.at("jm")) == before, "the ninth wish wrote something");
}

#[test]
fn a_follow_up_to_work_that_was_tossed_meanwhile_is_refused_in_core_s_words() {
    // A page left open while the work was tossed elsewhere: its note with the offer is still on show.
    let w = ready();
    let wish = w.ran("edits", 0, "completed");
    let run = w.wish(&wish)["run"].as_str().unwrap().to_string();
    w.runs(&["toss"], &wish);
    let note = format!("work-{wish}");
    let stale = json!({"type": "text", "id": note, "title": WORK,
        "field": {"offer": MORE, "hint": MORE_HINT, "most": 200}});
    let left = w.at("tmp").join("left.json");
    fs::write(&left, stale.to_string()).unwrap();
    let before = tree(&w.at("jm"));
    let printed = acts(&w, &[format!("left:{}", left.display()), format!("type:{note}:{NEXT}")]);
    let (kept, shown) = typed(&printed[0]);
    let why = format!("{}: the work of run {run} was tossed, so it cannot be followed.", quoted(NEXT));
    assert_eq!((kept, plain(&shown)[0]), (false, (UNKEPT, why.as_str())));
    assert!(tree(&w.at("jm")) == before, "a refused follow-up wrote something");

    // A tap on a choice of work that waits leaves the offer under it.
    let w = ready();
    let wish = w.ran("edits", 0, "completed");
    let printed = acts(&w, &[format!("poke:{wish}"), format!("choose:work-{wish}:0"), "run".into()]);
    let shown = scene(printed.last().unwrap());
    assert!(text(&shown, &format!("work-{wish}")).is_some_and(|note| note.4 == Some((MORE, MORE_HINT))));
}

#[test]
fn a_wish_whose_work_was_tossed_has_no_field_to_type_in() {
    let w = ready();
    let wish = w.ran("edits", 0, "completed");
    w.runs(&["toss"], &wish);
    // Opened now, the wish has no offer, and so no field to type in.
    let printed = acts(&w, &[format!("poke:{wish}"), format!("type:wish-{wish}:{NEXT}")]);
    assert_eq!(text(&scene(&printed[0]), &format!("wish-{wish}")).map(|note| note.4), Some(None));
    assert_eq!(printed[1], json!(null));
}
