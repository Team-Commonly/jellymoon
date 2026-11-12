//! How the kit's tests try a theme's art: through a trial, in a child, as every caller does. No test reads a
//! theme's picture for drawing any other way.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use sticker_kit::theme::{Accepted, Trial};
use sticker_kit::{Loaded, Theme, ThemeError};

pub const HONEST: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><circle cx="5" cy="5" r="4" fill="#123456"/></svg>"##;

/// How much longer than its limit a trial may be seen to take, on a machine that is busy with other tests.
pub const OUTER: Duration = Duration::from_secs(20);

/// A fresh folder in the target folder's place for tests, removed on drop: a test that fails leaves nothing in
/// the system's temporary folder.
pub struct Folder(pub PathBuf);

impl Folder {
    pub fn new(name: &str) -> Self {
        let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("trial-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("a folder for the test");
        Folder(path)
    }

    /// A theme whose star is `svg`, and what a trial says of it: the picture given, or the refusal's reason.
    pub fn star(&self, svg: &str, trial: &Trial) -> Result<String, String> {
        fs::write(self.0.join("star.svg"), svg).unwrap();
        fs::write(self.0.join("theme.toml"), "[art]\nstar = \"star.svg\"\n").unwrap();
        let mut tried = Theme::load(&self.0.join("theme.toml")).expect("the theme loads").theme.try_art(trial);
        match (tried.accepted.pop(), tried.refused.pop()) {
            (Some(accepted), None) => Ok(accepted.svg().to_string()),
            (None, Some(ThemeError::RefusedSvg { key, reason })) if key == "art.star" => Err(reason),
            other => panic!("one picture has one answer: {other:?}"),
        }
    }

    /// How many answers are kept in `kept`, a folder inside this one.
    pub fn kept(&self, kept: &str) -> usize {
        fs::read_dir(self.0.join(kept)).map_or(0, Iterator::count)
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub fn sized(side: u32, inside: &str) -> String {
    format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="{side}" height="{side}">{inside}</svg>"#)
}

/// The picture the gate on PR 154 found, inside every written rule: `cubics` pairs of curves a million long, under
/// a stroke as wide, all scaled down to nothing. 30 pairs drew for ten seconds in a debug build and 250 for seven
/// in a release one. No test lets one run past a trial's limit.
pub fn wide_stroke(cubics: usize) -> String {
    let d = format!("M0 0{}{}", "m1e6 1e6".repeat(10), "c1e6 0 0 1e6 1e6 1e6c-1e6 0 0 -1e6 -1e6 -1e6".repeat(cubics));
    let path = format!(r##"<path d="{d}" fill="none" stroke="#123456" stroke-width="1e6"/>"##);
    sized(64, &format!(r#"<g transform="scale(1e-6)">{path}</g>"#))
}

/// The helper started by hand as `which`, given `picture`, and not watched as a trial watches: how it ended
/// within `wait` (none when it had not, and it is then ended here), what it said, and how long it lived. With
/// `open`, its input is held open after the picture.
pub fn alone(which: &[&str], picture: &str, wait: Duration, open: bool) -> (Option<ExitStatus>, String, Duration) {
    let _ = trial();
    let mut command = Command::new(env!("CARGO_BIN_EXE_trial-child"));
    command.args(which).env_clear().stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
    if let Some(root) = std::env::var_os("SystemRoot").filter(|_| cfg!(windows)) {
        command.env("SystemRoot", root);
    }
    let started = Instant::now();
    let mut child = command.spawn().expect("the helper starts");
    let (mut stdin, mut stdout) = (child.stdin.take().unwrap(), child.stdout.take().unwrap());
    let (text, (done, hold)) = (picture.to_owned(), std::sync::mpsc::channel::<()>());
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(text.as_bytes());
        if open {
            let _ = hold.recv();
        }
    });
    let reader = std::thread::spawn(move || {
        let mut said = String::new();
        let _ = stdout.read_to_string(&mut said);
        said
    });
    let mut status = None;
    while status.is_none() && started.elapsed() < wait {
        std::thread::sleep(Duration::from_millis(5));
        status = child.try_wait().expect("the helper is waited for");
    }
    let lived = started.elapsed();
    let _ = child.kill();
    let _ = child.wait();
    drop(done);
    let _ = writer.join();
    (status, reader.join().unwrap(), lived)
}

/// A trial whose child is the tests' helper, started as `which`, with no answer kept.
pub fn trial_as(which: &[&str]) -> Trial {
    // A system may look a program over the first time it is started, for seconds (macOS, a fresh build). The
    // helper is started once with no limit before any trial counts its time. An app tries with its own running
    // program, which has been looked over already.
    static STARTED: std::sync::Once = std::sync::Once::new();
    STARTED.call_once(|| {
        let started = std::time::Instant::now();
        let _ = std::process::Command::new(env!("CARGO_BIN_EXE_trial-child")).arg("fails").output();
        eprintln!("the helper's first start took {:?}", started.elapsed());
    });
    let mut trial = Trial::new(PathBuf::from(env!("CARGO_BIN_EXE_trial-child")), which[0], None, "tests");
    trial.arguments = which.iter().map(|argument| argument.to_string()).collect();
    trial
}

/// The kit's own child.
pub fn trial() -> Trial {
    trial_as(&["trial"])
}

/// A theme file read and its art tried: the theme and the pictures to draw, or the first thing refused.
pub fn tried(path: &Path) -> Result<(Loaded, Vec<Accepted>), ThemeError> {
    let loaded = Theme::load(path)?;
    let tried = loaded.theme.try_art(&trial());
    match tried.refused.into_iter().next() {
        Some(refused) => Err(refused),
        None => Ok((loaded, tried.accepted)),
    }
}

/// Which pieces were given a picture, and the text of each.
pub fn given(art: &[Accepted]) -> Vec<(&'static str, String)> {
    art.iter().map(|accepted| (accepted.piece(), accepted.svg().to_string())).collect()
}
