use tempfile::TempDir;

use super::*;

const BEFORE: &str = "{\n  \"model\": \"opus\"\n}\n";

/// A folder with a settings file in `at`, a home for Jellymoon, and a made-up program path.
fn world(at: &str) -> (TempDir, PathBuf, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let settings = dir.path().join(at).join("settings.json");
    fs::create_dir_all(settings.parent().unwrap()).unwrap();
    fs::write(&settings, BEFORE).unwrap();
    let (home, program) = (dir.path().join("jm"), dir.path().join("bin").join("jellymoon"));
    (dir, settings, home, program)
}

#[test]
fn a_write_that_fails_leaves_the_settings_whole_and_can_be_tried_again() {
    let (_dir, settings, home, program) = world("harness");
    // The name the new file would get is taken, so the write fails before the settings are touched.
    let taken = settings.with_file_name(format!("settings.json.jellymoon-{}", process::id()));
    fs::write(&taken, "someone else's").unwrap();
    assert!(matches!(install_hook(&settings, &home, &program), Err(Error::Settings(_))));
    assert_eq!(fs::read_to_string(&settings).unwrap(), BEFORE);
    assert_eq!(fs::read_to_string(&taken).unwrap(), "someone else's", "a file that was there is not removed");
    assert!(!home.join("hook.json").exists(), "nothing is recorded as installed");

    fs::remove_file(&taken).unwrap();
    assert!(install_hook(&settings, &home, &program).unwrap().changed);
    let installed = fs::read_to_string(&settings).unwrap();
    fs::write(&taken, "someone else's").unwrap();
    assert!(matches!(remove_hook(&home), Err(Error::Settings(_))));
    assert_eq!(fs::read_to_string(&settings).unwrap(), installed);
    fs::remove_file(&taken).unwrap();
    assert!(remove_hook(&home).unwrap().changed, "the record was kept, so the removal can be tried again");
    assert_eq!(fs::read_to_string(&settings).unwrap(), BEFORE);
    assert_eq!(fs::read_dir(settings.parent().unwrap()).unwrap().count(), 1, "no new file is left behind");
}

#[cfg(unix)]
#[test]
fn a_link_stays_a_link_and_the_files_permissions_are_kept() {
    use std::os::unix::fs::{PermissionsExt, symlink};

    for mode in [0o600, 0o444, 0o644] {
        let (dir, real, home, program) = world("dotfiles");
        fs::set_permissions(&real, fs::Permissions::from_mode(mode)).unwrap();
        let link = dir.path().join("settings.json");
        symlink(&real, &link).unwrap();
        let state = || {
            let linked = fs::symlink_metadata(&link).unwrap().file_type().is_symlink();
            (linked, fs::metadata(&real).unwrap().permissions().mode() & 0o777, fs::read_to_string(&real).unwrap())
        };

        assert!(install_hook(&link, &home, &program).unwrap().changed);
        let (linked, kept, text) = state();
        assert!(linked && kept == mode && text.contains(r#""args":["hook"]"#), "{mode:o}: {linked} {kept:o} {text}");
        assert!(remove_hook(&home).unwrap().changed);
        assert_eq!(state(), (true, mode, BEFORE.to_string()), "{mode:o}");
        assert_eq!(fs::read_dir(real.parent().unwrap()).unwrap().count(), 1, "no new file is left behind");
    }
}

// The new file is made new. A link waiting at its name is never written through to where it leads.
#[cfg(unix)]
#[test]
fn a_link_at_the_name_of_the_new_file_is_not_written_through() {
    let dir = tempfile::tempdir().unwrap();
    let (target, elsewhere) = (dir.path().join("kept.json"), dir.path().join("elsewhere"));
    fs::write(&elsewhere, "not ours").unwrap();
    let new = dir.path().join(format!("kept.json.jellymoon-{}", process::id()));
    std::os::unix::fs::symlink(&elsewhere, &new).unwrap();
    assert_eq!(put(&target, "ours", false).map_err(|e| e.kind()), Err(ErrorKind::AlreadyExists));
    assert_eq!(fs::read_to_string(&elsewhere).unwrap(), "not ours");
    assert!(!target.exists() && fs::read_link(&new).is_ok(), "nothing is at the place, and the link is left alone");
}
