//! The `jellymoon-app` command. With `layout` it prints where the kit puts every piece of a page, one line of JSON
//! each; without, it opens the page in a window. With `--page` both draw the page data they are given and nothing
//! else happens. With no `--page` the page is the day's own (`today.rs`): kept in the user's folder, and read through
//! core when the day has moved on. A tap in the window goes to core through `taps.rs`. `layout --jelly` prints the
//! jellyfish for one line of `jellymoon hear` (`jelly.rs`); the open app shows it in two windows of its own
//! (`shell.rs`), drawn once a second from what core keeps of the day.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{fs, io};

use jellymoon_app::today::{self, Asked, Today};
use jellymoon_app::{echoed, file_part, jelly, lines, one_line, read, scene, sides};
use lexopt::prelude::*;
use sticker_kit::{Theme, ThemeError, Viewport};

const USAGE: &str = "usage:
  jellymoon-app [layout] [--theme <theme>] [--size <W>x<H>] <which page>
      opens a window (t switches the theme); with `layout`, prints each placed piece instead
  a theme is sticker, plain, or a theme file
  which page:
    (nothing)                       today, kept in your jellymoon folder and read with your own Claude Code
    --date <YYYY-MM-DD>             another day
    --fresh                         read the day again even when its kept page is up to date
    --logs <dir> --skip <names> --model <name>    as for `jellymoon page`
    --waiting                       what is shown while a day is read
    --page <file, or - for stdin>   a page-data file: nothing is read and nothing is kept
  jellymoon-app layout --jelly <file, or - for stdin> --screen <W>x<H> [--theme <theme>] [--date <YYYY-MM-DD>]
      prints the jellyfish and its bubble for one line of `jellymoon hear`, and where both windows stand on a
      screen of that size; with --date that day's made wishes are read too, and no line is no hearing at all
";

/// What is said when `--jelly` or `--screen` is given with anything that is not theirs.
const JELLY_ALONE: &str =
    "--jelly goes with `layout` and --screen <W>x<H>, and with --theme and --date only: a page is another command";

/// The size a page is designed at, and the window's first size.
const DEFAULT_SIZE: Viewport = Viewport { w: 1440.0, h: 900.0 };

/// Why a command printed nothing: it was not a command (the usage follows the reason), or it could not be done.
enum Failure {
    Usage(String),
    Run(String),
}

enum Command {
    Help,
    Layout(Options),
    Window(Options),
    Jelly(Box<Jellyfish>),
}

struct Options {
    page: Source,
    theme: Theme,
    size: Viewport,
}

/// What `layout --jelly` was asked: where the line is, the screen, and the day whose made wishes are read, with
/// Jellymoon's home, when one was named.
struct Jellyfish {
    file: String,
    screen: Viewport,
    theme: Theme,
    day: Option<(PathBuf, String)>,
}

/// Which page: a page-data file, a day, or the scene shown while a day is read.
enum Source {
    File(String),
    /// A day, and whether it was named with `--date`: one that was not is today, which moves on.
    Day {
        day: Box<Today>,
        named: bool,
    },
    Waiting(String),
}

fn main() -> ExitCode {
    let failed = match run() {
        Ok(text) => {
            return if io::stdout().write_all(text.as_bytes()).is_ok() { ExitCode::SUCCESS } else { ExitCode::FAILURE };
        }
        Err(failed) => failed,
    };
    let (text, code) = match failed {
        Failure::Usage(why) => (format!("jellymoon-app: {why}\n{USAGE}"), 2),
        Failure::Run(why) => (format!("jellymoon-app: {why}\n"), 1),
    };
    let _ = io::stderr().write_all(text.as_bytes());
    ExitCode::from(code)
}

/// Runs the command and returns what it prints.
fn run() -> Result<String, Failure> {
    match parse(lexopt::Parser::from_env())? {
        Command::Help => Ok(USAGE.to_string()),
        Command::Layout(options) => {
            let (scene, entry) = match &options.page {
                Source::File(file) => (scene::scene(&page(file)?), None),
                Source::Day { day, .. } => {
                    let (scene, page) = day.last();
                    (day.with_work(day.with_stops(scene)), day.entry(page.as_ref()))
                }
                Source::Waiting(date) => (today::waiting(date), None),
            };
            Ok(lines::lines(&scene, entry.as_ref(), &options.theme, options.size))
        }
        Command::Window(options) => window(options).map(|()| String::new()),
        Command::Jelly(asked) => jellyfish(&asked),
    }
}

/// The jellyfish for the line given, placed on the screen. With a day, core reads that day's made wishes and says
/// which of the two is drawn; a line that is nothing is then no hearing at all. Any other input is refused.
fn jellyfish(asked: &Jellyfish) -> Result<String, Failure> {
    let bytes = input(&asked.file, jellymoon_core::HEAR_LINE_MAX as u64 + 1)?;
    let no_line = || Failure::Run("the input is not one line of `jellymoon hear`".to_string());
    let read = || jellymoon_core::Jelly::read(&bytes).ok_or_else(no_line);
    let shown = match &asked.day {
        Some((home, day)) => {
            let line = if bytes.trim_ascii().is_empty() { None } else { Some(read()?) };
            let drawn = jellymoon_core::jellyfish(line.as_ref(), home, Some(day));
            drawn.map_err(|why| Failure::Run(one_line(&why.to_string())))?
        }
        None => read()?,
    };
    Ok(jelly::lines(&shown.drawn(), &asked.theme, asked.screen))
}

/// Opens the page's window on the first scene there is, and the jellyfish. A day is then read off the window's
/// thread, and what that gives replaces the scene; once a second core is asked what the jellyfish shows of that
/// day, and of today when the day was not named. A page that is no day has a quiet jellyfish. `JELLYMOON_QUIT_AFTER=<seconds>` ends the app by itself, for
/// checks.
#[cfg(feature = "window")]
fn window(options: Options) -> Result<(), Failure> {
    use jellymoon_app::shell::Asked;
    use jellymoon_app::taps::OnShow;
    let (scene, day, named) = match options.page {
        Source::File(file) => (scene::scene(&page(&file)?), None, false),
        Source::Waiting(date) => (today::waiting(&date), None, false),
        Source::Day { day, named } => {
            let (first, kept) = day.first();
            let shown = OnShow { today: (*day).clone(), page: kept.clone(), scene: first.clone() };
            (first, Some((shown, day.later(kept))), named)
        }
    };
    let seconds = std::env::var("JELLYMOON_QUIT_AFTER").ok().and_then(|seconds| seconds.parse::<f32>().ok());
    let quit_after = seconds.and_then(|seconds| std::time::Duration::try_from_secs_f32(seconds).ok());
    let (theme, viewport) = (options.theme, options.size);
    // The jellyfish asks which day it is at each look (`shell.rs`).
    let today = std::sync::Arc::new(|| today::date(None));
    jellymoon_app::window::open(Asked { scene, theme, viewport, day, named, today, quit_after });
    Ok(())
}

#[cfg(not(feature = "window"))]
fn window(_: Options) -> Result<(), Failure> {
    Err(Failure::Run("this build has no window: it was built without the `window` feature".to_string()))
}

fn parse(mut parser: lexopt::Parser) -> Result<Command, Failure> {
    let usage = |e: lexopt::Error| Failure::Usage(e.to_string());
    let text = |parser: &mut lexopt::Parser| parser.value().and_then(|v| v.string()).map_err(usage);
    let (mut layout, mut first, mut waiting) = (false, true, false);
    let (mut page, mut theme, mut size) = (None, String::from("sticker"), None);
    let (mut line, mut screen) = (None, None);
    let mut asked = Asked::default();
    while let Some(arg) = parser.next().map_err(usage)? {
        match arg {
            Long("help") | Short('h') => return Ok(Command::Help),
            Value(command) if first && command == "help" => return Ok(Command::Help),
            Value(command) if first && command == "layout" => layout = true,
            Long("page") => page = Some(text(&mut parser)?),
            Long("date") => asked.date = Some(text(&mut parser)?),
            Long("logs") => asked.logs = Some(parser.value().map_err(usage)?.into()),
            Long("skip") => asked.skip = text(&mut parser)?,
            Long("model") => asked.model = Some(text(&mut parser)?),
            Long("fresh") => asked.fresh = true,
            Long("waiting") => waiting = true,
            Long("theme") => theme = text(&mut parser)?,
            Long("size") => {
                let given = text(&mut parser)?;
                let wrong = || {
                    let said = echoed(&given);
                    format!("--size must be <W>x<H> in pixels, from 960x640 to 16384x16384, not {said:?}")
                };
                size = Some(sides(&given, jellymoon_app::SMALLEST).ok_or_else(|| Failure::Usage(wrong()))?);
            }
            Long("jelly") => line = Some(text(&mut parser)?),
            Long("screen") => {
                let given = text(&mut parser)?;
                let wrong = || {
                    let said = echoed(&given);
                    format!("--screen must be <W>x<H> in pixels, each side from 1 to 16384, not {said:?}")
                };
                screen = Some(sides(&given, Viewport { w: 1.0, h: 1.0 }).ok_or_else(|| Failure::Usage(wrong()))?);
            }
            other => return Err(usage(other.unexpected())),
        }
        first = false;
    }
    if line.is_some() || screen.is_some() {
        let alone = layout && page.is_none() && !waiting && size.is_none() && asked.only_a_date();
        let (Some(file), Some(screen), true) = (line, screen, alone) else {
            return Err(Failure::Usage(JELLY_ALONE.to_string()));
        };
        let day = asked.date.map(|date| day_read(&date)).transpose()?;
        return Ok(Command::Jelly(Box::new(Jellyfish { file, screen, theme: named(&theme)?, day })));
    }
    let page = match page {
        Some(file) => Source::File(file),
        None if waiting => {
            let date = today::date(asked.date.as_deref());
            Source::Waiting(date.ok_or_else(|| Failure::Usage("--date must be a day as YYYY-MM-DD".to_string()))?)
        }
        None => day_open(asked)?,
    };
    let options = Options { page, theme: named(&theme)?, size: size.unwrap_or(DEFAULT_SIZE) };
    Ok(if layout { Command::Layout(options) } else { Command::Window(options) })
}

/// The day the app opens on, and whether `--date` named it.
fn day_open(asked: today::Asked) -> Result<Source, Failure> {
    let named = asked.date.is_some();
    Ok(Source::Day { day: Box::new(Today::from_env(asked).map_err(Failure::Usage)?), named })
}

/// The day `--date` names for a jellyfish, with the home its made wishes are read in.
fn day_read(date: &str) -> Result<(PathBuf, String), Failure> {
    let day =
        today::date(Some(date)).ok_or_else(|| Failure::Usage("--date must be a day as YYYY-MM-DD".to_string()))?;
    let home = jellymoon_core::own_folder().map_err(|why| Failure::Usage(why.to_string()))?;
    Ok((home.ok_or_else(|| Failure::Usage("no home directory: set JELLYMOON_HOME".to_string()))?, day))
}

/// The most ignored keys of a theme file that are said, one a line; the rest are counted.
const MOST_WARNINGS: usize = 8;

/// The theme `--theme` names: a built-in one, or else a theme file. The keys the kit ignored are said on stderr.
fn named(theme: &str) -> Result<Theme, Failure> {
    if let Some(built_in) = Theme::builtin(theme) {
        return Ok(built_in);
    }
    // A file is named by its last name, as any file that cannot be read is. A name that is no file may be a
    // theme's name mistyped, so what `--theme` takes is said before the words of `--page` and `--jelly`.
    let loaded = Theme::load(Path::new(theme)).map_err(|why| {
        Failure::Run(match why {
            ThemeError::NoFile => format!("--theme is sticker, plain or a theme file: {}", unread(theme)),
            why => {
                let (name, why) = (file_part(theme), one_line(&why.to_string()));
                format!("--theme {:?} is not sticker, plain or a theme file the kit takes: {why}", name.trim())
            }
        })
    })?;
    let mut said: Vec<String> = loaded.warnings.iter().take(MOST_WARNINGS).map(|warning| one_line(warning)).collect();
    if loaded.warnings.len() > MOST_WARNINGS {
        said.push(format!("+{} more", loaded.warnings.len() - MOST_WARNINGS));
    }
    for line in said {
        let _ = io::stderr().write_all(format!("jellymoon-app: theme: {line}\n").as_bytes());
    }
    Ok(loaded.theme)
}

/// The page the file holds, or what is on stdin when the file is `-`.
fn page(file: &str) -> Result<read::Page, Failure> {
    // One byte more than a page may be is enough to know it is too large, however much there is to read.
    let bytes = input(file, read::MAX_PAGE_BYTES as u64 + 1)?;
    read::bytes(bytes).map_err(|why| Failure::Run(format!("the file is not page data: {why}")))
}

/// At most `most` bytes of the file, or of stdin when the file is `-`.
fn input(file: &str, most: u64) -> Result<Vec<u8>, Failure> {
    let mut bytes = Vec::new();
    let got = if file == "-" {
        io::stdin().take(most).read_to_end(&mut bytes)
    } else {
        fs::File::open(file).and_then(|file| file.take(most).read_to_end(&mut bytes))
    };
    let why = |_| if file == "-" { "the input cannot be read".to_string() } else { unread(file) };
    got.map_err(|error| Failure::Run(why(error)))?;
    Ok(bytes)
}

/// What is said of a file that cannot be read, in fixed words, with the file's last name on one line when it has
/// one: no folder a person typed, and no word of the system's.
fn unread(file: &str) -> String {
    let name = file_part(file);
    let name = name.trim();
    if name.is_empty() { "the file cannot be read".to_string() } else { format!("the file cannot be read: {name}") }
}
