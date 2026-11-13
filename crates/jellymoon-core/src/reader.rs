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
    let (date, turns) = typed_on(options, zone, line_max)?;
    Ok(ReaderDay { day: date.to_string(), turns: turns.into_iter().map(|(_, turn, _)| turn).collect() })
}

/// The folders of one name that something was typed in on the day, for a run to find its wish's repo (F39). They are
/// the log's own text: what is done with one is up to the caller, and none is ever printed.
pub(crate) fn folders(options: &DayOptions, name: &str) -> Result<Vec<PathBuf>, Error> {
    Ok(typed_in(options)?.into_iter().filter(|folder| is_named(folder, name)).collect())
}

/// Whether a folder's last name is `name`: the one way a folder is matched to a project's name.
pub(crate) fn is_named(folder: &Path, name: &str) -> bool {
    folder.file_name().is_some_and(|last| last == name)
}

/// Every folder something was typed in on the day, each once: the ones no name switches off, as the reader
/// switches a folder off, by its whole path.
pub(crate) fn typed_in(options: &DayOptions) -> Result<Vec<PathBuf>, Error> {
    let mut found: Vec<PathBuf> = Vec::new();
    for (_, _, folder) in typed_on(options, &TimeZone::system(), LINE_MAX)?.1 {
        let folder = PathBuf::from(folder).components().collect::<PathBuf>();
        if folder.is_absolute() && folder.as_os_str().len() <= session::FOLDER_MAX && !found.contains(&folder) {
            found.push(folder);
        }
    }
    Ok(found)
}

/// The day, and every turn typed on it, oldest first.
fn typed_on(options: &DayOptions, zone: &TimeZone, line_max: usize) -> Result<(Date, Vec<session::Typed>), Error> {
    let date = match &options.date {
        Some(date) => parse(date)?,
        None => Zoned::now().with_time_zone(zone.clone()).date(),
    };
    let edge = |date: Date| date.to_zoned(zone.clone()).map(|start| start.timestamp()).map_err(|_| Error::Date);
    let (start, end) = (edge(date)?, edge(date.tomorrow().map_err(|_| Error::Date)?)?);
    let mut left = session::Left { skip: options.skip.clone(), ..Default::default() };
    if let Some(home) = &options.home {
        if !home.is_absolute() {
            return Err(Error::Home);
        }
        left.skip.extend(switched_off(home).map_err(Error::Skip)?);
        left.copies = session::Copies::of(home);
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
        for (typed, mut turn, folder) in session::turns(&file, &left, &mut seen, line_max) {
            if let Some(at) = typed.filter(|at| (start..end).contains(at)) {
                turn.at = at.to_zoned(zone.clone()).strftime("%H:%M").to_string();
                turns.push((typed, turn, folder));
            }
        }
    }
    turns.sort_by_key(|(typed, ..)| *typed);
    Ok((date, turns))
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
        if hidden(line) {
            return Err(format!("line {} holds a character that cannot be seen; {WAY_OUT}", at + 1));
        }
    }
    Ok(text.lines().map(str::to_string).collect())
}

/// A character that draws nothing or moves what is drawn: a control character, or one of this closed list. The soft
/// hyphen, the Arabic letter mark, the Hangul fillers, the Mongolian vowel separator, the line and paragraph
/// separators, the byte order mark; the zero-width spaces, joiners and direction marks, the direction overrides, the
/// invisible operators and isolates, the variation selectors and the tag characters. In a line for the person it
/// becomes a space; in a name of a folder it could not be told from the name without it.
fn unseen(c: char) -> bool {
    let single = "\u{ad}\u{61c}\u{115f}\u{1160}\u{180e}\u{2028}\u{2029}\u{3164}\u{feff}\u{ffa0}";
    let ranges = [
        '\u{200b}'..='\u{200f}',
        '\u{202a}'..='\u{202e}',
        '\u{2060}'..='\u{206f}',
        '\u{fe00}'..='\u{fe0f}',
        '\u{e0000}'..='\u{e007f}',
        '\u{e0100}'..='\u{e01ef}',
    ];
    c.is_control() || single.contains(c) || ranges.iter().any(|range| range.contains(&c))
}

/// What to do about a name with such a character: the rest of the folder's name still matches it.
const WAY_OUT: &str = "name a part of the folder's name without it";

/// Whether a name holds a character that cannot be seen, once the white space around it is dropped as the match
/// drops it.
fn hidden(name: &str) -> bool {
    name.trim().contains(unseen)
}

/// The names in a `--skip` value, which commas part. A name with a character that cannot be seen would match nothing
/// and look right, so the value is refused: the error names the name by its place, never by its text.
pub fn skip_names(value: &str) -> Result<Vec<String>, String> {
    if value.is_empty() {
        return Ok(Vec::new()); // no name at all, and not one empty name
    }
    let names: Vec<String> = value.split(',').map(str::to_string).collect();
    match names.iter().position(|name| hidden(name)) {
        Some(at) => Err(format!("--skip: name {} holds a character that cannot be seen; {WAY_OUT}", at + 1)),
        None => Ok(names),
    }
}

/// A calendar day written exactly YYYY-MM-DD.
pub(crate) fn parse(date: &str) -> Result<Date, Error> {
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

    // The table, written out a second time: every character of every range, not only its ends, and each single one.
    #[test]
    fn every_character_of_the_table_is_refused_inside_a_name_in_the_flag_and_in_the_list() {
        let ranges = [0x0..=0x1f, 0x7f..=0x9f, 0x200b..=0x200f, 0x202a..=0x202e, 0x2060..=0x206f, 0xfe00..=0xfe0f];
        let (tags, more) = (0xe0000..=0xe007f, 0xe0100..=0xe01ef);
        let single = [0xad, 0x61c, 0x115f, 0x1160, 0x180e, 0x2028, 0x2029, 0x3164, 0xfeff, 0xffa0];
        let table: Vec<char> =
            ranges.into_iter().chain([tags, more]).flatten().chain(single).filter_map(char::from_u32).collect();
        assert_eq!(table.len(), 32 + 33 + 5 + 5 + 16 + 16 + 128 + 240 + 10);
        let home = tempfile::tempdir().unwrap();
        for c in table {
            let flag = skip_names(&format!("alpha,be{c}ta")).unwrap_err();
            assert!(flag.starts_with("--skip: name 2 holds a character that cannot be seen"), "{c:?}: {flag}");
            if c == '\n' {
                continue; // in the list a line break parts two names
            }
            fs::write(home.path().join("skip.txt"), format!("alpha\nbe{c}ta\n")).unwrap();
            let list = switched_off(home.path()).unwrap_err();
            assert!(list.starts_with("line 2 holds a "), "{c:?}: {list}");
        }
        // The neighbours of each range are ordinary characters, and so is white space at a name's edge.
        let seen = "\u{a0}\u{ae}\u{200a}\u{2010}\u{202f}\u{205f}\u{2070}\u{fdff}\u{fe10}\u{e0080}\u{e00ff}\u{e01f0}";
        assert_eq!(skip_names(&format!(" al{seen}pha\t, beta ")).unwrap().len(), 2);
    }

    // What a run finds its wish's repo by, and a wish is offered for (`wish::wishable`).
    #[test]
    fn a_folder_in_a_run_s_copy_is_not_one_something_was_typed_in() {
        let dir = tempfile::tempdir().unwrap();
        let (home, logs) = (dir.path().join("jm"), dir.path().join("logs"));
        let (copies, kept) = (home.join("copies"), home.join("kept").join("beta"));
        let typed = [dir.path().join("alpha"), copies.join("r1"), copies.join("r2").join("beta"), kept];
        let noon = jiff::civil::date(2026, 3, 10).at(12, 0, 0, 0).to_zoned(TimeZone::system()).unwrap();
        let line = |folder: &PathBuf| {
            serde_json::json!({"type": "user", "timestamp": noon.timestamp(), "cwd": folder,
                "origin": {"kind": "human"}, "message": {"content": "hello"}})
            .to_string()
        };
        fs::create_dir_all(logs.join("p")).unwrap();
        fs::write(logs.join("p").join("s.jsonl"), typed.iter().map(line).collect::<Vec<_>>().join("\n")).unwrap();
        let date = Some("2026-03-10".to_string());
        let read = DayOptions { logs, date, skip: Vec::new(), home: Some(home) };
        assert_eq!(typed_in(&read).unwrap(), [typed[0].clone(), typed[3].clone()]);
        assert_eq!(folders(&read, "beta").unwrap(), [typed[3].clone()]);
        let names = ["alpha", "r1", "beta", "r2"].map(str::to_string);
        assert_eq!(crate::wishable(&read, &names), ["alpha", "beta"]);
        // With no home there are no copies to know of.
        assert_eq!(typed_in(&DayOptions { home: None, ..read }).unwrap().len(), 4);
    }

    #[test]
    fn only_a_real_calendar_day_written_in_full_is_a_date() {
        assert!(parse("2026-03-10").is_ok());
        for date in ["2026-3-10", "2026-02-30", "10-03-2026", "2026-03-10T00:00", " 2026-03-10", "tomorrow", ""] {
            assert!(parse(date).is_err(), "{date:?}");
        }
    }
}
