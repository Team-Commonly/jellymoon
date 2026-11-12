//! `app/macos/bundle.sh` writes the oldest macOS the app runs on into the bundle, and `install.sh` compares only the
//! first number of it. So the script refuses a minimum that is not a whole release, before it makes anything. It is a
//! shell script for a Mac (ADR 0013), so it is run on a Mac only, on a copy beside a made-up `install.sh`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[cfg(target_os = "macos")]
#[test]
fn a_minimum_macos_that_is_not_a_whole_release_is_refused_before_anything_is_made() {
    use std::path::Path;
    use std::process::Command;
    use std::{env, fs};

    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let script = dir.path().join("app").join("macos").join("bundle.sh");
    fs::create_dir_all(script.parent().unwrap()).unwrap();
    fs::copy(repo.join("app").join("macos").join("bundle.sh"), &script).unwrap();
    let (file, out) = (dir.path().join("a-file"), dir.path().join("out"));
    fs::write(&file, "").unwrap();
    let bundle = |least: &str| {
        fs::write(dir.path().join("install.sh"), format!("MIN_MACOS={least}\n")).unwrap();
        let mut sh = Command::new("sh");
        sh.arg(&script).args([&file, &file]).args(["1.0", "arm64"]).arg(&out);
        let ran = sh.env("TMPDIR", dir.path()).output().unwrap();
        (ran.status.code(), String::from_utf8_lossy(&ran.stderr).into_owned())
    };
    for least in ["12.3", "11.0.1", "11.5.0", "11."] {
        let said = format!(
            "bundle: MIN_MACOS={least} in install.sh is not a whole release: install.sh compares the first number \
             only, so write <number>.0\n"
        );
        assert_eq!(bundle(least), (Some(1), said), "{least}");
        assert!(!out.exists(), "{least}: something was made");
    }
    for least in [".0", "11.x", ""] {
        assert_eq!(bundle(least), (Some(1), "bundle: install.sh has no MIN_MACOS=<number> line\n".to_string()));
    }
    // A whole release goes on to the signing, which a made-up file fails or passes: the refusal is not said.
    for least in ["12", "11.0", "11.0.0"] {
        let (_, said) = bundle(least);
        assert!(!said.contains("MIN_MACOS"), "{least}: {said}");
    }
}
