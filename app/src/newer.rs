//! A newer build is installed (`docs/development/app.md`, "A newer build is installed"). On a Mac the installer
//! replaces the app's bundle in place while an open window goes on with the build it loaded. The running program
//! does not know its version: the release stamps it only into the bundle's `Info.plist` (`app/macos/bundle.sh`).
//! So the version is read from that file when the window opens and kept, read again at most once [`EVERY`], and
//! when the one on disk is higher the page has one plain note, the last of its column. A file is read and no
//! more: no program is started, nothing is written, and every failure is silence.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use sticker_kit::Note;

/// The note's title and its body.
pub const TITLE: &str = "A newer Jellymoon is installed.";
pub const BODY: &str = "Quit Jellymoon and open it again to use it.";

/// The least time between two reads of the version on disk.
pub const EVERY: Duration = Duration::from_secs(60);

/// The largest property list that is read.
pub const MOST_BYTES: u64 = 64 * 1024;

const KEY: &str = "<key>CFBundleShortVersionString</key>";

/// A version: its numbers with the zeros at their end left out (`0.3` is `0.3.0`), and whether it has no
/// `-suffix`. In that order one is higher than another: by its numbers, as numbers, and with the same numbers
/// the one without a suffix.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(Vec<u64>, bool);

/// `0.2.10` or `0.3.0-rc1` as a version. None of anything else, and of `0.0.0`, which is no release's version.
pub fn version(text: &str) -> Option<Version> {
    let (numbers, suffix) = text.split_once('-').map_or((text, None), |(numbers, suffix)| (numbers, Some(suffix)));
    if suffix.is_some_and(str::is_empty) {
        return None;
    }
    let number = |piece: &str| piece.bytes().all(|byte| byte.is_ascii_digit()).then(|| piece.parse().ok()).flatten();
    let mut numbers = numbers.split('.').map(number).collect::<Option<Vec<u64>>>()?;
    while numbers.last() == Some(&0) {
        numbers.pop();
    }
    (!numbers.is_empty()).then_some(Version(numbers, suffix.is_none()))
}

/// The version in the property list at `plist`, as `bundle.sh` writes it: the `<string>` after the key
/// `CFBundleShortVersionString`. None of no file, of a file over [`MOST_BYTES`], and of one that is not that text.
pub fn read(plist: &Path) -> Option<Version> {
    let about = std::fs::metadata(plist).ok()?;
    if !about.is_file() || about.len() > MOST_BYTES {
        return None;
    }
    let mut text = String::new();
    std::fs::File::open(plist).ok()?.take(MOST_BYTES + 1).read_to_string(&mut text).ok()?;
    if text.len() as u64 > MOST_BYTES {
        return None;
    }
    let after = text.split_once(KEY)?.1.trim_start().strip_prefix("<string>")?;
    version(after.split_once("</string>")?.0)
}

/// The bundle a program at `exe` runs from, `<bundle>/Contents/MacOS/<name>`. None of a program in no bundle.
pub fn bundle_of(exe: &Path) -> Option<&Path> {
    let (macos, contents) = (exe.parent()?, exe.parent()?.parent()?);
    (macos.file_name()? == "MacOS" && contents.file_name()? == "Contents").then(|| contents.parent()).flatten()
}

/// The read of the version on disk, as work for another thread: whether it is higher than the one kept.
pub type Check = Box<dyn FnOnce() -> bool + Send>;

/// One window's watch of the bundle it runs from. It is the window's own: kept on its thread, but for the one
/// flag a [`Check`] sets from another.
#[derive(Default)]
pub struct Watch {
    /// The bundle's property list and the version it held at the open. None: there is nothing to read, ever.
    opened: Option<(PathBuf, Version)>,
    found: Arc<AtomicBool>,
    /// When the last check was handed out.
    checked: std::cell::Cell<Option<Instant>>,
    /// Whether a column was given with the note in it.
    drawn: std::cell::Cell<bool>,
}

impl Watch {
    /// The watch of `bundle`, whose version is read now. With no bundle, or no version in it, nothing is read again.
    pub fn of(bundle: Option<&Path>) -> Self {
        let plist = bundle.map(|bundle| bundle.join("Contents").join("Info.plist"));
        let opened = plist.and_then(|plist| read(&plist).map(|version| (plist, version)));
        Watch { opened, ..Watch::default() }
    }

    /// The watch of the bundle this program runs from. A program in no bundle (a `cargo run`, a test, Windows,
    /// Linux) has none.
    pub fn running() -> Self {
        Watch::of(std::env::current_exe().ok().as_deref().and_then(bundle_of))
    }

    /// The check a look at `now` carries, or none: there is nothing to read, the last check is not [`EVERY`]
    /// old yet, or a newer build was found and its note is drawn. Found and not drawn yet (the page changed
    /// under the look that found it), the check reads nothing and says so again.
    pub fn check(&self, now: Instant) -> Option<Check> {
        let (plist, opened) = self.opened.clone()?;
        if self.found.load(Ordering::Relaxed) {
            return (!self.drawn.get()).then(|| Box::new(|| true) as Check);
        }
        if self.checked.get().is_some_and(|last| now.saturating_duration_since(last) < EVERY) {
            return None;
        }
        self.checked.set(Some(now));
        let found = self.found.clone();
        Some(Box::new(move || {
            let newer = read(&plist).is_some_and(|on_disk| on_disk > opened);
            if newer {
                found.store(true, Ordering::Relaxed);
            }
            newer
        }))
    }

    /// `notes` with the note after them, once a newer build was found: the notes of a scene on its way to its column.
    pub fn noted(&self, mut notes: Vec<Note>) -> Vec<Note> {
        if self.found.load(Ordering::Relaxed) {
            self.drawn.set(true);
            notes.push(Note::Plain { title: TITLE.to_string(), body: BODY.to_string() });
        }
        notes
    }
}

/// Whether `note` is the note of a newer build: `column.rs` stands it last.
pub fn names(note: &Note) -> bool {
    matches!(note, Note::Plain { title, .. } if title == TITLE)
}
