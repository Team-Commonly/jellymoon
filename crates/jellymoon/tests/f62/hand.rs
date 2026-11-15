//! A list written by hand. The file is untrusted when it is read back: one that is not what Jellymoon writes,
//! in any line, is not used at all, in a fixed sentence that names a line by its place; and a line that passes
//! starts a run only in the folder it names, as that folder really is, after the checks every listed folder
//! gets.

use std::fs;

use serde_json::{Value, json};

use super::refused::link;
use super::*;
use crate::support::skipped;

/// The list of `w` is not used: the three commands refuse it in the fixed sentence, nothing listed is offered
/// or run in, and the file is as it was.
fn not_used(w: &World, repo: &std::path::Path, why: &str) {
    let before = w.all();
    for args in [&["project", "list"][..], &["project", "forget", "alpha"]] {
        assert_eq!(refused(&w.run(args)), unused(why), "{args:?}");
    }
    let ran = w.add(repo);
    assert_eq!(refused(&ran), unused(why));
    names_no_path(w, &ran);
    assert_eq!(w.heard_for(&[]), cannot(NO_PROJECT), "{why}");
    assert!(w.all() == before, "a list that is not used was written over, or something was made: {why}");
    // One wish serves every list of a world: a day holds eight.
    if w.made_wishes().is_none() {
        w.wish("alpha");
    }
    let ran = w.run(&["run", "--wish", "m1", "--date", DAY]);
    let none = format!(
        "jellymoon: no folder of the name the wish gives was typed in on {DAY}, and {}; or name the folder with \
         --repo\n",
        unused(why).trim_start_matches("jellymoon: ").trim_end()
    );
    assert_eq!(refused(&ran), none);
    names_no_path(w, &ran);
    w.started_nothing(repo);
}

#[test]
fn a_line_jellymoon_would_not_have_written_makes_the_whole_list_unused() {
    let w = heard_world();
    let repo = w.repo("alpha");
    let real = w.real(&repo);
    let sep = std::path::MAIN_SEPARATOR;
    let not_one = "project 1 of it is not one Jellymoon wrote";
    let one = |name: &str, folder: &str, used: &str| json!([{"name": name, "folder": folder, "used": used}]);
    let around = format!("{}{sep}beta{sep}..{sep}alpha", w.real(&w.at("work")));
    let lists: Vec<(Value, &str)> = vec![
        (one("alpha", &around, DAY), not_one),                       // a `..` in the path
        (one("alpha", &format!("{real}{sep}.{sep}"), DAY), not_one), // a `.`
        (one("alpha", &format!("{real}{sep}"), DAY), not_one),       // a separator at its end
        (one("alpha", &format!("work{sep}alpha"), DAY), not_one),    // not a full path
        (one("alpha", "alpha", DAY), not_one),
        (one("beta", &real, DAY), not_one), // a name that is not the folder's
        (one(&format!("work{sep}alpha"), &real, DAY), not_one), // a name that is a path
        (one("work/alpha", &real, DAY), not_one),
        (one("al\u{2065}pha", &format!("{real}{sep}al\u{2065}pha"), DAY), not_one), // something unseen
        (one("", &real, DAY), not_one),
        (one("alpha", &real, "2026-3-10"), not_one), // a day that is none
        (one("alpha", &real, "yesterday"), not_one),
        (one("alpha", &format!("{real}{sep}{}{sep}alpha", "x".repeat(5000)), DAY), not_one), // too long
        (json!([w.line("alpha", &real), w.line("alpha", &real)]), "project 2 of it has the name of one before it"),
    ];
    for (projects, why) in lists {
        w.write_list(json!({ "projects": projects }).to_string().as_bytes());
        not_used(&w, &repo, why);
    }
}

#[test]
fn a_file_that_is_not_a_list_is_not_used() {
    let w = heard_world();
    let repo = w.repo("alpha");
    let line = w.line("alpha", &w.real(&repo));
    let shape = "it is not a list as Jellymoon writes one";
    let more: Vec<Value> = (0..101).map(|n| w.line(&format!("p{n}"), &format!("{}{n}", w.real(&repo)))).collect();
    let mut huge = json!({ "projects": [line] }).to_string().into_bytes();
    huge.resize(512 * 1024 + 1, b' ');
    let mut field = line.clone();
    field["run"] = json!("now");
    let files: Vec<(Vec<u8>, &str)> = vec![
        (b"".to_vec(), shape),
        (b"alpha\n".to_vec(), shape),
        (b"[]".to_vec(), shape),
        (json!({"projects": {}}).to_string().into_bytes(), shape),
        (json!({"projects": [line], "more": 1}).to_string().into_bytes(), shape),
        (json!({ "projects": [field] }).to_string().into_bytes(), shape),
        (json!({ "projects": more }).to_string().into_bytes(), shape),
        (b"{\"projects\": [\xff\xfe]}".to_vec(), "it is not text"),
        (huge, "it is too large"),
    ];
    for (bytes, why) in files {
        w.write_list(&bytes);
        not_used(&w, &repo, why);
    }
}

#[test]
fn a_list_others_can_write_to_or_that_is_a_link_is_not_used() {
    let w = heard_world();
    let repo = w.repo("alpha");
    let good = json!({"projects": [w.line("alpha", &w.real(&repo))]}).to_string();
    #[cfg(unix)]
    for mode in [0o620, 0o602, 0o666] {
        w.write_list(good.as_bytes());
        fs::set_permissions(w.list_file(), std::os::unix::fs::PermissionsExt::from_mode(mode)).unwrap();
        not_used(&w, &repo, "others can write to it");
    }
    // A link to a list that would be used is no list: what it leads to is not read.
    w.write_list(good.as_bytes());
    fs::rename(w.list_file(), w.at("elsewhere.json")).unwrap();
    #[cfg(unix)]
    let linked = std::os::unix::fs::symlink(w.at("elsewhere.json"), w.list_file());
    #[cfg(windows)]
    let linked = std::os::windows::fs::symlink_file(w.at("elsewhere.json"), w.list_file());
    if linked.is_err() {
        return skipped("this system makes no link for this user");
    }
    not_used(&w, &repo, "it is not a plain file");
    assert_eq!(fs::read_to_string(w.at("elsewhere.json")).unwrap(), good);
    // A folder at the name is none either.
    fs::remove_file(w.list_file()).unwrap();
    fs::create_dir(w.list_file()).unwrap();
    not_used(&w, &repo, "it is not a plain file");
}

#[test]
fn a_line_written_by_hand_that_names_a_link_is_listed_and_starts_nothing() {
    let w = heard_world();
    let elsewhere = w.repo("elsewhere/alpha");
    let at = w.at("work").join("alpha");
    if !link(&elsewhere, &at) {
        return skipped("this system makes no link for this user");
    }
    let folder = format!("{}{}alpha", w.real(&w.at("work")), std::path::MAIN_SEPARATOR);
    w.write_list(json!({"projects": [w.line("alpha", &folder)]}).to_string().as_bytes());
    assert_eq!(w.listed(), json!([{"name": "alpha", "used": DAY}]));
    assert_eq!(w.heard_for(&[]), cannot(NO_PROJECT));
    let ran = w.run_found("alpha", &[]);
    let why = fallen("it is not there, or is reached through a link now");
    assert_eq!(refused(&ran), format!("jellymoon: {why}\n"));
    w.started_nothing(&elsewhere);
}

// What a line written by hand can do: what `project add` does. Whoever can write Jellymoon's folder can list a
// repo of the person's own, and a run then starts in that repo and nowhere else.
#[test]
fn a_line_written_by_hand_as_jellymoon_writes_one_is_a_listed_project() {
    let w = heard_world();
    let (repo, other) = (w.repo("alpha"), w.repo("beta"));
    w.write_list(format!("{}\n", json!({"projects": [w.line("alpha", &w.real(&repo))]})).as_bytes());
    assert_eq!(printed(&w.run_found("alpha", &[]))["outcome"], "completed");
    assert_eq!((w.branches(&repo).len(), w.branches(&other).len()), (1, 0));
}

#[test]
fn a_folder_typed_in_is_still_found_when_the_list_is_not_used() {
    let w = heard_world();
    let repo = w.repo("alpha");
    w.typed_at("-work-alpha", &repo);
    w.write_list(b"not a list\n");
    assert_eq!(w.heard_for(&[]), heard(SAID, "alpha", 1));
    assert_eq!(printed(&w.run_found("alpha", &[]))["outcome"], "completed");
    assert_eq!(fs::read(w.list_file()).unwrap(), b"not a list\n");
}
