//! The installer opens the app it installed (install opens). No test ever starts the real `open`: every world that
//! runs the installer either sets `JELLYMOON_NO_OPEN` (what `Mac::command` does) or has a made-up `open` first on its
//! `PATH` (`Mac::install_opening`), and the door refuses a world that has neither (`door::unguarded_installer`).

use std::env;
use std::fs;
use std::panic::catch_unwind;
use std::path::Path;

use super::support::door::{self, UNGUARDED};
use super::support::scratch;

fn guarded(vars: &[(&str, &str)], args: &[&str]) -> bool {
    !door::unguarded_installer(&door::roots(), vars, args)
}

#[test]
fn the_door_refuses_a_world_that_runs_the_installer_with_no_way_to_keep_the_real_open_away() {
    let dir = scratch();
    let (made_up, bare) = (dir.path().join("made-up"), dir.path().join("bare"));
    fs::create_dir_all(&made_up).unwrap();
    fs::create_dir_all(&bare).unwrap();
    fs::write(made_up.join("open"), "#!/bin/sh\n").unwrap();
    // Lists of folders are written as this system writes `PATH`.
    let list = |folders: [&Path; 2]| env::join_paths(folders).unwrap().into_string().unwrap();
    let (after, first) = (list([&bare, &made_up]), list([&made_up, Path::new("/usr/bin")]));
    let bare = bare.display().to_string();

    // A world that does not run the installer is not asked.
    assert!(guarded(&[("PATH", "/usr/bin")], &["--help"]));
    // One that does, by its release address or by its script, needs one of the two.
    assert!(!guarded(&[("JELLYMOON_RELEASE_URL", "file:///nowhere"), ("PATH", "/usr/bin")], &["--help"]));
    assert!(!guarded(&[("PATH", "/usr/bin")], &["/anywhere/install.sh"]));
    let url = ("JELLYMOON_RELEASE_URL", "file:///nowhere");
    assert!(!guarded(&[url, ("JELLYMOON_NO_OPEN", ""), ("PATH", "/usr/bin")], &[]), "an empty value is no guard");
    assert!(!guarded(&[url, ("PATH", &bare)], &[]), "no `open` in the first folder");
    assert!(!guarded(&[url, ("PATH", &after)], &[]), "only the first folder of PATH is the made-up one");
    assert!(!guarded(&[url, ("PATH", "/usr/bin:/bin")], &[]), "the system's folder is no made-up one");
    assert!(guarded(&[url, ("JELLYMOON_NO_OPEN", "1"), ("PATH", "/usr/bin")], &[]));
    assert!(guarded(&[url, ("PATH", &first)], &[]));

    // At the door itself it is a panic with one fixed sentence, and nothing is started.
    let home = dir.path().to_str().unwrap();
    let vars = [("HOME", home), ("PATH", "/usr/bin"), ("JELLYMOON_RELEASE_URL", "file:///nowhere")];
    let panic = catch_unwind(|| drop(door::command(Path::new("/bin/sh"), vars, &["install.sh"]))).unwrap_err();
    assert!(panic.downcast_ref::<String>().is_some_and(|said| said == UNGUARDED));
}

#[cfg(target_os = "macos")]
mod mac {
    use std::fs;
    use std::path::Path;
    use std::process::{Command, Output};

    use crate::mac::{Mac, chips, ok, release, repo, run, text};

    impl Mac {
        /// Installs with a made-up `open` first on `PATH` that runs `body`; `off` also sets `JELLYMOON_NO_OPEN`.
        fn install_opening(&self, release: &Path, body: &str, off: bool) -> Output {
            let shims = self.root.path().join("shims");
            fs::create_dir_all(&shims).unwrap();
            fs::write(shims.join("open"), format!("#!/bin/sh\n{body}\n")).unwrap();
            ok(Command::new("chmod").arg("755").arg(shims.join("open")));
            let script = repo().join("install.sh");
            let path = format!("{}:/usr/bin:/bin:/usr/sbin", shims.display());
            run(&mut self.world(Path::new("/bin/sh"), &path, &[script.to_str().unwrap()], Some(release), off))
        }
        /// What the made-up `open` writes for each call: its argument, and whether the bundle it was given is there.
        fn records(&self) -> String {
            let log = self.root.path().join("open.log");
            format!("echo \"$1 $([ -d \"$1/Contents\" ] && echo there || echo missing)\" >> {}", log.display())
        }
        fn open_log(&self) -> Vec<String> {
            let log = fs::read_to_string(self.root.path().join("open.log")).unwrap_or_default();
            log.lines().map(str::to_string).collect()
        }
        fn app_line(&self) -> String {
            format!("{} there", self.app().display())
        }
    }

    const OPENING: &str =
        "Opening Jellymoon now: it shows today's page, read with one call through your own Claude Code.";
    const BY_HAND: &str = "Could not open it from here; the install itself is done.";

    fn by_hand_line(mac: &Mac) -> String {
        let app = mac.app();
        let line = format!(
            "  open \"{}\"    shows today's page, read with one call through your own Claude Code",
            app.display()
        );
        line
    }

    #[test]
    fn the_app_is_opened_once_by_its_installed_path_after_the_checksum_passed() {
        let mac = Mac::new();
        let ran = mac.install_opening(&release(mac.root.path(), "one", "1.0.0"), &mac.records(), false);
        assert!(ran.status.success(), "{}", text(&ran.stderr));
        assert_eq!(mac.open_log(), [mac.app_line()], "called once, with the bundle already in place");

        let said = text(&ran.stdout);
        let lines: Vec<&str> = said.lines().collect();
        let at = |needle: &str| lines.iter().position(|line| line.contains(needle)).unwrap();
        assert!(at("Jellymoon is installed.") < at(OPENING) && at(OPENING) < at("Next:"), "{said}");
        assert!(!said.contains("stays the version you had"), "{said}");
        // What it said to run by hand is not said when it was opened; the rest of the closing lines are.
        assert!(!said.contains(&by_hand_line(&mac)) && !said.contains(BY_HAND), "{said}");
        for kept in [
            "the app      ",
            "the command  ",
            "is not on your PATH yet",
            "the same from this terminal",
            "jellymoon uninstall",
        ] {
            assert!(said.contains(kept), "{kept}: {said}");
        }
    }

    #[test]
    fn an_update_opens_it_again_and_says_an_open_app_stays_the_version_it_was() {
        let mac = Mac::new();
        assert!(mac.install_opening(&release(mac.root.path(), "one", "1.0.0"), &mac.records(), false).status.success());
        let ran = mac.install_opening(&release(mac.root.path(), "two", "2.0.0"), &mac.records(), false);
        assert!(ran.status.success(), "{}", text(&ran.stderr));
        assert_eq!(mac.open_log(), [mac.app_line(), mac.app_line()]);
        let said = text(&ran.stdout);
        assert!(said.contains("Jellymoon is updated."), "{said}");
        assert!(said.contains(OPENING), "{said}");
        assert!(
            said.contains("If it was already open, it stays the version you had until you quit it and open it again.")
        );
    }

    #[test]
    fn it_is_never_opened_when_the_checksum_fails_when_told_not_to_or_when_the_install_is_refused() {
        let mac = Mac::new();
        let release = release(mac.root.path(), "one", "1.0.0");
        let archive = release.join(format!("jellymoon-macos-{}.tar.gz", chips().0));
        let good = fs::read(&archive).unwrap();
        let mut bytes = good.clone();
        bytes.push(0);
        fs::write(&archive, bytes).unwrap();
        let tampered = mac.install_opening(&release, &mac.records(), false);
        assert_eq!(tampered.status.code(), Some(1));
        assert_eq!(mac.open_log(), Vec::<String>::new(), "a download that does not match is never opened");

        // A command that is a folder is refused before any download.
        fs::create_dir_all(mac.binary().join("kept")).unwrap();
        let refused = mac.install_opening(&release, &mac.records(), false);
        assert_eq!(refused.status.code(), Some(1), "{}", text(&refused.stderr));
        assert_eq!(mac.open_log(), Vec::<String>::new(), "a refused install is never opened");
        fs::remove_dir_all(mac.binary()).unwrap();

        // Told not to, an install that worked opens nothing and says the command to run, as it did before.
        fs::write(&archive, good).unwrap();
        let quiet = mac.install_opening(&release, &mac.records(), true);
        assert!(quiet.status.success(), "{}", text(&quiet.stderr));
        assert_eq!(mac.open_log(), Vec::<String>::new());
        let said = text(&quiet.stdout);
        assert!(said.contains(&by_hand_line(&mac)) && !said.contains(OPENING), "{said}");
        assert!(mac.app().exists());
    }

    #[test]
    fn an_open_that_fails_leaves_the_install_done_and_says_the_command_to_run() {
        let mac = Mac::new();
        let body = format!("{}\nexit 1", mac.records());
        let ran = mac.install_opening(&release(mac.root.path(), "one", "1.0.0"), &body, false);
        assert_eq!(ran.status.code(), Some(0), "{}", text(&ran.stderr));
        assert_eq!(mac.open_log(), [mac.app_line()], "it was tried once");
        let said = text(&ran.stdout);
        assert!(said.contains("Jellymoon is installed.") && said.contains(BY_HAND), "{said}");
        assert!(said.contains(&by_hand_line(&mac)), "{said}");
        assert!(mac.app().exists() && mac.binary().exists());
    }
}
