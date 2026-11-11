//! The reader: one local day of what a person typed to Claude Code, from the harness's session logs, with the agent's
//! last text reply to each turn (ADR 0011, section 1). It reads files and writes nothing.

use std::collections::HashSet;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{Timestamp, Zoned};

use crate::{Error, ReaderDay, session};

/// The longest log line kept, in bytes. A longer one is read past.
const LINE_MAX: usize = 64 << 20;

/// The most bytes of `skip.txt` that are read: far more than a list of folder names needs.
const MAX_SKIP_BYTES: usize = 64 * 1024;

/// One read of the session logs.
#[derive(Debug, Clone, Default)]
pub struct DayOptions {
    /// The Claude Code projects directory, laid out `<logs>/<project folder>/<session>.jsonl`.
    pub logs: PathBuf,
    /// The local calendar day, as YYYY-MM-DD. None is today.
    pub date: Option<String>,
    /// Names of the folders the person switched off. A turn whose folder contains one contributes nothing. Spaces
    /// around a name are dropped and an empty name is ignored.
    pub skip: Vec<String>,
    /// Jellymoon's own folder. Its `skip.txt`, one folder name a line, names more folders switched off. None reads no
    /// such file.
    pub home: Option<PathBuf>,
}

/// Reads every session log under `options.logs` and returns the turns typed on the day, oldest first. The day is a
/// calendar day in the computer's time zone, which the `TZ` variable overrides.
pub fn day(options: &DayOptions) -> Result<ReaderDay, Error> {
    read(options, &TimeZone::system(), LINE_MAX)
}

fn read(options: &DayOptions, zone: &TimeZone, line_max: usize) -> Result<ReaderDay, Error> {
    let date = match &options.date {
        Some(date) => parse(date)?,
        None => Zoned::now().with_time_zone(zone.clone()).date(),
    };
    let edge = |date: Date| date.to_zoned(zone.clone()).map(|start| start.timestamp()).map_err(|_| Error::Date);
    let (start, end) = (edge(date)?, edge(date.tomorrow().map_err(|_| Error::Date)?)?);
    let mut skip = options.skip.clone();
    if let Some(home) = &options.home {
        if !home.is_absolute() {
            return Err(Error::Home);
        }
        skip.extend(switched_off(home).map_err(Error::Skip)?);
    }
    match fs::metadata(&options.logs) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Err(Error::NoLogs(options.logs.display().to_string().replace(unseen, " ")));
        }
        found => found.map_err(Error::Logs)?,
    };
    let mut seen = HashSet::new();
    let mut turns = Vec::new();
    for file in session_files(&options.logs, start) {
        for (typed, mut turn) in session::turns(&file, &skip, &mut seen, line_max) {
            if let Some(typed) = typed.filter(|at| (start..end).contains(at)) {
                turn.at = typed.to_zoned(zone.clone()).strftime("%H:%M").to_string();
                turns.push((typed, turn));
            }
        }
    }
    turns.sort_by_key(|(typed, _)| *typed);
    Ok(ReaderDay { day: date.to_string(), turns: turns.into_iter().map(|(_, turn)| turn).collect() })
}

/// The folder names in `<home>/skip.txt`, one a line. Nothing at all under that name is no names. Anything else that
/// is not a whole, plain list is an error and never an empty list, since a folder the person switched off must not
/// reach the reading call: a link to nothing, a folder, a pipe, a file that cannot be read, is too large or is not
/// UTF-8, or a line with a character no folder name holds or nobody can see (which says the file is not the list it
/// looks like). An error names a line by its number, never by its text.
fn switched_off(home: &Path) -> Result<Vec<String>, String> {
    let path = home.join("skip.txt");
    match fs::symlink_metadata(&path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.to_string()),
        Ok(_) => {}
    }
    // Asked before it is opened, and through a link: opening a pipe would wait for a writer that never comes.
    if !fs::metadata(&path).map_err(|e| e.to_string())?.is_file() {
        return Err("it is not a file".to_string());
    }
    let mut bytes = Vec::new();
    let file = fs::File::open(&path).map_err(|e| e.to_string())?;
    file.take(MAX_SKIP_BYTES as u64 + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_SKIP_BYTES {
        return Err(format!("it is over {MAX_SKIP_BYTES} bytes"));
    }
    let text = String::from_utf8(bytes).map_err(|_| "it is not UTF-8 text".to_string())?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let odd = [
        ('\0', "a NUL character"),
        ('\u{feff}', "a byte order mark"),
        ('\r', "a carriage return that does not end it"),
    ];
    for (at, line) in text.lines().enumerate() {
        if let Some((_, what)) = odd.iter().find(|(odd, _)| line.contains(*odd)) {
            return Err(format!("line {} holds {what}", at + 1));
        }
        if line.contains(|c| c != '\t' && unseen(c)) {
            return Err(format!("line {} holds a character that cannot be seen", at + 1));
        }
    }
    Ok(text.lines().map(str::to_string).collect())
}

/// A character that draws nothing or moves what is drawn: a control character, or one of the invisible format
/// characters (soft hyphen, zero-width spaces and joiners, direction marks and overrides, the byte order mark). In a
/// line for the person it becomes a space; in a list name it could not be told from the name without it.
fn unseen(c: char) -> bool {
    let format = ['\u{ad}', '\u{61c}', '\u{180e}', '\u{feff}'];
    let ranges = ['\u{200b}'..='\u{200f}', '\u{202a}'..='\u{202e}', '\u{2060}'..='\u{2064}', '\u{2066}'..='\u{206f}'];
    c.is_control() || format.contains(&c) || ranges.iter().any(|range| range.contains(&c))
}

/// A calendar day written exactly YYYY-MM-DD.
fn parse(date: &str) -> Result<Date, Error> {
    let shaped = date.len() == 10
        && date.bytes().enumerate().all(
            |(at, byte)| {
                if at == 4 || at == 7 { byte == b'-' } else { byte.is_ascii_digit() }
            },
        );
    date.parse().ok().filter(|_| shaped).ok_or(Error::Date)
}

/// Lists the session logs, in name order: regular `.jsonl` files in real folders of `logs`, last written on or after the
/// day began (an older file holds none of it). Nothing is followed or opened that is not one: a link could lead out of
/// the logs, and a pipe would never end. An entry's type is read as it is, without following it.
fn session_files(logs: &Path, start: Timestamp) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for folder in entries(logs) {
        if !folder.file_type().is_ok_and(|kind| kind.is_dir()) {
            continue;
        }
        for file in entries(&folder.path()) {
            let fresh = |meta: fs::Metadata| {
                meta.is_file()
                    && meta.modified().ok().and_then(|at| Timestamp::try_from(at).ok()).is_some_and(|at| at >= start)
            };
            if file.file_name().to_string_lossy().ends_with(".jsonl") && file.metadata().is_ok_and(fresh) {
                files.push(file.path());
            }
        }
    }
    files
}

/// A folder's entries in name order; none when it cannot be listed.
fn entries(folder: &Path) -> Vec<fs::DirEntry> {
    let mut entries: Vec<fs::DirEntry> = fs::read_dir(folder).into_iter().flatten().flatten().collect();
    entries.sort_by_key(fs::DirEntry::file_name);
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(at: &str, text: &str) -> String {
        serde_json::json!({"type": "user", "timestamp": at, "cwd": "/work/alpha", "origin": {"kind": "human"}, "message": {"content": text}}).to_string()
    }

    fn read_utc(logs: &Path, lines: &[String], line_max: usize) -> Vec<String> {
        fs::create_dir_all(logs.join("-work-alpha")).unwrap();
        fs::write(logs.join("-work-alpha/s.jsonl"), lines.join("\n")).unwrap();
        let options =
            DayOptions { logs: logs.to_path_buf(), date: Some("2026-03-10".to_string()), ..DayOptions::default() };
        read(&options, &TimeZone::UTC, line_max).unwrap().turns.into_iter().map(|turn| turn.said).collect()
    }

    // Item 18. The cap is 64 MiB; here it is made small, so the test needs no giant file.
    #[test]
    fn a_line_over_the_cap_is_read_past_and_the_lines_after_it_are_still_read() {
        let logs = tempfile::tempdir().unwrap();
        let long = typed("2026-03-10T11:00:00Z", &"far too long ".repeat(400));
        let lines = [
            typed("2026-03-10T10:00:00Z", "before the long line"),
            long,
            typed("2026-03-10T12:00:00Z", "after the long line"),
        ];
        assert_eq!(read_utc(logs.path(), &lines, 2000), ["before the long line", "after the long line"]);
        assert_eq!(read_utc(logs.path(), &lines, LINE_MAX).len(), 3, "under the real cap all three are kept");
    }

    #[test]
    fn a_line_exactly_at_the_cap_is_kept_and_one_byte_over_is_not() {
        let logs = tempfile::tempdir().unwrap();
        let lines = [typed("2026-03-10T10:00:00Z", "at the cap"), typed("2026-03-10T11:00:00Z", "the end")];
        let cap = lines[0].len() + 1; // the line and its line break
        assert_eq!(read_utc(logs.path(), &lines, cap), ["at the cap", "the end"]);
        assert_eq!(read_utc(logs.path(), &lines, cap - 1), ["the end"]);
    }

    #[test]
    fn a_line_that_is_not_utf8_is_read_with_the_bad_bytes_replaced() {
        let logs = tempfile::tempdir().unwrap();
        fs::create_dir_all(logs.path().join("-work-alpha")).unwrap();
        let line = typed("2026-03-10T10:00:00Z", "caf? au lait");
        let line: Vec<u8> = line.bytes().map(|byte| if byte == b'?' { 0xE9 } else { byte }).collect(); // Latin-1's é
        fs::write(logs.path().join("-work-alpha/s.jsonl"), line).unwrap();
        let options = DayOptions {
            logs: logs.path().to_path_buf(),
            date: Some("2026-03-10".to_string()),
            ..DayOptions::default()
        };
        assert_eq!(read(&options, &TimeZone::UTC, LINE_MAX).unwrap().turns[0].said, "caf\u{fffd} au lait");
    }

    #[test]
    fn only_a_real_calendar_day_written_in_full_is_a_date() {
        assert!(parse("2026-03-10").is_ok());
        for date in ["2026-3-10", "2026-02-30", "10-03-2026", "2026-03-10T00:00", " 2026-03-10", "tomorrow", ""] {
            assert!(parse(date).is_err(), "{date:?}");
        }
    }
}
