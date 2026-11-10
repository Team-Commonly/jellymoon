//! The reader: one local day of what a person typed to Claude Code, from the harness's session logs, with the agent's
//! last text reply to each turn (ADR 0011, section 1). It reads files and writes nothing.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{Timestamp, Zoned};

use crate::{Error, ReaderDay, session};

/// The longest log line kept, in bytes. A longer one is read past.
const LINE_MAX: usize = 64 << 20;

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
    fs::metadata(&options.logs).map_err(Error::Logs)?;
    let mut seen = HashSet::new();
    let mut turns = Vec::new();
    for file in session_files(&options.logs, start) {
        for (typed, mut turn) in session::turns(&file, &options.skip, &mut seen, line_max) {
            if let Some(typed) = typed.filter(|at| (start..end).contains(at)) {
                turn.at = typed.to_zoned(zone.clone()).strftime("%H:%M").to_string();
                turns.push((typed, turn));
            }
        }
    }
    turns.sort_by_key(|(typed, _)| *typed);
    Ok(ReaderDay { day: date.to_string(), turns: turns.into_iter().map(|(_, turn)| turn).collect() })
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
        let options = DayOptions { logs: logs.to_path_buf(), date: Some("2026-03-10".to_string()), skip: Vec::new() };
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
        let options =
            DayOptions { logs: logs.path().to_path_buf(), date: Some("2026-03-10".to_string()), skip: Vec::new() };
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
