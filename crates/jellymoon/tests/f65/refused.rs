//! What `jellymoon project new` refuses: each in one fixed line, exit 1, that names no folder and says no
//! argument back, with the whole test folder as it was. A name that is a path, in every shape of one; a name
//! that is taken, on the list or by what is there; a link where the folder or the project would be; no git.

use std::ffi::OsString;
use std::fs;
use std::path::Path;

use serde_json::json;

use super::*;
use crate::support::{hub_key, shows, skipped};

pub const NAME: &str = "project new takes a name for the new project: one folder's name of at most 100 characters, \
                        with no dot, dash, tilde or space at its start, no dot or space at its end, no two spaces \
                        together, none of : * ? | < > \" in it, no name a system keeps for a device, nothing that \
                        cannot be seen and nothing shaped like a credential; nothing was made";
pub const LISTED: &str = "a project of that name is on the list already, so nothing was made; pick another name, or \
                          let the listed one go with `jellymoon project forget`";
pub const THERE: &str = "something is already there under that name in the folder Jellymoon makes projects in \
                         (projects in Jellymoon's folder), so nothing was made; pick another name, list what is \
                         there with `jellymoon project add --repo` and its folder, or, if a `project new` that was \
                         cut off left it, remove it and ask again";
pub const NO_FOLDER: &str = "the folder Jellymoon makes projects in (projects in Jellymoon's folder) is not a folder \
                             Jellymoon made: a link or a file is at its name; nothing was made";
pub const NO_GIT: &str = "no `git` program was found on PATH, and a new project is a git repo, so nothing was made";

/// Names Windows takes for no folder or keeps for a device, one a shell reads as the person's home, one with a
/// joiner that is not seen between two letters, one with two spaces together, and five that hold what draws
/// nothing or draws as a space (F62's check of a name, which `project add` shares).
const NO_NAME: [&str; 20] = [
    "a*b",
    "a?b",
    "a|b",
    "a<b",
    "a>b",
    "a\"b",
    "COM\u{b9}",
    "lpt\u{b2}.txt",
    "CONIN$",
    "conout$.log",
    "~",
    "~alpha",
    "al\u{200d}pha",
    "al\u{200c}pha",
    "al  pha",
    "al\u{34f}pha",
    "al\u{115f}pha",
    "al\u{3164}pha",
    "al\u{2800}pha",
    "al\u{fe0f}pha",
];

/// A link at `at` that leads to `to`. False where this system makes none for this user.
pub fn link(to: &Path, at: &Path) -> bool {
    #[cfg(unix)]
    let linked = std::os::unix::fs::symlink(to, at);
    #[cfg(windows)]
    let linked = std::os::windows::fs::symlink_dir(to, at);
    linked.is_ok()
}

/// Removes a link [`link`] made. Windows removes a link to a folder as a folder, and any other as a file.
pub fn unlink(at: &Path) {
    if fs::remove_file(at).is_err() {
        fs::remove_dir(at).unwrap();
    }
}

/// `project new` of the name is refused in that line, which says nothing of the name, and nothing is changed.
pub fn new_refused(w: &World, name: &str, why: &str) {
    let before = w.all();
    let ran = w.new_project(name);
    assert_eq!(refused(&ran), format!("jellymoon: {why}\n"), "{name:?}");
    names_no_path(w, &ran);
    assert!(w.all() == before && w.starts() == 0, "a refusal wrote something: {name:?}");
}

#[test]
fn a_name_that_is_a_path_or_is_no_project_s_name_is_refused_and_nothing_is_made() {
    let w = heard_world();
    let outside = w.folder("outside");
    let (sep, key) = (std::path::MAIN_SEPARATOR, hub_key());
    let full = outside.join("alpha").to_str().unwrap().to_string();
    let up = format!("..{sep}..{sep}work{sep}alpha");
    let odd = [
        full.as_str(),
        up.as_str(),
        "a/b",
        "a\\b",
        "/alpha",
        "alpha/",
        "..",
        ".",
        "",
        ".hidden",
        "..alpha",
        "-alpha",
        "--repo",
        "alpha.",
        "C:",
        "C:alpha",
        "a:b",
        "al\u{200b}pha",
        "al\u{2065}pha",
        "al\npha",
        " alpha",
        "alpha ",
        "NUL",
        "con",
        "Com1.txt",
        "LPT9",
        key.as_str(),
    ];
    let long = "a".repeat(101);
    for name in odd.into_iter().chain(NO_NAME).chain([long.as_str()]) {
        new_refused(&w, name, NAME);
        let ran = w.new_project(name);
        assert!(name.len() < 8 || !shows(&ran.stderr, name), "the name was said back: {}", ran.stderr);
    }
    assert!(!w.at("jm").exists(), "a refusal made Jellymoon's folder");
    // And with a home and a project there already, nothing is changed either: the whole world is compared.
    printed(&w.new_project("beta"));
    for name in [full.as_str(), up.as_str(), "..", ".hidden", "-alpha", "C:alpha"] {
        new_refused(&w, name, NAME);
    }
    // Four letters that start as a device's do and are none are a name; and the longest name a project has is made.
    assert_eq!(printed(&w.new_project("COMA"))["name"], "COMA");
    assert_eq!(printed(&w.new_project(&"a".repeat(100)))["name"], "a".repeat(100));
}

#[test]
fn a_name_on_the_list_is_taken_whatever_folder_it_names_and_in_either_case_of_its_letters() {
    let w = heard_world();
    let elsewhere = w.repo("alpha");
    printed(&w.run(&["project", "add", "--repo", elsewhere.to_str().unwrap(), "--date", DAY]));
    for name in ["alpha", "Alpha", "ALPHA"] {
        new_refused(&w, name, LISTED);
    }
    assert!(!w.jm("projects").exists(), "a refusal made the folder projects are made in");
    // A made project is on the list too.
    printed(&w.new_project("Beta"));
    for name in ["Beta", "beta"] {
        new_refused(&w, name, LISTED);
    }
    assert_eq!(w.listed(), json!([{"name": "alpha", "used": DAY}, {"name": "Beta", "used": DAY}]));
}

#[test]
fn a_folder_or_a_file_already_there_by_the_name_is_taken_in_either_case_of_its_letters() {
    let w = heard_world();
    fs::create_dir_all(w.made("alpha")).unwrap();
    fs::write(w.made("alpha").join("kept.txt"), "the person's own\n").unwrap();
    fs::write(w.made("beta"), "a file\n").unwrap();
    for name in ["alpha", "Alpha", "beta", "BETA"] {
        new_refused(&w, name, THERE);
    }
    assert!(!w.list_file().exists() && !w.jm("runs").exists());
    // Two forms of one name: the file system says whether they are one. Where it does, the second is taken;
    // where it keeps them apart, they are two projects, each listed by the letters it was made with.
    let (one, other) = ("caf\u{e9}", "cafe\u{301}");
    printed(&w.new_project(one));
    let before = w.all();
    let ran = w.new_project(other);
    if ran.code == 0 {
        assert_eq!(printed(&ran)["name"], other);
        assert!(w.made(one).join(".git").is_dir() && w.made(other).join(".git").is_dir());
        assert_eq!(w.names_in("projects").len(), 4);
    } else {
        assert_eq!(refused(&ran), format!("jellymoon: {THERE}\n"));
        assert!(w.all() == before);
    }
}

#[test]
fn a_link_where_the_project_would_be_is_never_followed_and_never_replaced() {
    let w = heard_world();
    let outside = w.folder("outside");
    fs::create_dir_all(w.jm("projects")).unwrap();
    // A link to a folder, a link that leads nowhere, and a link to a file.
    fs::write(w.at("work").join("file.txt"), "a file\n").unwrap();
    let links =
        [("alpha", outside.clone()), ("beta", w.at("work").join("nowhere")), ("gamma", w.at("work").join("file.txt"))];
    for (name, to) in &links {
        if !link(to, &w.made(name)) {
            return skipped("this system makes no link for this user");
        }
    }
    for (name, to) in &links {
        new_refused(&w, name, THERE);
        assert_eq!(&fs::read_link(w.made(name)).unwrap(), to, "the link was replaced");
    }
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 0, "something was made where a link leads");
    assert!(!w.at("work").join("nowhere").exists());
}

#[test]
fn a_link_or_a_file_where_the_folder_of_projects_would_be_is_refused_and_nothing_is_made_through_it() {
    let w = heard_world();
    let outside = w.folder("outside");
    fs::create_dir_all(w.at("jm")).unwrap();
    fs::write(w.jm("projects"), "a file\n").unwrap();
    new_refused(&w, "alpha", NO_FOLDER);
    fs::remove_file(w.jm("projects")).unwrap();
    for to in [outside.clone(), w.at("work").join("nowhere")] {
        if !link(&to, &w.jm("projects")) {
            return skipped("this system makes no link for this user");
        }
        new_refused(&w, "alpha", NO_FOLDER);
        assert_eq!(fs::read_link(w.jm("projects")).unwrap(), to, "the link was replaced");
        unlink(&w.jm("projects"));
    }
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 0, "something was made where the link leads");
    assert!(!w.at("work").join("nowhere").exists() && !w.jm("runs").exists());
}

#[test]
fn with_no_git_on_the_path_it_says_so_and_makes_nothing() {
    let w = heard_world();
    // A `PATH` that holds the harness alone, one that is empty, and one that names git's folder by no full path.
    let paths: [OsString; 3] = [w.at("bin").into(), OsString::new(), "git".into()];
    for path in paths {
        let before = w.all();
        let ran = w.run_with(&[("PATH", path)], &["project", "new", "alpha", "--date", DAY]);
        assert_eq!(refused(&ran), format!("jellymoon: {NO_GIT}\n"));
        assert!(w.all() == before && !w.at("jm").exists(), "with no git something was made");
    }
    // And beside a project that is there, nothing changes.
    printed(&w.new_project("beta"));
    let before = w.all();
    let ran = w.run_with(&[("PATH", w.at("bin").into())], &["project", "new", "alpha", "--date", DAY]);
    assert_eq!(refused(&ran), format!("jellymoon: {NO_GIT}\n"));
    assert!(w.all() == before);
}

#[test]
fn a_list_that_is_full_or_is_not_jellymoon_s_own_writing_is_refused_before_anything_is_made() {
    let w = heard_world();
    let (root, sep) = (w.real(&w.at("work")), std::path::MAIN_SEPARATOR);
    let line = |n: usize| json!({"name": format!("p{n}"), "folder": format!("{root}{sep}p{n}"), "used": DAY});
    w.write_list(json!({ "projects": (0..100).map(line).collect::<Vec<_>>() }).to_string().as_bytes());
    let full = "the list has its 100 projects already, so this one was not added; let one go with `jellymoon \
                project forget`";
    new_refused(&w, "alpha", full);
    // With one let go there is room.
    printed(&w.run(&["project", "forget", "p7"]));
    assert_eq!(printed(&w.new_project("alpha"))["name"], "alpha");
    assert_eq!(w.listed().as_array().unwrap().len(), 100);

    w.write_list(b"not a list\n");
    let unused = "the list of projects (projects.json in Jellymoon's folder) is not used: it is not a list as \
                  Jellymoon writes one; move the file away and add your projects again";
    new_refused(&w, "beta", unused);
    assert_eq!(w.names_in("projects"), ["alpha"]);
}
