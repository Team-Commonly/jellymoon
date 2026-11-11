//! What a tap on a day's page does (F35, `docs/development/delivery.md`, "A tap in the app"). [`settle`] is a tap on
//! one statement of a clash: it makes at most one ruling, of that clash's two statements and nothing else, and keeps
//! which one stands beside the day's kept page. [`turn_on_delivery`] is the tap that installs the hook, for the
//! installed `jellymoon` command and only after that file was checked. The app draws what these return; it decides
//! nothing.
//!
//! The order of a tap is fixed: the day's lock, then the ruling, then the settlement. A ruling whose settlement could
//! not be written is still a ruling, and [`Settled::unkept`] says so. Which side stands is read from the rulings that
//! wait as well as from the settlement, so the same tap again makes no second ruling.

use std::fs::{self, OpenOptions};
use std::io::{self, ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use std::{fmt, thread};

use serde::{Deserialize, Serialize};

use crate::busy::{in_use, retried};
use crate::hook::put;
use crate::reader::parse;
use crate::ruling::{own_dir, private_dir, rule, ruled_on, waiting, words};
use crate::{Clash, Error, Ruling};

/// The most bytes of a kept page that are read: what the app's own door takes.
const PAGE_MAX: usize = 1024 * 1024;
/// The most bytes of a day's settlement that are read. Eight clashes of two 200-character statements are far less.
const RULED_MAX: usize = 64 * 1024;
/// The most clashes of a page that can be tapped: the first eight, which are the ones the app draws.
pub const CLASHES_MAX: usize = 8;
/// How long a tap waits for another tap on the same day, and how old a lock is before it is taken for one left by a
/// run that was killed. A lock whose age cannot be read, or that is dated ahead of now, is taken for one too.
const LOCK_WAIT: Duration = crate::busy::WAIT;
const LOCK_STALE: Duration = Duration::from_secs(30);

/// One of a clash's two statements: `a` is its `quote_a`, `b` its `quote_b`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    A,
    B,
}

/// A clash as a tap would rule on it: its two statements in the exact words a ruling would carry, and the one that
/// stands now by the person's own ruling. The page draws these strings, so what is tapped is what is sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Standing {
    pub a: String,
    pub b: String,
    pub stands: Option<Side>,
}

impl Standing {
    /// Whether there is anything to choose: two statements that are not the same once cleaned, neither empty. A
    /// clash that is not a choice is never drawn as one, and [`settle`] refuses it.
    pub fn is_choice(&self) -> bool {
        !self.a.is_empty() && !self.b.is_empty() && self.a != self.b
    }
}

/// What a tap on a clash did, as `jellymoon settle` prints it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Settled {
    /// The ruling this tap made. None when the statement tapped stands already: nothing was made.
    pub ruling: Option<Ruling>,
    /// The statement that stands and the one it replaces, in the exact words of the ruling.
    pub stands: String,
    pub replaces: String,
    /// Why the page could not keep which side stands, when it could not. The ruling was made all the same.
    pub unkept: Option<String>,
}

impl fmt::Display for Settled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_json::to_string(self).map_err(|_| fmt::Error)?)
    }
}

/// The settlement of one day, `pages/<day>.ruled.json`. It says which side stands and is never the source of a ruling.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Ruled {
    day: String,
    clashes: Vec<One>,
}

/// One clash by its two statements as ruled, and the side the person chose.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct One {
    a: String,
    b: String,
    stands: Side,
}

/// The parts of a kept page a tap needs. The clashes are looked at one by one, and only the first eight.
#[derive(Deserialize)]
struct Kept {
    day: String,
    clashes: Vec<serde_json::Value>,
}

fn refused(why: impl Into<String>) -> Error {
    Error::Tap(why.into())
}

/// A file of the pages folder as it may be used: a plain file, never a link, that only its owner can write to, of
/// at most `most` bytes. Windows refuses to open one that is being replaced at that moment: tried again (`busy.rs`).
pub(crate) fn plain(file: &Path, most: usize) -> io::Result<Vec<u8>> {
    retried(|| read_plain(file, most))
}

fn read_plain(file: &Path, most: usize) -> io::Result<Vec<u8>> {
    let not = |why: &str| io::Error::new(ErrorKind::InvalidData, why.to_string());
    let meta = fs::symlink_metadata(file)?;
    if !meta.is_file() {
        return Err(not("it is not a plain file"));
    }
    #[cfg(unix)]
    if std::os::unix::fs::PermissionsExt::mode(&meta.permissions()) & 0o022 != 0 {
        return Err(not("others can write to it"));
    }
    let mut bytes = Vec::new();
    fs::File::open(file)?.take(most as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > most { Err(not("it is too large")) } else { Ok(bytes) }
}

/// The bytes of the day's kept page, `pages/<day>.json`, for the app's door and for [`settle_kept`]. Refused: a day
/// that is not a date, no file, a link, a file that its group or others can write to, a file over 1 MiB.
pub fn kept_page(home: &Path, day: &str) -> Result<Vec<u8>, Error> {
    parse(day)?;
    plain(&home.join("pages").join(format!("{day}.json")), PAGE_MAX).map_err(|e| match e.kind() {
        ErrorKind::NotFound => refused(format!("no page is kept for {day}")),
        _ => refused(format!("the kept page of {day} is not used: {e}")),
    })
}

fn ruled_file(home: &Path, day: &str) -> PathBuf {
    home.join("pages").join(format!("{day}.ruled.json"))
}

/// The day's settlement. A file that is missing, a link, writable by others, too large, not of this exact shape, of
/// another day or of more clashes than a page has is no settlement. Each statement is read as the page's are, through
/// [`words`]: one kept before a statement was cleaned to its end is still the page's.
fn ruled(home: &Path, day: &str) -> Vec<One> {
    let kept = plain(&ruled_file(home, day), RULED_MAX).ok();
    let kept = kept.and_then(|bytes| serde_json::from_slice::<Ruled>(&bytes).ok()).filter(|kept| kept.day == day);
    let kept = kept.map(|kept| kept.clashes).filter(|clashes| clashes.len() <= CLASHES_MAX).unwrap_or_default();
    kept.into_iter().map(|one| One { a: words(&one.a), b: words(&one.b), ..one }).collect()
}

/// The side that stands for the clash of `a` and `b`: by the newest ruling of those two statements that still
/// waits, else by the settlement.
fn stands(waiting: &[Ruling], kept: &[One], a: &str, b: &str) -> Option<Side> {
    let told = waiting.iter().rev().find_map(|ruling| match (ruling.stands.as_str(), ruling.over.as_str()) {
        (stands, over) if stands == a && over == b => Some(Side::A),
        (stands, over) if stands == b && over == a => Some(Side::B),
        _ => None,
    });
    told.or_else(|| kept.iter().find(|one| one.a == a && one.b == b).map(|one| one.stands))
}

/// Each of the page's first eight clashes as it is drawn. Nothing is written.
pub fn standing(home: &Path, day: &str, clashes: &[Clash]) -> Vec<Standing> {
    let (waiting, kept) = (waiting(home), if parse(day).is_ok() { ruled(home, day) } else { Vec::new() });
    let one = |clash: &Clash| {
        let (a, b) = (words(&clash.quote_a), words(&clash.quote_b));
        let stands = stands(&waiting, &kept, &a, &b);
        Standing { a, b, stands }
    };
    clashes.iter().take(CLASHES_MAX).map(one).collect()
}

/// A tap on statement `option` (1 is `quote_a`, 2 is `quote_b`) of clash `clash` (from 1) of the day's kept page.
pub fn settle_kept(home: &Path, day: &str, clash: usize, option: usize) -> Result<Settled, Error> {
    let bytes = kept_page(home, day)?;
    let not_page = || refused(format!("the kept page of {day} is not used: it is not page data of that day"));
    let kept: Kept = serde_json::from_slice(&bytes).map_err(|_| not_page())?;
    if kept.day != day {
        return Err(not_page());
    }
    let first = kept.clashes.into_iter().take(CLASHES_MAX);
    let clashes: Vec<Clash> = first.map(serde_json::from_value).collect::<Result<_, _>>().map_err(|_| not_page())?;
    settle(home, day, &clashes, clash, option)
}

/// A tap on statement `option` of clash `clash` of these clashes, which are the day's page data. The statement tapped
/// stands and the other is what it replaces; both go through the ruling's own cleaning and no other text is added.
/// A refusal has written nothing: so is a tap on a clash with a statement that cannot be cleaned, which the page
/// draws as the mark. A tap on the statement that already stands makes no ruling.
pub fn settle(home: &Path, day: &str, clashes: &[Clash], clash: usize, option: usize) -> Result<Settled, Error> {
    parse(day)?;
    let found = clash.checked_sub(1).filter(|at| *at < CLASHES_MAX).and_then(|at| clashes.get(at));
    let found = found.ok_or_else(|| refused(format!("the page of {day} has no clash {clash}")))?;
    let side = match option {
        1 => Side::A,
        2 => Side::B,
        _ => return Err(refused(format!("a clash has two statements, 1 and 2, and {option} was asked for"))),
    };
    let (a, b) = (ruled_on(&found.quote_a)?, ruled_on(&found.quote_b)?);
    if a.is_empty() || b.is_empty() {
        return Err(Error::Ruling);
    }
    if a == b {
        let why = "are the same once cleaned, so there is nothing to choose";
        return Err(refused(format!("the two statements of clash {clash} of {day} {why}")));
    }
    let _lock = locked(home, day).map_err(Error::Store)?;
    let mut kept = ruled(home, day);
    let now = stands(&waiting(home), &kept, &a, &b);
    let (stands, replaces) = if side == Side::A { (&a, &b) } else { (&b, &a) };
    let ruling = if now == Some(side) { None } else { Some(rule(home, stands, replaces)?) };
    let (stands, replaces) = match &ruling {
        Some(ruling) => (ruling.stands.clone(), ruling.over.clone()),
        None => (stands.clone(), replaces.clone()),
    };
    let mut unkept = None;
    if !kept.iter().any(|one| one.a == a && one.b == b && one.stands == side) {
        // Only the clashes of the page as it is now are kept, this one with its new side.
        let on_page = |one: &One| {
            clashes.iter().take(CLASHES_MAX).any(|c| words(&c.quote_a) == one.a && words(&c.quote_b) == one.b)
        };
        kept.retain(|one| !(one.a == a && one.b == b) && on_page(one));
        kept.push(One { a, b, stands: side });
        let text = serde_json::to_string(&Ruled { day: day.to_string(), clashes: kept }).map_err(io::Error::from);
        let done = text.and_then(|text| put(&ruled_file(home, day), &format!("{text}\n"), false));
        unkept = done.err().map(|e| e.to_string());
    }
    Ok(Settled { ruling, stands, replaces, unkept })
}

/// Writes a file of Jellymoon's own in a folder for the person alone: the kept page. The folder is made, or made
/// 0700 when it was there; the text goes into a new file beside its place (never over a file that is there), for
/// the person alone, and is moved there. A link at the place is replaced, not followed.
pub fn keep_private(file: &Path, text: &str) -> io::Result<()> {
    if let Some(folder) = file.parent() {
        own_dir(folder)?;
    }
    put(file, text, false)
}

/// The day's lock, taken: what a tap and a made wish write behind. The pages folder is made when it is missing.
pub(crate) fn locked(home: &Path, day: &str) -> io::Result<Lock> {
    let pages = home.join("pages");
    private_dir(&pages)?;
    let lock = Lock::take(pages.join(format!("{day}.ruled.lock")))?;
    // Only behind the lock is the folder made the person's alone: a change refused for the lock changes nothing.
    own_dir(&pages)?;
    Ok(lock)
}

/// One change at a time on a day: a file only one caller can create, removed when the change is done.
pub(crate) struct Lock(PathBuf);

impl Lock {
    /// Waits [`LOCK_WAIT`], by the clock, for the name to be free. A lock that is stale is removed and taken. A
    /// folder at the name is never removed: the tap is refused after the same wait. On Windows a name another
    /// process is removing, or looking at, cannot be made for that moment and says "denied": that is a lock in the
    /// way like any other (`busy.rs`). Only when the wait ends with no lock there is the error itself the answer.
    fn take(path: PathBuf) -> io::Result<Lock> {
        let started = Instant::now();
        loop {
            let taken = OpenOptions::new().write(true).create_new(true).open(&path);
            let Err(e) = taken else { return Ok(Lock(path)) };
            let there = fs::symlink_metadata(&path);
            let used = in_use(&e);
            if e.kind() != ErrorKind::AlreadyExists && !there.as_ref().is_ok_and(fs::Metadata::is_dir) && !used {
                return Err(e);
            }
            if started.elapsed() >= LOCK_WAIT {
                let none = matches!(&there, Err(gone) if gone.kind() == ErrorKind::NotFound);
                let held = || io::Error::new(ErrorKind::TimedOut, "another tap on this day has not finished");
                return Err(if used && none { e } else { held() });
            }
            let age = |meta: fs::Metadata| meta.modified().ok().and_then(|at| at.elapsed().ok());
            let stale = match there {
                Ok(meta) if meta.is_dir() => false,
                Ok(meta) => age(meta).is_none_or(|age| age > LOCK_STALE),
                Err(_) => true,
            };
            if stale {
                let _ = fs::remove_file(&path);
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = retried(|| fs::remove_file(&self.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A clash with a statement that is empty once cleaned is no choice, whichever side it is on.
    #[test]
    fn two_statements_are_a_choice_only_when_neither_is_empty_and_they_differ() {
        let standing = |a: &str, b: &str| Standing { a: a.to_string(), b: b.to_string(), stands: None };
        assert!(standing("retry", "never retry").is_choice());
        for (a, b) in [("", "never retry"), ("retry", ""), ("", ""), ("retry", "retry")] {
            assert!(!standing(a, b).is_choice(), "{a:?} and {b:?}");
        }
    }
}
