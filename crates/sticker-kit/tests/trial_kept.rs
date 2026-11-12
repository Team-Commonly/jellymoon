//! What is kept of a trial, for whom, and where. Only an answer about the picture is kept: an acceptance, a rule's
//! refusal, a cost's. What may go otherwise next time is tried again. An answer is for the caller that asked and
//! the build that drew, and the folder it is kept in is the person's alone.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::fs;
use std::path::Path;
use std::time::Duration;

use support::{Folder, HONEST, trial_as};

/// How often the counting child was started.
fn starts(count: &Path) -> usize {
    fs::read(count).map_or(0, |bytes| bytes.len())
}

#[test]
fn only_an_answer_about_the_picture_is_kept() {
    let folder = Folder::new("about");
    let kept = Some(folder.0.join("kept"));
    let said = |which: &str| {
        let mut trial = trial_as(&[which]);
        (trial.kept, trial.limit) = (kept.clone(), Duration::from_millis(1500));
        folder.star(HONEST, &trial).expect_err(which)
    };
    // Each of these is about the child or the machine, and may go otherwise at the next start.
    assert!(said("fails").starts_with("its trial ended badly ("), "a good answer and a code");
    assert!(said("overflows").starts_with("its trial ended badly ("));
    assert_eq!(said("garbage"), "its trial did not answer as a trial does");
    assert_eq!(said("twice"), "its trial did not answer as a trial does");
    assert_eq!(said("unsure"), "the made-up child could not try it");
    assert!(said("deaf").starts_with("its trial took longer than 1500 ms"));
    assert_eq!(folder.kept("kept"), 0, "none of them is kept");

    // What a rule says of the picture is kept, as an acceptance is.
    let count = folder.0.join("count");
    let mut counted = trial_as(&["counted", count.to_str().unwrap()]);
    counted.kept = kept.clone();
    let scripted = HONEST.replace("<circle", "<script/><circle");
    for _ in 0..2 {
        assert_eq!(folder.star(&scripted, &counted), Err("it contains the element `<script`".to_string()));
        assert_eq!(folder.star(HONEST, &counted), Ok(HONEST.to_string()));
    }
    assert_eq!((starts(&count), folder.kept("kept")), (2, 2));
}

#[test]
fn a_kept_answer_is_for_the_caller_that_asked() {
    let folder = Folder::new("stamp");
    let count = folder.0.join("count");
    let mut counted = trial_as(&["counted", count.to_str().unwrap()]);
    counted.kept = Some(folder.0.join("kept"));
    let mut tried = |stamp: &str| {
        counted.stamp = stamp.to_string();
        assert_eq!(folder.star(HONEST, &counted), Ok(HONEST.to_string()), "{stamp}");
        starts(&count)
    };
    assert_eq!((tried("an app 1.0"), tried("an app 1.0")), (1, 1), "the same caller asks once");
    assert_eq!(tried("an app 1.1"), 2, "another version of the caller has another renderer: it is tried again");
    assert_eq!((tried("an app 1.0"), tried("an app 1.1")), (2, 2), "and each has its answer");
    // A stamp is one line of the key, whatever the caller gives.
    assert_eq!(tried("an app\n1.2\u{7}"), 3);
    let keys: Vec<String> = fs::read_dir(folder.0.join("kept"))
        .unwrap()
        .flatten()
        .map(|kept| fs::read_to_string(kept.path()).unwrap().lines().next().unwrap().to_string())
        .collect();
    let build = if cfg!(debug_assertions) { "debug" } else { "release" };
    for stamp in ["an app 1.0", "an app 1.1", "an app1.2"] {
        let key =
            format!("sticker-kit 0.1.0, usvg 0.46.0, read and drawn at twice its size, rules 2, {build}, for {stamp}");
        assert!(keys.contains(&key), "{key}: {keys:?}");
    }
}

#[cfg(unix)]
#[test]
fn the_kept_folder_is_the_person_s_alone_and_a_link_in_its_place_is_not_followed() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let mode = |path: &Path| fs::symlink_metadata(path).unwrap().permissions().mode() & 0o777;
    let folder = Folder::new("alone-kept");
    let count = folder.0.join("count");
    let mut counted = trial_as(&["counted", count.to_str().unwrap()]);
    let kept = folder.0.join("made").join("kept");
    counted.kept = Some(kept.clone());
    assert_eq!(folder.star(HONEST, &counted), Ok(HONEST.to_string()));
    let file = fs::read_dir(&kept).unwrap().next().unwrap().unwrap().path();
    assert_eq!((mode(&kept), mode(&file)), (0o700, 0o600), "the folder and the answer are for the person alone");

    // A folder that was there, open to others, is closed to them before anything is kept in it.
    let open = folder.0.join("open");
    fs::create_dir(&open).unwrap();
    fs::set_permissions(&open, fs::Permissions::from_mode(0o755)).unwrap();
    counted.kept = Some(open.clone());
    assert_eq!(folder.star(HONEST, &counted), Ok(HONEST.to_string()));
    assert_eq!((mode(&open), fs::read_dir(&open).unwrap().count()), (0o700, 1));

    // A link where the folder would be: nothing is kept behind it or read from it, and every picture is tried.
    let (behind, link) = (folder.0.join("behind"), folder.0.join("link"));
    fs::create_dir(&behind).unwrap();
    fs::copy(&file, behind.join(file.file_name().unwrap())).unwrap();
    symlink(&behind, &link).unwrap();
    counted.kept = Some(link);
    let before = starts(&count);
    for tried in 1..=2 {
        assert_eq!(folder.star(HONEST, &counted), Ok(HONEST.to_string()));
        assert_eq!(starts(&count), before + tried, "the answer behind the link is not read");
    }
    assert_eq!(fs::read_dir(&behind).unwrap().count(), 1, "and nothing is written behind it");

    // A kept answer that is a link is not read either.
    let name = file.file_name().unwrap();
    fs::rename(&file, folder.0.join("moved")).unwrap();
    symlink(folder.0.join("moved"), kept.join(name)).unwrap();
    counted.kept = Some(kept.clone());
    let before = starts(&count);
    assert_eq!(folder.star(HONEST, &counted), Ok(HONEST.to_string()));
    assert_eq!(starts(&count), before + 1, "a link in an answer's place is no answer");
    assert!(fs::symlink_metadata(kept.join(name)).unwrap().is_file(), "the answer kept afresh takes its place");
}
