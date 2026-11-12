//! The child's half of a trial (`trial.rs`): the program a trial starts is the caller's own, and it calls
//! [`stand_trial`] before it does anything else. Here, and nowhere in the process that owns a window, a picture
//! nobody has judged is read with the XML and SVG parsers, held to the written rules (`svg_check.rs`) and drawn by
//! the renderer as the page will draw it. Whatever that does to this process, the parent only counts the time.

use std::ffi::OsStr;
use std::io::{self, Read, Write};

use super::art_cost;
use super::bounds::MAX_ART_BYTES;
use super::trial::{Answer, write_answer};

/// The most memory a trial may map, where the system has a cheap limit a process can put on itself: Linux. The
/// renderer's largest picture is 512 by 512 pixels, a megabyte.
#[cfg(target_os = "linux")]
const MOST_MEMORY: u64 = 2 * 1024 * 1024 * 1024;

/// Makes this process a trial and ends it, when its first argument is `selects`; returns at once when it is not.
/// The picture is read from the standard input, and the answer is one line on the standard output:
/// `accepted <numbers> <paint> <pixels drawn>`, or `refused: <why>`. A program calls this first thing in `main`.
pub fn stand_trial(selects: &str) {
    if std::env::args_os().nth(1).as_deref() != Some(OsStr::new(selects)) {
        return;
    }
    let said = format!("{}\n", write_answer(&answer()));
    let mut out = io::stdout();
    let told = out.write_all(said.as_bytes()).and_then(|()| out.flush());
    std::process::exit(if told.is_ok() { 0 } else { 1 });
}

fn answer() -> Answer {
    hold_memory()?;
    let mut text = String::new();
    io::stdin().take(MAX_ART_BYTES + 1).read_to_string(&mut text).map_err(|_| "it is not text".to_string())?;
    if text.len() as u64 > MAX_ART_BYTES {
        return Err(format!("it is over {} KiB", MAX_ART_BYTES / 1024));
    }
    let cost = art_cost(&text)?;
    Ok((cost, drawn(&text)?))
}

/// Draws the picture with the renderer the page draws with, by the very call GPUI makes for an SVG image: at
/// twice the picture's size. Returns how many pixels that was.
#[cfg(feature = "gpui")]
fn drawn(svg: &str) -> Result<u64, String> {
    let renderer = gpui::SvgRenderer::new(std::sync::Arc::new(()));
    let image = renderer.render_single_frame(svg.as_bytes(), 1.0);
    let size = image.map_err(|e| format!("the renderer does not draw it: {e}"))?.size(0);
    Ok(u64::from(size.width.0.unsigned_abs()) * u64::from(size.height.0.unsigned_abs()))
}

/// Without the drawing half there is no renderer, and no page is drawn either: the picture is read, and the
/// answer is kept apart from a drawn one (`trial.rs`, `key`).
#[cfg(not(feature = "gpui"))]
fn drawn(_: &str) -> Result<u64, String> {
    Ok(0)
}

#[cfg(target_os = "linux")]
fn hold_memory() -> Result<(), String> {
    use rustix::process::{Resource, Rlimit, setrlimit};
    let most = Rlimit { current: Some(MOST_MEMORY), maximum: Some(MOST_MEMORY) };
    setrlimit(Resource::As, most).map_err(|e| format!("its trial could not limit its memory: {e}"))
}

/// macOS does not enforce a limit on the memory a process maps, and Windows has none short of a job object:
/// there the limit of time is what bounds a trial's memory.
#[cfg(not(target_os = "linux"))]
fn hold_memory() -> Result<(), String> {
    Ok(())
}
