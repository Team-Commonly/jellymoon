//! F50's follow-up, what its gate left open: a `copies` that is a link, a line with no working directory, and a
//! line of a copy that repeats the uuid of one typed elsewhere.

use super::*;

/// A typed line with no working directory on it.
fn bare(minute: usize, words: &str) -> String {
    let mut line: Value = serde_json::from_str(&said(&at(minute), "", words)).unwrap();
    line.as_object_mut().unwrap().remove("cwd");
    line.to_string()
}

/// A typed line under one uuid, as a resumed or forked session repeats a line.
fn twice(minute: usize, folder: &str, words: &str) -> String {
    support::line("user", &at(minute), folder, "human", json!(words), json!({"uuid": "u-twice"}))
}

#[test]
fn a_copy_s_line_hides_no_line_of_its_uuid_typed_elsewhere_whichever_file_is_read_first() {
    let kept = [("typed in alpha".to_string(), "done in alpha".to_string())];
    for (first, second) in [("-a", "-b"), ("-b", "-a")] {
        let w = world();
        let copy = w.copy();
        let in_copy = [twice(0, text(&copy), &format!("{LEFT} typed")), reply(&at(1), text(&copy), LEFT)];
        w.log(&format!("{first}/s.jsonl"), &in_copy);
        w.log(&format!("{second}/s.jsonl"), &[twice(2, A, "typed in alpha"), reply(&at(3), A, "done in alpha")]);
        let ran = w.run("day", &[]);
        assert!(!ran.stdout.contains(LEFT), "{}", ran.stdout);
        assert_eq!(turns(&ran.stdout), kept, "the copy's file is {first}");
    }
    // A switched-off folder's line is remembered, as before: read first, it is the one kept, and it is not printed.
    let w = world();
    w.log("-a/s.jsonl", &[twice(0, B, "dear diary")]);
    w.log("-b/s.jsonl", &[twice(2, A, "typed in alpha"), reply(&at(3), A, "done in alpha")]);
    assert_eq!(w.day(&["--skip", "private-notes"]), [""; 0]);
}

#[test]
fn a_line_with_no_folder_is_judged_by_the_last_line_of_its_file_that_had_one() {
    let w = world();
    let copy = w.copy();
    let copy = text(&copy);
    // A run's session: the line between the copy's lines is left out with them.
    let run = [
        said(&at(0), copy, &format!("{LEFT} typed")),
        bare(1, &format!("{LEFT} typed with no folder")),
        reply(&at(2), A, &format!("{LEFT} said elsewhere to it")),
        said(&at(3), copy, &format!("{LEFT} typed again")),
    ];
    w.log("-jm-copies-run/s1.jsonl", &run);
    // At the top of a file no line was judged yet, whatever the file before it ended in and whatever follows. And
    // it is the last line that judges, not any line before: after a line elsewhere, one is read again.
    let top = [
        bare(20, "at the top of a file"),
        said(&at(21), copy, &format!("{LEFT} typed below")),
        said(&at(22), A, "back in alpha"),
        bare(23, "and after it"),
    ];
    w.log("-top/s2.jsonl", &top);
    // The person's own session: read as before.
    let own = [said(&at(30), A, "typed in alpha"), bare(31, "after a person's line"), reply(&at(32), A, "done")];
    w.log("-work-alpha/s3.jsonl", &own);
    let top = ["at the top of a file", "back in alpha", "and after it"];
    assert_eq!(w.day(&[]), [&top[..], &["typed in alpha", "after a person's line"]].concat());
    let ran = w.run("day", &[]);
    assert_eq!(turns(&ran.stdout)[4], ("after a person's line".to_string(), "done".to_string()));
    // Switching off still goes by the name of the log's own folder for such a line, and by nothing before it.
    assert_eq!(w.day(&["--skip", "work-alpha"]), [&top[..], &["typed in alpha"]].concat());
    assert_eq!(w.day(&["--skip", "top"]), ["back in alpha", "typed in alpha", "after a person's line"]);
}

// Unix only, as the link test beside it is: a link to a folder is made here with no right asked for, and on Windows
// making one needs a right a test cannot count on.
#[cfg(unix)]
#[test]
fn a_copy_logged_under_the_folder_a_linked_copies_leads_to_is_left_out() {
    let w = world();
    let far = w.dir.path().join("far");
    fs::create_dir_all(far.join("copies")).unwrap();
    fs::create_dir(&w.home).unwrap();
    std::os::unix::fs::symlink(far.join("copies"), w.home.join("copies")).unwrap();
    // A harness started in `<home>/copies/<run>` logs the folder it stands in as the system resolved it.
    let resolved = fs::canonicalize(&far).unwrap();
    let home = fs::canonicalize(&w.home).unwrap();
    let folder = |folder: PathBuf, left: bool| (text(&folder).to_string(), left);
    let mut folders = [
        (A.to_string(), false),
        folder(resolved.join("copies").join(RUN), true),
        folder(w.copy(), true),
        folder(home.join("copies").join(RUN), true),
        folder(resolved.join("copies"), false),
        folder(resolved.join("copies-old").join(RUN), false),
        folder(resolved.join(RUN), false),
    ];
    w.typed_in(&folders);
    assert_eq!(w.day(&[]), ["seen 0", "seen 4", "seen 5", "seen 6"]);
    // A link to nothing resolves to nothing, and that is no error: the home's two spellings are what is left.
    fs::remove_dir(far.join("copies")).unwrap();
    folders[1].1 = false;
    w.typed_in(&folders);
    assert_eq!(w.day(&[]), ["seen 0", "seen 1", "seen 4", "seen 5", "seen 6"]);
}
