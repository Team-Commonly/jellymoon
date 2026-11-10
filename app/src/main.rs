//! The `jellymoon-app` command. With `layout` it prints where the kit puts every piece of a page, one line of JSON
//! each; without, it opens the page in a window. Both draw page data they are given: no reading call, no network and
//! no file written.

use std::io::{Read, Write};
use std::process::ExitCode;
use std::{fs, io};

use jellymoon_app::{lines, read, scene};
use lexopt::prelude::*;
use sticker_kit::{Theme, Viewport};

const USAGE: &str = "usage:
  jellymoon-app layout --page <file, or - for stdin> [--theme sticker|plain] [--size <W>x<H>]   (prints each placed piece)
  jellymoon-app --page <file, or - for stdin> [--theme sticker|plain] [--size <W>x<H>]          (opens a window; t switches the theme)
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
    page: String,
    theme: Theme,
    size: Viewport,
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
            Ok(lines::lines(&scene::scene(&page(&options.page)?), &options.theme, options.size))
        }
        Command::Window(options) => {
            let scene = scene::scene(&page(&options.page)?);
            window(scene, options).map(|()| String::new())
        }
    }
}

/// Opens the window. `JELLYMOON_QUIT_AFTER=<seconds>` closes it by itself, for checks.
#[cfg(feature = "window")]
fn window(scene: sticker_kit::Scene, options: Options) -> Result<(), Failure> {
    let seconds = std::env::var("JELLYMOON_QUIT_AFTER").ok().and_then(|seconds| seconds.parse::<f32>().ok());
    let quit_after = seconds.and_then(|seconds| std::time::Duration::try_from_secs_f32(seconds).ok());
    jellymoon_app::window::open(scene, options.theme, options.size, quit_after);
    Ok(())
}

#[cfg(not(feature = "window"))]
fn window(_: sticker_kit::Scene, _: Options) -> Result<(), Failure> {
    Err(Failure::Run("this build has no window: it was built without the `window` feature".to_string()))
}

fn parse(mut parser: lexopt::Parser) -> Result<Command, Failure> {
    let usage = |e: lexopt::Error| Failure::Usage(e.to_string());
    let (mut layout, mut first) = (false, true);
    let (mut page, mut theme, mut size) = (None, Theme::sticker(), DEFAULT_SIZE);
    while let Some(arg) = parser.next().map_err(usage)? {
        match arg {
            Long("help") | Short('h') => return Ok(Command::Help),
            Value(command) if first && command == "help" => return Ok(Command::Help),
            Value(command) if first && command == "layout" => layout = true,
            Long("page") => page = Some(parser.value().and_then(|v| v.string()).map_err(usage)?),
            Long("theme") => {
                let name = parser.value().and_then(|v| v.string()).map_err(usage)?;
                theme = Theme::builtin(&name)
                    .ok_or_else(|| Failure::Usage(format!("--theme must be sticker or plain, not {name:?}")))?;
            }
            Long("size") => {
                let given = parser.value().and_then(|v| v.string()).map_err(usage)?;
                size = viewport(&given).ok_or_else(|| {
                    Failure::Usage(format!("--size must be <W>x<H> in pixels, like 960x640, not {given:?}"))
                })?;
            }
            other => return Err(usage(other.unexpected())),
        }
        first = false;
    }
    let page = page.ok_or_else(|| Failure::Usage("no page: name a page-data file with --page".to_string()))?;
    let options = Options { page, theme, size };
    Ok(if layout { Command::Layout(options) } else { Command::Window(options) })
}

/// `960x640` as a viewport. Each side is a whole number of pixels from 1 to 16384.
fn viewport(given: &str) -> Option<Viewport> {
    let (w, h) = given.split_once('x')?;
    let side = |text: &str| text.parse::<u16>().ok().filter(|side| (1..=16384).contains(side)).map(f32::from);
    Some(Viewport { w: side(w)?, h: side(h)? })
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
