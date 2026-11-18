//! A release is published for Apple-chip Macs first, and the Intel archive joins it when it is built
//! (docs/development/install.md, "Cutting a release"). Until it has, an Intel Mac is told so in one line and nothing
//! is installed: both before the archive is attached and in the moment it is attached and `SHA256SUMS` does not
//! name it yet. A made-up `sysctl` and `uname` first on `PATH` make the script read this Mac as an Intel one.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use super::mac::{Mac, chips, ok, release, repo, run, text};

const NO_INTEL: &str = "jellymoon install: this release has no build for an Intel Mac yet; nothing was installed\n";

fn install_as_intel(mac: &Mac, release: &Path) -> Output {
    let shims = mac.root.path().join("shims");
    fs::create_dir_all(&shims).unwrap();
    let uname = "case \"$1\" in -m) echo x86_64 ;; *) exec /usr/bin/uname \"$@\" ;; esac";
    for (name, body) in [("sysctl", "echo 0"), ("uname", uname)] {
        fs::write(shims.join(name), format!("#!/bin/sh\n{body}\n")).unwrap();
        ok(Command::new("chmod").arg("755").arg(shims.join(name)));
    }
    let path = format!("{}:/usr/bin:/bin:/usr/sbin", shims.display());
    let script = repo().join("install.sh");
    run(&mut mac.command(Path::new("/bin/sh"), &path, &[script.to_str().unwrap()], Some(release)))
}

#[test]
fn an_intel_mac_is_told_the_release_has_no_build_for_it_yet_and_nothing_is_installed() {
    let mac = Mac::new();
    let release = release(mac.root.path(), "release", "1.0.0");
    let intel = release.join("jellymoon-macos-x86_64.tar.gz");
    let sums = fs::read_to_string(release.join("SHA256SUMS")).unwrap();
    let apple = sums.lines().find(|line| line.ends_with("arm64.tar.gz")).unwrap();
    fs::write(release.join("SHA256SUMS"), format!("{apple}\n")).unwrap();
    let kept = mac.root.path().join("x86_64.tar.gz");
    fs::rename(&intel, &kept).unwrap();

    // As the release is published: the Apple-chip archive and a SHA256SUMS of one line.
    let before = install_as_intel(&mac, &release);
    assert_eq!((before.status.code(), text(&before.stderr)), (Some(1), NO_INTEL.to_string()));
    assert_eq!((mac.names("home"), mac.names("tmp")), (Vec::<String>::new(), Vec::new()));

    // The Intel archive attached, and SHA256SUMS not yet replaced: the same line, and still nothing installed.
    fs::rename(&kept, &intel).unwrap();
    let between = install_as_intel(&mac, &release);
    assert_eq!((between.status.code(), text(&between.stderr)), (Some(1), NO_INTEL.to_string()));
    assert_eq!((mac.names("home"), mac.names("tmp")), (Vec::<String>::new(), Vec::new()));

    // With both lines the Intel archive is the one installed: `release` gives the other chip's archive 9.9.9.
    fs::write(release.join("SHA256SUMS"), &sums).unwrap();
    let after = install_as_intel(&mac, &release);
    assert!(after.status.success(), "{}", text(&after.stderr));
    assert_eq!(mac.version(), if chips().0 == "x86_64" { "1.0.0" } else { "9.9.9" });
}
