//! The rules of a new project that no command line reaches, or reaches only on one system.

use super::*;

#[test]
fn a_name_is_makeable_when_no_system_reads_it_as_anything_but_one_folder() {
    let fine = [
        "alpha",
        "my-app",
        "a.b",
        "v1.2",
        "caf\u{e9}",
        "COM",
        "COM10",
        "console",
        "nul0",
        "a b",
        "COMA",
        "LPTX",
        "coms.txt",
        "a~b",
        "CONIN",
        "a$",
    ];
    for name in fine {
        assert!(makeable(name), "{name}");
    }
}

#[test]
fn a_name_some_system_reads_as_more_than_one_folder_s_name_is_not_makeable() {
    let odd = [
        "",
        ".",
        "..",
        "a/b",
        "a\\b",
        ".a",
        "-a",
        " a",
        "a.",
        "a ",
        "a:b",
        "C:",
        "con",
        "NUL",
        "nul.txt",
        "PRN",
        "aux.tar.gz",
        "com1",
        "COM9.log",
        "lpt0",
        "LPT5",
        "a*b",
        "a?b",
        "a|b",
        "a<b",
        "a>b",
        "a\"b",
        "COM\u{b9}",
        "lpt\u{b2}.txt",
        "com\u{b3}",
        "CONIN$",
        "conout$",
        "ConOut$.log",
        "con .txt",
        "~",
        "~a",
        "a\u{200d}b",
        "a\u{200c}b",
        "a  b",
        "a\u{34f}b",
        "a\u{115f}b",
        "a\u{1160}b",
        "a\u{3164}b",
        "a\u{ffa0}b",
        "a\u{2800}b",
        "a\u{fe0f}b",
        "a\u{fe00}b",
    ];
    for name in odd {
        assert!(!makeable(name), "{name}");
    }
    assert!(same("Alpha", "aLPHA") && !same("alpha", "alpha1"));
}

#[test]
fn a_home_has_room_for_a_project_when_its_path_and_the_name_are_within_what_the_list_holds() {
    let most = FOLDER_MAX - "/projects/alpha".len();
    assert!(fits(most, "alpha") && !fits(most + 1, "alpha") && !fits(most, "alphas"));
}

/// A folder at `project` as a making leaves one: there, with the making's own file in it.
fn made(folder: &Path, project: &Path, both: bool) -> Made {
    let mut made = Made::new();
    fs::create_dir_all(project).unwrap_or_else(|e| panic!("{e}"));
    fs::write(project.join(&made.token), "").unwrap_or_else(|e| panic!("{e}"));
    (made.projects, made.project) = (mark(folder).filter(|_| both), mark(project));
    made
}

#[test]
fn what_is_taken_back_is_the_folder_that_was_made_and_never_one_the_list_names() {
    let home = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
    let (folder, project) = (home.path().join("projects"), home.path().join("projects").join("alpha"));
    // Listed, it stays, without the making's file; unlisted, it goes, and `projects` stays when this command
    // did not make it.
    let first = made(&folder, &project, true);
    let real = as_git_takes(fs::canonicalize(&project).unwrap_or_default());
    let line = Line { name: "alpha".into(), folder: real.to_string_lossy().into(), used: "2026-03-10".into() };
    hold(home.path()).and_then(|_lock| write(home.path(), vec![line])).unwrap_or_else(|e| panic!("{e}"));
    assert!(!first.take_back(home.path(), &folder, &project));
    assert!(project.is_dir() && !project.join(&first.token).exists(), "a folder the list names was removed");
    fs::remove_file(home.path().join("projects.json")).unwrap_or_else(|e| panic!("{e}"));
    assert!(made(&folder, &project, false).take_back(home.path(), &folder, &project));
    assert!(!project.exists() && folder.is_dir());
    // A folder put in its place is another folder; and `projects` goes when this command made it.
    let was = made(&folder, &project, true);
    fs::remove_dir_all(&project).and_then(|()| fs::create_dir(&project)).unwrap_or_else(|e| panic!("{e}"));
    assert!(!was.take_back(home.path(), &folder, &project));
    assert!(project.is_dir(), "a folder this command did not make was removed");
    fs::remove_dir(&project).unwrap_or_else(|e| panic!("{e}"));
    assert!(made(&folder, &project, true).take_back(home.path(), &folder, &project));
    assert!(!folder.exists());
}

// Windows hands a folder made again under a name its old time of making, and has no number to tell two folders
// by: there the folder put in place looks, from outside, like the one that was made. So it is here, on every
// system: the mark is read from the folder that was put in place.
#[test]
fn a_folder_that_looks_like_the_made_one_from_outside_and_holds_no_file_of_the_making_is_not_it() {
    let home = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
    let (folder, project) = (home.path().join("projects"), home.path().join("projects").join("alpha"));
    let mut was = made(&folder, &project, false);
    fs::remove_dir_all(&project).and_then(|()| fs::create_dir(&project)).unwrap_or_else(|e| panic!("{e}"));
    fs::write(project.join("mine.txt"), "the person's own\n").unwrap_or_else(|e| panic!("{e}"));
    was.project = mark(&project);
    assert!(was.project.is_some() && !was.ours(&project));
    assert!(!was.take_back(home.path(), &folder, &project) && project.join("mine.txt").is_file());
    // The file of another making, or a folder under the file's name, is not this making's file either.
    fs::write(project.join(Made::new().token), "").unwrap_or_else(|e| panic!("{e}"));
    assert!(!was.ours(&project));
    fs::create_dir(project.join(&was.token)).unwrap_or_else(|e| panic!("{e}"));
    assert!(!was.ours(&project));
    // And two makings never share a file's name.
    assert_ne!(Made::new().token, Made::new().token);
}

#[test]
fn what_is_said_of_the_made_folder_is_passed_on_and_only_another_name_is_the_file_system_s() {
    let asked = |name: &str| Ok((name.to_string(), "/p/alpha".to_string()));
    assert_eq!(kept_as(asked("alpha"), "alpha").unwrap_or_default(), "/p/alpha");
    let said = |asked| kept_as(asked, "alpha").unwrap_or_else(|e| e.to_string());
    assert_eq!(said(asked("Alpha")), SPELLED);
    assert_eq!(said(Err(refused("--repo is not a folder of your own"))), "--repo is not a folder of your own");
}

#[test]
fn a_refusal_after_the_folder_was_taken_back_says_so_once() {
    let said = |e: Error| after(e).to_string();
    assert_eq!(said(refused(LISTED)), format!("{LISTED}{BACK}"));
    assert_eq!(said(refused(SPELLED)), SPELLED);
    let store = said(Error::Store(io::Error::other("no room")));
    assert_eq!(store, format!("jellymoon's own files: no room{BACK}"));
}

#[test]
fn a_line_for_this_very_name_and_folder_is_the_same_line_and_any_other_of_the_name_is_taken() {
    let line =
        |name: &str, folder: &str| vec![Line { name: name.into(), folder: folder.into(), used: "2026-03-10".into() }];
    assert!(onto(&mut line("alpha", "/p/alpha"), "alpha", "/p/alpha", "2026-03-11").is_ok());
    assert!(onto(&mut line("alpha", "/q/alpha"), "alpha", "/p/alpha", "2026-03-11").is_err());
    assert!(onto(&mut line("Alpha", "/p/alpha"), "alpha", "/p/alpha", "2026-03-11").is_err());
}
