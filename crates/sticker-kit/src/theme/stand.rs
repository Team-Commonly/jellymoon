//! The child's half of a trial (`trial.rs`): the program a trial starts is the caller's own, and it calls
//! [`stand_trial`] before it does anything else. Here, and nowhere in the process that owns a window, a picture
//! nobody has judged is read with the XML and SVG parsers, held to the written rules (`svg_check.rs`) and drawn by
//! the renderer, by the call the page makes and on a stack no larger than the page's. What that costs is measured
//! (`meter.rs`). Whatever it does to this process, the parent only reads one line and how the process ended.
//!
//! The child has a clock of its own, started before it reads a byte: it ends itself at the limit whoever is or is
//! not watching, so a child whose caller is gone lives at most the limit.

use std::ffi::OsStr;
use std::io::{self, Read, Write};
use std::sync::{Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use super::art_cost;
use super::bounds::MAX_ART_BYTES;
use super::meter::Meter;
use super::trial::{Drawn, OVER, Said, TRIAL_LIMIT, write_said};

/// The stack a picture is read and drawn on in the child: the smallest any system gives the thread that draws a
/// page's pictures, so that what would end the page ends the child. GPUI rasterises a picture on its background
/// executor. On macOS that is a queue of the system's, whose threads have 512 KiB; on Linux it is threads the
/// standard library starts, of 2 MiB; on Windows it is the system's thread pool, of the program's own 1 MiB.
pub const PAGE_STACK: usize = 512 * 1024;

/// The most memory a trial may map, where the system has a cheap limit a process can put on itself: Linux. The
/// meter's count is of what the child holds and is asked now and then; this is what the system itself refuses.
#[cfg(target_os = "linux")]
const MOST_MAPPED: u64 = 2 * 1024 * 1024 * 1024;

/// Makes this process a trial and ends it, when its first argument is `selects`; returns at once when it is not.
/// The picture is read from the standard input, and the answer is one line on the standard output:
/// `accepted <numbers> <paint> <pixels drawn> <ms of processor time> <KiB of memory>`, `refused: <why>`, or
/// `unsure: <why>` when the child could not try the picture. The last argument is the limit in milliseconds, up
/// to [`TRIAL_LIMIT`], which it also is without one. A program calls this first thing in `main`.
pub fn stand_trial(selects: &str) {
    if std::env::args_os().nth(1).as_deref() != Some(OsStr::new(selects)) {
        return;
    }
    let last = std::env::args_os().next_back();
    let given = last.and_then(|last| last.to_str()?.parse().ok()).map(Duration::from_millis);
    let limit = given.map_or(TRIAL_LIMIT, |given| given.min(TRIAL_LIMIT));
    let clock = thread::Builder::new().spawn(move || {
        thread::sleep(limit);
        std::process::exit(OVER);
    });
    if clock.is_err() {
        say(&Said::Unsure("its trial could not start its own clock".to_string()));
    }
    match thread::Builder::new().stack_size(PAGE_STACK).spawn(answer).map(thread::JoinHandle::join) {
        Ok(Ok(said)) => say(&said),
        // What panicked has said why on the standard error, which nobody reads.
        Ok(Err(_)) => std::process::exit(101),
        Err(_) => say(&Said::Unsure("its trial could not start a thread to draw on".to_string())),
    }
}

/// Says the one line and ends the process. Whichever thread comes first says it; any other waits here for the
/// end.
fn say(said: &Said) -> ! {
    static ONCE: Mutex<()> = Mutex::new(());
    let _once = ONCE.lock().unwrap_or_else(PoisonError::into_inner);
    let mut out = io::stdout();
    let told = out.write_all(format!("{}\n", write_said(said)).as_bytes()).and_then(|()| out.flush());
    std::process::exit(if told.is_ok() { 0 } else { 1 });
}

fn answer() -> Said {
    if let Err(why) = hold_memory() {
        return Said::Unsure(why);
    }
    let mut text = String::new();
    if io::stdin().take(MAX_ART_BYTES + 1).read_to_string(&mut text).is_err() {
        return Said::Is(Err("it is not text".to_string()));
    }
    if text.len() as u64 > MAX_ART_BYTES {
        return Said::Is(Err(format!("it is over {} KiB", MAX_ART_BYTES / 1024)));
    }
    // The bytes are here and no parser has read one: what the picture costs is counted from now.
    let mut meter = match Meter::start(say) {
        Ok(meter) => meter,
        Err(why) => return Said::Unsure(why),
    };
    let tried = art_cost(&text).and_then(|cost| Ok((cost, drawn(&text)?)));
    let (cost, pixels) = match tried {
        Ok(tried) => tried,
        Err(why) => return Said::Is(Err(why)),
    };
    match meter.cost() {
        Ok((ms, kib)) => Said::Is(Ok((cost, Drawn { pixels, ms, kib }))),
        Err(over) => over,
    }
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
    let most = Rlimit { current: Some(MOST_MAPPED), maximum: Some(MOST_MAPPED) };
    setrlimit(Resource::As, most).map_err(|e| format!("its trial could not limit its memory: {e}"))
}

/// macOS does not enforce a limit on the memory a process maps, and Windows has none short of a job object:
/// there the meter's count is what bounds a trial's memory.
#[cfg(not(target_os = "linux"))]
fn hold_memory() -> Result<(), String> {
    Ok(())
}
