//! The `jellymoon-app` command. With `layout` it prints where the kit puts every piece of a page, one line of JSON
//! each; without, it opens the page in a window. With `--page` both draw the page data they are given and nothing
//! else happens. With no `--page` the page is the day's own (`today.rs`): kept in the user's folder, and read through
//! core when the day has moved on.

use std::io::{Read, Write};
use std::path::Path;
use std::process::ExitCode;
use std::{fs, io};

use jellymoon_app::today::{self, Asked, Today};
use jellymoon_app::{lines, read, scene};
use lexopt::prelude::*;
use sticker_kit::{Theme, Viewport};

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
";

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
}

struct Options {
    page: Source,
    theme: Theme,
    size: Viewport,
}

/// Which page: a page-data file, a day, or the scene shown while a day is read.
enum Source {
    File(String),
    Day(Box<Today>),
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
            let scene = match &options.page {
                Source::File(file) => scene::scene(&page(file)?),
                Source::Day(day) => day.last(),
                Source::Waiting(date) => today::waiting(date),
            };
            Ok(lines::lines(&scene, &options.theme, options.size))
        }
        Command::Window(options) => window(options).map(|()| String::new()),
    }
}

/// Opens the window on the first scene there is. A day is then read off the window's thread, and what that gives
/// replaces the scene. `JELLYMOON_QUIT_AFTER=<seconds>` closes the window by itself, for checks.
#[cfg(feature = "window")]
fn window(options: Options) -> Result<(), Failure> {
    use jellymoon_app::window::{Later, open};
    let (scene, later): (_, Option<Later>) = match options.page {
        Source::File(file) => (scene::scene(&page(&file)?), None),
        Source::Waiting(date) => (today::waiting(&date), None),
        Source::Day(day) => {
            let (first, kept) = day.first();
            (first, Some(Box::new(move || day.read(kept.as_ref()))))
        }
    };
    let seconds = std::env::var("JELLYMOON_QUIT_AFTER").ok().and_then(|seconds| seconds.parse::<f32>().ok());
    let quit_after = seconds.and_then(|seconds| std::time::Duration::try_from_secs_f32(seconds).ok());
    open(scene, options.theme, options.size, quit_after, later);
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
    let (mut page, mut theme, mut size) = (None, String::from("sticker"), DEFAULT_SIZE);
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
                size = viewport(&given).ok_or_else(|| {
                    Failure::Usage(format!(
                        "--size must be <W>x<H> in pixels, from 960x640 to 16384x16384, not {given:?}"
                    ))
                })?;
            }
            other => return Err(usage(other.unexpected())),
        }
        first = false;
    }
    let page = match page {
        Some(file) => Source::File(file),
        None if waiting => {
            let date = today::date(asked.date.as_deref());
            Source::Waiting(date.ok_or_else(|| Failure::Usage("--date must be a day as YYYY-MM-DD".to_string()))?)
        }
        None => Source::Day(Box::new(Today::from_env(asked).map_err(Failure::Usage)?)),
    };
    let options = Options { page, theme: named(&theme)?, size };
    Ok(if layout { Command::Layout(options) } else { Command::Window(options) })
}

/// The most ignored keys of a theme file that are said, one a line; the rest are counted.
const MOST_WARNINGS: usize = 8;

/// The theme `--theme` names: a built-in one, or else a theme file. The keys the kit ignored are said on stderr.
fn named(theme: &str) -> Result<Theme, Failure> {
    if let Some(built_in) = Theme::builtin(theme) {
        return Ok(built_in);
    }
    let loaded = Theme::load(Path::new(theme)).map_err(|why| {
        let why = one_line(&why.to_string());
        Failure::Run(format!("--theme {theme:?} is not sticker, plain or a theme file the kit takes: {why}"))
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

/// The first and the last line of `text` as one line with no control character. A TOML error is several lines: where
/// it is, the text there, and what is wrong.
fn one_line(text: &str) -> String {
    let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
    let first = lines.next().unwrap_or_default();
    let line = lines.next_back().map_or(first.to_string(), |last| format!("{first}: {last}"));
    line.replace(char::is_control, " ")
}

/// `960x640` as a viewport. Each side is a whole number of pixels, from the smallest size a page is drawn at to
/// [`jellymoon_app::LARGEST_SIDE`].
fn viewport(given: &str) -> Option<Viewport> {
    let (w, h) = given.split_once('x')?;
    let side = |text: &str, least: f32| {
        text.parse::<u16>().ok().map(f32::from).filter(|side| (least..=jellymoon_app::LARGEST_SIDE).contains(side))
    };
    Some(Viewport { w: side(w, jellymoon_app::SMALLEST.w)?, h: side(h, jellymoon_app::SMALLEST.h)? })
}

/// The page the file holds, or what is on stdin when the file is `-`.
fn page(file: &str) -> Result<read::Page, Failure> {
    // One byte more than a page may be is enough to know it is too large, however much there is to read.
    let most = read::MAX_PAGE_BYTES as u64 + 1;
    let mut bytes = Vec::new();
    let got = if file == "-" {
        io::stdin().take(most).read_to_end(&mut bytes)
    } else {
        fs::File::open(file).and_then(|file| file.take(most).read_to_end(&mut bytes))
    };
    got.map_err(|e| Failure::Run(format!("reading {}: {e}", if file == "-" { "stdin" } else { file })))?;
    read::bytes(bytes).map_err(|why| Failure::Run(format!("the file is not page data: {why}")))
}
