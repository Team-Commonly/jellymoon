//! F33: one command installs Jellymoon on a Mac with no Apple Developer signature, and `jellymoon uninstall` takes it
//! off again. The Mac tests make a release in a scratch folder with `app/macos/bundle.sh` (the built `jellymoon`
//! stands in for the app's binary), hand it to `install.sh` as a `file://` folder, and point `HOME` and `TMPDIR` at
//! scratch folders: nothing here touches the real home or the network. The install is a Mac shell script by its
//! nature, so those tests are Mac only (ADR 0013, point 7); the uninstall test runs on every system.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::ffi::OsStr;
use std::fs;

use serde_json::{Value, json};
use support::{jellymoon, scratch};

#[test]
fn uninstall_removes_the_app_and_the_binary_and_nothing_else() {
    let dir = scratch();
    let (apps, bin) = (dir.path().join("Applications"), dir.path().join("bin"));
    let binary = bin.join(format!("jellymoon{}", std::env::consts::EXE_SUFFIX));
    fs::create_dir_all(apps.join("Jellymoon.app").join("Contents")).unwrap();
    fs::create_dir_all(&bin).unwrap();
    for file in [&apps.join("Jellymoon.app").join("Contents").join("Info.plist"), &apps.join("Other.app"), &binary] {
        fs::write(file, "made up").unwrap();
    }
    fs::write(bin.join("other-tool"), "made up").unwrap();
    let vars: [(&str, &OsStr); 2] = [("JELLYMOON_APP_DIR", apps.as_os_str()), ("JELLYMOON_BIN_DIR", bin.as_os_str())];

    let ran = jellymoon(&vars, &["uninstall"]);
    assert_eq!((ran.code, ran.stderr.as_str()), (0, ""));
    let said: Value = serde_json::from_str(&ran.stdout).unwrap();
    let removed = [apps.join("Jellymoon.app").display().to_string(), binary.display().to_string()];
    assert_eq!(said, json!({"hook": false, "hook_by_hand": false, "removed": removed}));
    let left = |folder| fs::read_dir(folder).unwrap().map(|entry| entry.unwrap().file_name()).collect::<Vec<_>>();
    assert_eq!((left(&apps), left(&bin)), (vec!["Other.app".into()], vec!["other-tool".into()]));

    // A second time there is nothing to remove, and that is not an error.
    let again = jellymoon(&vars, &["uninstall"]);
    assert_eq!((again.code, again.stdout.as_str()), (0, "{\"removed\":[],\"hook\":false,\"hook_by_hand\":false}\n"));
    // It takes no arguments: a path given to it is refused, never removed.
    let refused = jellymoon(&vars, &["uninstall", apps.to_str().unwrap()]);
    assert_eq!((refused.code, refused.stdout.as_str()), (2, ""));
    assert!(apps.join("Other.app").exists());
}

/// Anywhere but a Mac the script says it is for the Mac, fails, and installs nothing.
#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn the_install_script_refuses_a_system_that_is_not_a_mac() {
    let home = scratch();
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../install.sh");
    let mut sh = std::process::Command::new("sh");
    sh.arg(script).env("HOME", home.path()).env("TMPDIR", home.path());
    let ran = sh.env("JELLYMOON_RELEASE_URL", "file:///nowhere").output().unwrap();
    assert!(!ran.status.success());
    assert!(String::from_utf8_lossy(&ran.stderr).contains("for the Mac only"));
    assert_eq!(fs::read_dir(home.path()).unwrap().count(), 0);
}

#[cfg(target_os = "macos")]
mod mac {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Output, Stdio};

    use serde_json::{Value, json};
    use tempfile::TempDir;

    use super::support::scratch;

    const QUARANTINE: &str = "com.apple.quarantine";

    fn repo() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    /// The archive names end in the Mac's own names for its chips, not Rust's.
    fn chips() -> (&'static str, &'static str) {
        if cfg!(target_arch = "aarch64") { ("arm64", "x86_64") } else { ("x86_64", "arm64") }
    }

    fn run(command: &mut Command) -> Output {
        command.stdin(Stdio::null()).output().unwrap()
    }

    fn ok(command: &mut Command) -> String {
        let ran = run(command);
        assert!(ran.status.success(), "{command:?} failed: {}", String::from_utf8_lossy(&ran.stderr));
        String::from_utf8(ran.stdout).unwrap()
    }

    fn write_sums(release: &Path) {
        let (this, other) = chips();
        let names = [format!("jellymoon-macos-{this}.tar.gz"), format!("jellymoon-macos-{other}.tar.gz")];
        let sums = ok(Command::new("shasum").args(["-a", "256"]).args(names).current_dir(release));
        fs::write(release.join("SHA256SUMS"), sums).unwrap();
    }

    /// A release folder as the workflow makes one: an archive for each chip and SHA256SUMS. This Mac's archive
    /// carries `version`; the other chip's carries 9.9.9, so installing the wrong one shows.
    fn release(root: &Path, name: &str, version: &str) -> PathBuf {
        let release = root.join(name);
        let (this, other) = chips();
        for (chip, version) in [(this, version), (other, "9.9.9")] {
            let built = env!("CARGO_BIN_EXE_jellymoon");
            ok(Command::new("sh")
                .arg(repo().join("app/macos/bundle.sh"))
                .args([built, built, version, chip])
                .arg(&release));
        }
        write_sums(&release);
        release
    }

    /// A made-up machine: a home and a temp folder, both empty, and the paths the install uses by default.
    struct Mac {
        root: TempDir,
    }

    impl Mac {
        fn new() -> Mac {
            let root = scratch();
            fs::create_dir(root.path().join("home")).unwrap();
            fs::create_dir(root.path().join("tmp")).unwrap();
            Mac { root }
        }
        fn home(&self) -> PathBuf {
            self.root.path().join("home")
        }
        fn app(&self) -> PathBuf {
            self.home().join("Applications/Jellymoon.app")
        }
        fn binary(&self) -> PathBuf {
            self.home().join(".local/bin/jellymoon")
        }
        /// Every variable is set here: the real home, the real `PATH` and the real release address are never seen.
        fn command(&self, program: &Path, path: &str) -> Command {
            let mut command = Command::new(program);
            command.env_clear().env("HOME", self.home()).env("TMPDIR", self.root.path().join("tmp")).env("PATH", path);
            command
        }
        fn install(&self, release: &Path) -> Output {
            self.install_on(release, "/usr/bin:/bin:/usr/sbin")
        }
        fn install_on(&self, release: &Path, path: &str) -> Output {
            let mut sh = self.command(Path::new("/bin/sh"), path);
            run(sh.arg(repo().join("install.sh")).env("JELLYMOON_RELEASE_URL", format!("file://{}", release.display())))
        }
        /// Installs with a program of this name first on the script's `PATH`, standing in for the system's.
        fn install_with(&self, release: &Path, name: &str, body: &str) -> Output {
            let shims = self.root.path().join("shims");
            fs::create_dir_all(&shims).unwrap();
            fs::write(shims.join(name), format!("#!/bin/sh\n{body}\n")).unwrap();
            ok(Command::new("chmod").arg("755").arg(shims.join(name)));
            self.install_on(release, &format!("{}:/usr/bin:/bin:/usr/sbin", shims.display()))
        }
        fn version(&self) -> String {
            let plist = fs::read_to_string(self.app().join("Contents/Info.plist")).unwrap();
            let after = plist.split("<key>CFBundleVersion</key><string>").nth(1).unwrap();
            after.split('<').next().unwrap().to_string()
        }
        /// The names in a folder under the made-up machine; empty when the folder was never made.
        fn names(&self, folder: &str) -> Vec<String> {
            let Ok(entries) = fs::read_dir(self.root.path().join(folder)) else { return Vec::new() };
            entries.map(|entry| entry.unwrap().file_name().into_string().unwrap()).collect()
        }
    }

    fn text(bytes: &[u8]) -> String {
        String::from_utf8_lossy(bytes).into_owned()
    }

    fn quarantined(path: &Path) -> bool {
        ok(Command::new("xattr").arg("-r").arg(path)).contains(QUARANTINE)
    }

    #[test]
    fn it_installs_this_macs_release_into_the_users_own_folders() {
        let mac = Mac::new();
        let ran = mac.install(&release(mac.root.path(), "release", "1.0.0"));
        assert!(ran.status.success(), "{}", text(&ran.stderr));

        assert_eq!(mac.version(), "1.0.0", "the archive for this Mac's chip, not the other one");
        // The bundle states the minimum system that install.sh checks for, read from the script's own line.
        let plist = fs::read_to_string(mac.app().join("Contents/Info.plist")).unwrap();
        assert!(plist.contains("<key>LSMinimumSystemVersion</key><string>11.0</string>"), "{plist}");
        assert_eq!(mac.names("home/Applications"), ["Jellymoon.app"]);
        assert_eq!(mac.names("home/.local/bin"), ["jellymoon"]);
        assert_eq!(mac.names("tmp"), Vec::<String>::new(), "the download folder is gone");
        // What Gatekeeper would ask about is not there, and the bundle's ad-hoc signature survived the copy.
        assert!(!quarantined(&mac.home()));
        ok(Command::new("codesign").args(["--verify", "--strict"]).arg(mac.app()));
        ok(Command::new("codesign").args(["--verify", "--strict"]).arg(mac.binary()));
        // The installed command starts.
        assert!(ok(mac.command(&mac.binary(), "").arg("--help")).contains("jellymoon uninstall"));

        let said = text(&ran.stdout);
        let bin = mac.home().join(".local/bin");
        assert!(said.contains("Jellymoon is installed."), "{said}");
        assert_eq!(said.lines().filter(|line| line.contains("is not on your PATH")).count(), 1, "{said}");
        // With the folder on PATH there is no such line. Read from stdin, as `curl ... | sh` runs it.
        let mut sh = mac.command(Path::new("/bin/sh"), &format!("/usr/bin:/bin:/usr/sbin:{}", bin.display()));
        sh.env("JELLYMOON_RELEASE_URL", format!("file://{}", mac.root.path().join("release").display()));
        let piped = sh.stdin(fs::File::open(repo().join("install.sh")).unwrap()).output().unwrap();
        assert!(piped.status.success() && !text(&piped.stdout).contains("PATH"), "{}", text(&piped.stdout));
    }

    #[test]
    fn a_download_that_does_not_match_its_checksum_installs_nothing() {
        let mac = Mac::new();
        let release = release(mac.root.path(), "release", "1.0.0");
        let archive = release.join(format!("jellymoon-macos-{}.tar.gz", chips().0));
        let sums = fs::read_to_string(release.join("SHA256SUMS")).unwrap();

        // One byte more than what was summed.
        let mut bytes = fs::read(&archive).unwrap();
        bytes.push(0);
        fs::write(&archive, bytes).unwrap();
        let tampered = mac.install(&release);
        assert_eq!(tampered.status.code(), Some(1));
        assert!(text(&tampered.stderr).contains("does not match SHA256SUMS; nothing was installed"));
        assert_eq!((mac.names("home"), mac.names("tmp")), (Vec::<String>::new(), Vec::new()));

        // A list of checksums that does not name the archive is no checksum.
        fs::write(release.join("SHA256SUMS"), sums.replace(chips().0, "other")).unwrap();
        let unlisted = mac.install(&release);
        assert!(text(&unlisted.stderr).contains("no checksum for"), "{}", text(&unlisted.stderr));
        // And a release that is not there fails the same way.
        let missing = mac.install(&mac.root.path().join("no-release"));
        assert!(text(&missing.stderr).contains("could not download"), "{}", text(&missing.stderr));
        assert_eq!((mac.names("home"), mac.names("tmp")), (Vec::<String>::new(), Vec::new()));
    }

    #[test]
    fn a_quarantine_flag_that_came_with_the_download_is_taken_off_what_was_installed() {
        let mac = Mac::new();
        let release = release(mac.root.path(), "release", "1.0.0");
        let archive = release.join(format!("jellymoon-macos-{}.tar.gz", chips().0));
        // Repack this Mac's archive with the flag on everything in it; the Mac's tar carries extended attributes.
        let flagged = mac.root.path().join("flagged");
        fs::create_dir(&flagged).unwrap();
        ok(Command::new("tar").arg("-xzf").arg(&archive).arg("-C").arg(&flagged));
        ok(Command::new("xattr").args(["-wr", QUARANTINE, "0081;00000000;made-up;"]).arg(&flagged));
        ok(Command::new("tar").arg("-czf").arg(&archive).arg("-C").arg(&flagged).args(["Jellymoon.app", "jellymoon"]));
        write_sums(&release);
        let probe = mac.root.path().join("probe");
        fs::create_dir(&probe).unwrap();
        ok(Command::new("tar").arg("-xzf").arg(&archive).arg("-C").arg(&probe));
        assert!(quarantined(&probe), "the archive under test must carry the flag, or this proves nothing");

        let ran = mac.install(&release);
        assert!(ran.status.success(), "{}", text(&ran.stderr));
        assert!(mac.binary().exists() && !quarantined(&mac.home()));
        ok(Command::new("codesign").args(["--verify", "--strict"]).arg(mac.app()));
    }

    #[test]
    fn a_second_run_updates_in_place() {
        let mac = Mac::new();
        assert!(mac.install(&release(mac.root.path(), "one", "1.0.0")).status.success());
        fs::write(mac.app().join("Contents/left-from-1.0.0"), "made up").unwrap();

        let ran = mac.install(&release(mac.root.path(), "two", "2.0.0"));
        assert!(ran.status.success(), "{}", text(&ran.stderr));
        assert!(text(&ran.stdout).contains("Jellymoon is updated."));
        assert_eq!(mac.version(), "2.0.0");
        assert!(!mac.app().join("Contents/left-from-1.0.0").exists(), "the old bundle is replaced, not added to");
        assert_eq!(mac.names("home/Applications"), ["Jellymoon.app"]);
        assert_eq!(mac.names("home/.local/bin"), ["jellymoon"]);
        ok(Command::new("codesign").args(["--verify", "--strict"]).arg(mac.app()));
    }

    #[test]
    fn an_update_that_stops_between_its_two_moves_leaves_the_old_app_in_place() {
        let mac = Mac::new();
        assert!(mac.install(&release(mac.root.path(), "one", "1.0.0")).status.success());
        let two = release(mac.root.path(), "two", "2.0.0");

        // An `mv` that fails when asked to move the new bundle in, after the old one was moved aside.
        let fails = "case \"$1\" in */.Jellymoon.app.new) exit 1 ;; esac\nexec /bin/mv \"$@\"";
        let stopped = mac.install_with(&two, "mv", fails);
        assert_eq!(stopped.status.code(), Some(1), "{}", text(&stopped.stdout));
        assert_eq!(mac.version(), "1.0.0");
        assert_eq!(mac.names("home/Applications"), ["Jellymoon.app"], "the old app is back, the new copy is gone");
        assert_eq!((mac.names("home/.local/bin"), mac.names("tmp")), (vec!["jellymoon".to_string()], Vec::new()));
        ok(Command::new("codesign").args(["--verify", "--strict"]).arg(mac.app()));

        // Killed outright between the moves, only the copy moved aside is left. The next run puts it back first,
        // even a run that then finds nothing to install.
        fs::rename(mac.app(), mac.home().join("Applications/.Jellymoon.app.old")).unwrap();
        assert!(!mac.install(&mac.root.path().join("no-release")).status.success());
        assert_eq!(
            (mac.names("home/Applications"), mac.version().as_str()),
            (vec!["Jellymoon.app".to_string()], "1.0.0")
        );
    }

    #[test]
    fn an_old_mac_and_a_folder_where_the_command_goes_are_refused_before_any_download() {
        let mac = Mac::new();
        let nowhere = mac.root.path().join("no-release");
        let old = mac.install_with(&nowhere, "sw_vers", "echo 10.15.7");
        assert!(
            text(&old.stderr).contains("needs macOS 11.0 or newer and this Mac has 10.15.7"),
            "{}",
            text(&old.stderr)
        );
        assert_eq!((old.status.code(), mac.names("home")), (Some(1), Vec::new()));

        fs::create_dir_all(mac.binary().join("kept")).unwrap();
        let folder = mac.install(&nowhere);
        assert!(text(&folder.stderr).contains("is a folder, not the jellymoon command"), "{}", text(&folder.stderr));
        let left = (mac.names("home/.local/bin/jellymoon"), mac.names("home/Applications"));
        assert_eq!(left, (vec!["kept".to_string()], Vec::new()));
    }

    #[test]
    fn the_installed_command_uninstalls_everything_the_script_added() {
        let mac = Mac::new();
        assert!(mac.install(&release(mac.root.path(), "release", "1.0.0")).status.success());

        let said: Value = serde_json::from_str(&ok(mac.command(&mac.binary(), "").arg("uninstall"))).unwrap();
        let removed = [mac.app().display().to_string(), mac.binary().display().to_string()];
        assert_eq!(said, json!({"hook": false, "hook_by_hand": false, "removed": removed}));
        assert_eq!(mac.names("home/Applications"), Vec::<String>::new());
        assert_eq!(mac.names("home/.local/bin"), Vec::<String>::new());
    }

    /// Read as text: outside its comments the script never asks for root and never touches Gatekeeper's switch.
    #[test]
    fn the_script_never_asks_for_root_or_turns_gatekeeper_off() {
        let script = fs::read_to_string(repo().join("install.sh")).unwrap();
        let code: Vec<&str> = script.lines().filter(|line| !line.trim_start().starts_with('#')).collect();
        for word in ["sudo", "spctl", "master-disable", "/Applications", "/usr/local"] {
            let found: Vec<&&str> =
                code.iter().filter(|line| line.replace("$HOME/Applications", "").contains(word)).collect();
            assert!(found.is_empty(), "{word} in install.sh: {found:?}");
        }
    }
}
