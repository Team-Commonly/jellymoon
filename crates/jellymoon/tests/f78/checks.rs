//! A harness in a folder the login shell named is held to more than one in a fixed place (F78, the row's third
//! step): it is passed over when the group or everyone can write to it, when everyone can write to its folder,
//! or when everyone can write to the folder its link-resolved file is in. The look goes on to the next folder,
//! and when no folder has one the line says what was passed over, and names no folder.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crate::shell::{INSTALL, NONE, finder, found};

fn mode(path: &Path, mode: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

#[test]
fn a_harness_others_can_write_to_or_in_a_folder_everyone_can_write_to_is_passed_over_for_the_next() {
    let mut w = finder();
    let shell = w.shell("zsh");
    let at = |name: &str| w.at(name);
    for folder in ["group", "everyone", "open", "real", "good"] {
        w.reader_in(&at(folder));
    }
    mode(&at("group").join("claude"), 0o775);
    mode(&at("everyone").join("claude"), 0o757);
    // A link in a folder nobody else can write to, to a program in a folder everyone can.
    fs::rename(at("real").join("claude"), at("real").join("the-program")).unwrap();
    w.made("link");
    std::os::unix::fs::symlink(at("real").join("the-program"), at("link").join("claude")).unwrap();
    for folder in ["open", "real"] {
        mode(&at(folder), 0o777);
    }
    // A folder the group can write to is taken, as Homebrew's are.
    mode(&at("good"), 0o775);
    let all = ["group", "everyone", "open", "link", "good"].map(at);
    shell.names(&all.iter().map(|folder| folder.as_path()).collect::<Vec<_>>());

    let ran = w.page();
    assert_eq!((ran.code, ran.stderr.as_str()), (0, ""), "jellymoon page");
    let started: Vec<&str> = ["group", "everyone", "open", "link", "real", "good"]
        .into_iter()
        .filter(|at| w.at(at).join("arg0").exists())
        .collect();
    assert_eq!(started, ["good"]);

    // With no harness left that passes, each is said once with why, and no folder is named.
    fs::remove_file(at("good").join("claude")).unwrap();
    let passed = "a `claude` in a folder of the login shell's PATH";
    let over = format!(
        " (passed over {passed}: others can write to it, {passed}: everyone can write to its folder, {passed}: \
         everyone can write to the folder its file is in)"
    );
    // The page that was made is kept in Jellymoon's folder, so this look is the page's own and not `no_page`.
    let ran = w.page();
    assert_eq!((ran.code, ran.stdout.as_str()), (1, ""));
    assert_eq!(ran.stderr, format!("{NONE}, the login shell's PATH{over}{INSTALL}"));
    let says = "Claude Code was not found on PATH, in a fixed place or in a folder of the login shell's PATH; a \
                `claude` program was passed over, as others can write to it or to a folder it is in";
    assert_eq!(w.harness(), found(false, "none", says));
    for folder in ["open", "real"] {
        mode(&at(folder), 0o755);
    }
}

// A login shell that answered and named no folder with a harness: the look ends as before, with its PATH among
// the places looked in.
#[test]
fn a_login_shell_whose_folders_hold_no_harness_is_named_among_the_places_looked_in() {
    let mut w = finder();
    let shell = w.shell("bash");
    shell.names(&[&w.made("empty")]);
    assert_eq!(w.no_page(), format!("{NONE}, the login shell's PATH{INSTALL}"));
    let says = "Claude Code was not found on PATH, in a fixed place or in a folder of the login shell's PATH";
    assert_eq!(w.harness(), found(false, "none", says));
    assert_eq!(shell.starts(), 2);
}
