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
    assert!(first.take_back(home.path(), &folder, &project).is_none());
    assert!(project.is_dir() && !project.join(&first.token).exists(), "a folder the list names was removed");
    fs::remove_file(home.path().join("projects.json")).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(made(&folder, &project, false).take_back(home.path(), &folder, &project), Some(true));
    assert!(!project.exists() && folder.is_dir());
    // A folder put in its place is another folder; and `projects` goes when this command made it.
    let was = made(&folder, &project, true);
    fs::remove_dir_all(&project).and_then(|()| fs::create_dir(&project)).unwrap_or_else(|e| panic!("{e}"));
    assert!(was.take_back(home.path(), &folder, &project).is_none());
    assert!(project.is_dir(), "a folder this command did not make was removed");
    fs::remove_dir(&project).unwrap_or_else(|e| panic!("{e}"));
    // `projects` holds no file of a making, so it goes only where the system's word tells it: a mark that is whole.
    let whole = mark(&folder).is_some_and(|(number, at)| number.is_some() && at.is_some());
    assert_eq!(made(&folder, &project, true).take_back(home.path(), &folder, &project), Some(true));
    assert_eq!(folder.exists(), !whole, "whole: {whole}");
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
    assert!(was.take_back(home.path(), &folder, &project).is_none() && project.join("mine.txt").is_file());
    // The file of another making, or a folder under the file's name, is not this making's file either.
    fs::write(project.join(Made::new().token), "").unwrap_or_else(|e| panic!("{e}"));
    assert!(!was.ours(&project));
    fs::create_dir(project.join(&was.token)).unwrap_or_else(|e| panic!("{e}"));
    assert!(!was.ours(&project));
    // And two makings never share a file's name.
    assert_ne!(Made::new().token, Made::new().token);
}

// A making that finds its file gone has only the system's word for the folder at its name, and that word is a
// mark with both the system's number and a time of making in it. A time alone (Windows) is what a folder put in
// place can share; a number alone (a file system that keeps no time of making) is handed out again. Both sides
// are values here, so each system proves the sides it does not run.
#[test]
fn a_folder_is_taken_for_the_made_one_by_its_mark_only_when_both_marks_hold_a_number_and_a_time_and_are_one() {
    let at = |seconds| Some(UNIX_EPOCH + std::time::Duration::from_secs(seconds));
    let whole = |number: u64, seconds| Some((Some((7, number)), at(seconds)));
    // Every pair of what a mark can hold: both, the number alone, the time alone, neither.
    let kinds = [whole(40, 1), Some((Some((7, 40)), None)), Some((None, at(1))), Some((None, None))];
    for (made, was) in kinds.iter().enumerate() {
        for (there, now) in kinds.iter().enumerate() {
            assert_eq!(made_still(*was, *now), made == 0 && there == 0, "made: kind {made}, there: kind {there}");
        }
    }
    assert!(!made_still(whole(40, 1), whole(41, 1)), "another number is another folder");
    assert!(!made_still(whole(40, 1), whole(40, 2)), "another time is another folder");
    // No folder made, or none there now.
    assert!(!made_still(None, None) && !made_still(None, whole(40, 1)) && !made_still(whole(40, 1), None));
}

// `projects` never holds a file of a making, so the system's word is all that tells it: it is taken back by the
// same question, and not by two marks that are merely one.
#[test]
fn the_folder_of_projects_is_this_command_s_to_take_back_only_by_a_mark_that_is_whole() {
    let at = Some(UNIX_EPOCH + std::time::Duration::from_secs(1));
    let whole = |number: u64| Some((Some((7, number)), at));
    let made = |projects| Made { projects, ..Made::new() };
    assert!(made(whole(40)).made_projects(whole(40)) && !made(whole(40)).made_projects(whole(41)));
    for part in [Some((Some((7, 40)), None)), Some((None, at)), Some((None, None)), None] {
        assert!(!made(part).made_projects(part), "projects was taken by a mark that is not whole: {part:?}");
    }
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
fn a_refusal_after_the_folder_was_taken_back_says_so_once_and_says_when_it_was_left() {
    let said = |e: Error, back: bool| after(e, back).to_string();
    assert_eq!(said(refused(LISTED), true), format!("{LISTED}{BACK}"));
    assert_eq!(said(refused(LISTED), false), format!("{LISTED}{LEFT}"));
    assert_eq!(said(refused(SPELLED), true), SPELLED);
    // A folder that was left is never said to have been taken back in the same sentence.
    let git = |end: &str| format!("git could not make the new project's repo{end}");
    assert_eq!(said(refused(git(TOOK)), true), git(", so what was made was taken back and nothing was listed"));
    assert_eq!(said(refused(git(TOOK)), false), format!("{}{LEFT}", git(", so nothing was listed")));
    let spelled = "this file system does not keep the folder under the name it was given, so nothing was listed; \
                   pick a name of plain letters";
    assert_eq!(said(refused(SPELLED), false), format!("{spelled}{LEFT}"));
    let store = said(Error::Store(io::Error::other("no room")), true);
    assert_eq!(store, format!("jellymoon's own files: no room{BACK}"));
}

#[cfg(unix)]
#[test]
fn a_link_to_a_file_at_the_making_s_name_is_not_the_making_s_file() {
    let home = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
    let (folder, project) = (home.path().join("projects"), home.path().join("projects").join("alpha"));
    let was = made(&folder, &project, false);
    let (file, neighbour) = (project.join(&was.token), home.path().join("neighbour"));
    fs::remove_file(&file).and_then(|()| fs::write(&neighbour, "")).unwrap_or_else(|e| panic!("{e}"));
    std::os::unix::fs::symlink(&neighbour, &file).unwrap_or_else(|e| panic!("{e}"));
    assert!(!was.ours(&project) && was.take_back(home.path(), &folder, &project).is_none());
    assert!(project.is_dir() && neighbour.is_file());
}

#[test]
fn a_made_folder_holds_its_making_s_file_and_one_whose_file_could_not_be_made_is_removed_again() {
    let home = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
    let project = home.path().join("alpha");
    // No file can be made under a name with a folder that is not there in it.
    let no_such = Path::new("no").join("such");
    assert!(matches!(begin(&project, &no_such.to_string_lossy()), Err(Error::Store(_))));
    assert!(!project.exists(), "a folder with no file of its making was left");
    assert!(begin(&project, ".jellymoon-making-1-2-3").is_ok());
    let meta = fs::metadata(project.join(".jellymoon-making-1-2-3")).unwrap_or_else(|e| panic!("{e}"));
    assert!(meta.is_file() && meta.len() == 0);
    #[cfg(unix)]
    assert_eq!(std::os::unix::fs::PermissionsExt::mode(&meta.permissions()) & 0o777, 0o600);
    // Something at the name is said so, and is left.
    let there = begin(&project, ".jellymoon-making-4-5-6").map_or_else(|e| e.to_string(), |()| String::new());
    assert!(there == THERE && project.join(".jellymoon-making-1-2-3").is_file());
}

/// The names in a folder, in order.
fn names(folder: &Path) -> Vec<String> {
    let names = fs::read_dir(folder).into_iter().flatten().flatten();
    let mut names: Vec<String> = names.map(|entry| entry.file_name().to_string_lossy().into()).collect();
    names.sort();
    names
}

#[test]
fn only_a_plain_empty_file_of_a_making_s_name_directly_in_a_folder_under_projects_is_tidied() {
    let home = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
    let project = home.path().join("projects").join("alpha");
    let (below, elsewhere) = (project.join("below"), home.path().join("work").join("beta"));
    let real = |folder: &Path| as_git_takes(fs::canonicalize(folder).unwrap_or_default()).to_string_lossy().to_string();
    for folder in [&below, &elsewhere] {
        fs::create_dir_all(folder).unwrap_or_else(|e| panic!("{e}"));
        fs::write(folder.join(".jellymoon-making-1-2-3"), "").unwrap_or_else(|e| panic!("{e}"));
        tidy(home.path(), &real(folder));
        assert_eq!(names(folder), [".jellymoon-making-1-2-3"], "a folder not directly under projects was tidied");
    }
    // In the folder itself: the file of a making goes; one that holds something, a folder under such a name,
    // every name of another shape, and the person's own file stay.
    let stay = ["-1-2", "-1-2-3-4", "-a-2-3", "-1--3", "-1-2-3.txt"].map(|end| format!(".jellymoon-making{end}"));
    for name in
        stay.iter().map(String::as_str).chain(["x.jellymoon-making-1-2-3", "mine.txt", ".jellymoon-making-1-2-3"])
    {
        fs::write(project.join(name), "").unwrap_or_else(|e| panic!("{e}"));
    }
    fs::write(project.join(".jellymoon-making-4-5-6"), "kept\n").unwrap_or_else(|e| panic!("{e}"));
    fs::create_dir(project.join(".jellymoon-making-7-8-9")).unwrap_or_else(|e| panic!("{e}"));
    #[cfg(unix)]
    std::os::unix::fs::symlink(project.join("mine.txt"), project.join(".jellymoon-making-10-11-12"))
        .unwrap_or_else(|e| panic!("{e}"));
    let before = names(&project);
    tidy(home.path(), &real(&project));
    let left: Vec<String> = before.into_iter().filter(|name| name != ".jellymoon-making-1-2-3").collect();
    assert_eq!(names(&project), left);
    assert_eq!(fs::read_to_string(project.join(".jellymoon-making-4-5-6")).unwrap_or_default(), "kept\n");
}

#[test]
fn a_line_for_this_very_name_and_folder_is_the_same_line_and_any_other_of_the_name_is_taken() {
    let line =
        |name: &str, folder: &str| vec![Line { name: name.into(), folder: folder.into(), used: "2026-03-10".into() }];
    assert!(onto(&mut line("alpha", "/p/alpha"), "alpha", "/p/alpha", "2026-03-11").is_ok());
    assert!(onto(&mut line("alpha", "/q/alpha"), "alpha", "/p/alpha", "2026-03-11").is_err());
    assert!(onto(&mut line("Alpha", "/p/alpha"), "alpha", "/p/alpha", "2026-03-11").is_err());
}
