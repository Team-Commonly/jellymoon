//! What an earlier build kept. A name with a character nobody sees (U+FE0F after an emoji, say) was a project's
//! name before the check of a name knew those characters. Such a name is refused where a name comes in: `project
//! add`, a wish for a name that is not listed, the offers. What Jellymoon reads back from its own files is read
//! as it was written: a list with such a line is used, the line can be forgotten, and a day with such a wish is
//! still the day's made wishes.

use std::fs;

use serde_json::{Value, json};

use super::*;

/// A folder's name an earlier build listed: a word, a space and an emoji with its selector.
const OLD: &str = "app \u{2764}\u{fe0f}";
const NO_NAME: &str = "jellymoon: --project takes one folder's name of at most 100 characters: no path, nothing that \
                       cannot be seen and nothing shaped like a credential\n";

/// `jellymoon wish` for a project, with `more` arguments.
fn wish_for(w: &World, project: &str, more: &[&str]) -> crate::support::Ran {
    w.run(&[&["wish", "--date", DAY, "--say", SAID, "--project", project][..], more].concat())
}

#[test]
fn a_list_an_earlier_build_wrote_is_used_and_its_line_of_a_name_now_refused_can_be_forgotten() {
    let w = heard_world();
    let (beta, old, gamma) = (w.repo("beta"), w.repo(OLD), w.repo("gamma"));
    let lines = json!([w.line("beta", &w.real(&beta)), w.line(OLD, &w.real(&old))]);
    w.write_list(format!("{}\n", json!({ "projects": lines })).as_bytes());
    assert_eq!(w.listed(), json!([{"name": "beta", "used": DAY}, {"name": OLD, "used": DAY}]));
    // A sound project of that list runs, and a third is added.
    assert!(printed(&wish_for(&w, "beta", &["--run"]))["run"].is_string());
    assert_eq!(w.record(&w.run_ended("m1"))["outcome"], "completed");
    assert_eq!(printed(&w.add(&gamma))["name"], "gamma");
    // The old name is a listed project still: a wish for it is kept and runs in its folder. It is not offered.
    assert_eq!(printed(&w.run_found(OLD, &[]))["outcome"], "completed");
    assert_eq!((w.branches(&old).len(), w.kept("m2")["project"].as_str()), (1, Some(OLD)));
    // Forgotten, the line is gone, and the name is one that comes in: it is not added and not wished for.
    assert_eq!(printed(&w.run(&["project", "forget", OLD]))["name"], OLD);
    assert_eq!(w.listed(), json!([{"name": "beta", "used": DAY}, {"name": "gamma", "used": DAY}]));
    let before = w.all();
    assert!(refused(&w.add(&old)).starts_with("jellymoon: --repo is not a folder a project can be named after"));
    assert_eq!(refused(&wish_for(&w, OLD, &[])), NO_NAME);
    assert!(w.all() == before, "a refusal wrote something");
}

#[test]
fn a_listed_project_of_a_name_now_refused_is_not_offered() {
    let w = heard_world();
    let old = w.repo(OLD);
    w.write_list(format!("{}\n", json!({"projects": [w.line(OLD, &w.real(&old))]})).as_bytes());
    assert_eq!(w.heard_for(&[]), cannot(NO_PROJECT));
}

#[test]
fn a_day_s_wishes_an_earlier_build_kept_are_used_with_a_wish_for_a_name_now_refused_among_them() {
    let w = heard_world();
    let (alpha, old) = (w.repo("alpha"), w.repo(OLD));
    printed(&w.add(&alpha));
    assert_eq!((w.wish("alpha"), w.wish("alpha")), ("m1".to_string(), "m2".to_string()));
    // The second wish as an earlier build kept one for a folder of the old name.
    let file = w.jm("pages").join(format!("{DAY}.wishes.json"));
    let mut kept: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
    kept["wishes"][1]["project"] = json!(OLD);
    fs::write(&file, format!("{kept}\n")).unwrap();
    // The day is still read: a new wish is kept, a sound wish runs, and what waits is answered.
    assert_eq!(w.wish("alpha"), "m3");
    assert_eq!(printed(&w.run(&["run", "--wish", "m1", "--date", DAY]))["outcome"], "completed");
    assert_eq!(printed(&w.run(&["run", "waiting", "--date", DAY]))["day"], DAY);
    // The kept wish for the old name runs where a run finds a folder: named with --repo, or listed.
    assert!(!refused(&w.run(&["run", "--wish", "m2", "--date", DAY])).contains("are not used"));
    assert_eq!(printed(&w.start("m2", &old))["outcome"], "completed");
    assert_eq!(w.branches(&old).len(), 1);
    // A new wish for the old name, which is on no list, is a name coming in.
    assert_eq!(refused(&wish_for(&w, OLD, &[])), NO_NAME);
}
