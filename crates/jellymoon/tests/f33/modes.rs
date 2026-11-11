//! F33, the modes of the folders `install.sh` makes. The app turns delivery on only for a command that no one else
//! could replace (`jellymoon_core::installed_command`), so a folder the installer makes is the person's alone to
//! write whatever the umask, and one that was there is left as it was and said when it is open.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Output;

use super::mac::{Mac, release, repo, text};
use super::support::open_above;

fn mode(path: &Path) -> u32 {
    fs::metadata(path).unwrap().permissions().mode() & 0o7777
}

fn chmod(path: &Path, mode: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

/// A made-up machine whose home, and the test's folder above it, are their owner's alone to write whatever the
/// umask the tests run under.
fn mac() -> Mac {
    let mac = Mac::new();
    chmod(mac.root.path(), 0o755);
    chmod(&mac.home(), 0o755);
    mac
}

/// Runs the installer from the shell's input, as `curl ... | sh` does, with the umask set first in that same shell.
fn install_under(mac: &Mac, umask: &str, release: &Path) -> Output {
    let piped = mac.root.path().join("piped.sh");
    let script = fs::read_to_string(repo().join("install.sh")).unwrap();
    fs::write(&piped, format!("umask {umask}\n{script}")).unwrap();
    let mut sh = mac.command(Path::new("/bin/sh"), "/usr/bin:/bin:/usr/sbin");
    sh.env("JELLYMOON_RELEASE_URL", format!("file://{}", release.display()));
    sh.stdin(fs::File::open(piped).unwrap()).output().unwrap()
}

/// `jellymoon hook install --installed` by the installed command, in the made-up home.
fn turn_on(mac: &Mac) -> Output {
    fs::create_dir_all(mac.home().join(".claude")).unwrap();
    mac.command(&mac.binary(), "").args(["hook", "install", "--installed"]).output().unwrap()
}

const NOT_OFFERED: &str = "the app will not offer to turn delivery on";

#[test]
fn under_a_umask_of_002_the_folders_it_makes_are_the_person_s_alone_to_write() {
    let mac = mac();
    let ran = install_under(&mac, "002", &release(mac.root.path(), "release", "1.0.0"));
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    for made in ["Applications", ".local", ".local/bin", ".local/bin/jellymoon"] {
        assert_eq!(mode(&mac.home().join(made)), 0o755, "{made}");
    }
    assert!(!text(&ran.stdout).contains(NOT_OFFERED), "{}", text(&ran.stdout));
    // So core takes the command the installer placed, and delivery is turned on for it.
    if open_above(mac.root.path()).is_none() {
        let on = turn_on(&mac);
        let taken = "{\"installed\":true,\"changed\":true}\n";
        assert_eq!((text(&on.stdout).as_str(), text(&on.stderr).as_str()), (taken, ""));
    }
}

#[test]
fn a_folder_that_was_there_keeps_its_mode_and_an_open_one_is_said_in_the_closing_lines() {
    let mac = mac();
    let (local, bin) = (mac.home().join(".local"), mac.home().join(".local/bin"));
    fs::create_dir_all(&bin).unwrap();
    chmod(&local, 0o700);
    chmod(&bin, 0o775);
    let release = release(mac.root.path(), "release", "1.0.0");
    let ran = install_under(&mac, "002", &release);
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!((mode(&local), mode(&bin)), (0o700, 0o775), "a folder that was there is not changed");
    let said = text(&ran.stdout);
    let lines: Vec<&str> = said.lines().filter(|line| line.contains(NOT_OFFERED)).collect();
    assert_eq!(lines.len(), 1, "{said}");
    let (line, named) = (lines[0], bin.display().to_string());
    assert!(line.contains(&named) && line.contains("can be written to by others"), "{line}");
    assert!(line.contains("jellymoon hook install  from a terminal"), "{line}");
    let on = turn_on(&mac);
    let refused =
        "jellymoon: the installed jellymoon command is not used: others can write to a folder on the way to it\n";
    assert_eq!((on.status.code(), text(&on.stderr).as_str()), (Some(1), refused), "and core does refuse it");

    // A sticky folder, where only a file's owner can replace it, and a folder of the person's alone are not said.
    for sound in [0o1777, 0o755] {
        chmod(&bin, sound);
        let again = install_under(&mac, "002", &release);
        assert!(again.status.success() && !text(&again.stdout).contains(NOT_OFFERED), "{sound:o}");
        assert_eq!(mode(&bin), sound, "{sound:o}");
    }
}
