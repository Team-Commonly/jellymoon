//! A picture is tried before it is trusted (`docs/adr/0016-a-picture-is-tried-before-it-is-trusted.md`). The
//! process that owns the window never reads a theme's picture with the XML or SVG parser and never rasterises
//! one, until another process has: the caller's own program, started again as a child that reads the picture,
//! holds it to the written rules, draws it by the page's call on a stack of the page's size, and measures what
//! that cost (`stand.rs`), under a limit of time. What the parser, the stroker or the rasteriser does by the size
//! of a value then costs one ended child at most.
//!
//! This file is the parent's half. It trusts the child for one short line and for ending well, and nothing else.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Mutex, PoisonError, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use super::{ArtCost, kept};

/// How long a trial may take, from the child's start to its end, and how long a child lives at most: it ends
/// itself at the limit when nobody else has. What a picture may cost is the meter's to say (`meter.rs`), in
/// processor time, which a busy machine does not stretch. The clock is for what the meter does not see: a child
/// that waits, or one that does not get its turn. So it is many times the processor time a picture may take, and
/// a program the system has not started before took 0.4 s to over 1 s to start in a release (`README.md`). A
/// trial the clock ends says nothing of the picture: it is not kept, and is tried again at the next start.
pub const TRIAL_LIMIT: Duration = Duration::from_millis(if cfg!(debug_assertions) { 20_000 } else { 4_000 });

/// The code a child ends with when its own clock ends it.
pub(super) const OVER: i32 = 124;

/// The most bytes of a child's answer that are read. The rest is never looked at.
const MOST_SAID: u64 = 1024;

/// What the child drew of a picture and what that cost: pixels, milliseconds of processor time and KiB of
/// memory. A child with a renderer that drew no pixel has not tried the picture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Drawn {
    pub pixels: u64,
    pub ms: u64,
    pub kib: u64,
}

/// What a child answers of a picture: what it costs by the rules and what was drawn of it, or why it is refused.
pub(super) type Answer = Result<(ArtCost, Drawn), String>;

/// What a child says: an answer about the picture, which holds whenever it is asked again and is kept, or why
/// it could not try it here and now, which is not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Said {
    Is(Answer),
    Unsure(String),
}

/// One trial at a time in a process, whichever thread asks: a theme's pictures are tried one after another.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

/// How to start a trial, as plain data from the caller: its own program, and the arguments by which that program
/// knows it is the child (it calls [`super::stand_trial`] with the first, before anything else).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trial {
    pub program: PathBuf,
    pub arguments: Vec<String>,
    /// The folder answers are kept in, so that a picture is tried once and not at every start. It is the
    /// person's alone (`kept.rs`). `None` keeps nothing.
    pub kept: Option<PathBuf>,
    /// What the caller is, as one line: its name and version. An answer is kept for the caller that asked, since
    /// another version of it may draw with another renderer.
    pub stamp: String,
    /// At most [`TRIAL_LIMIT`]: a longer one is that.
    pub limit: Duration,
}

impl Trial {
    /// A trial of [`TRIAL_LIMIT`]. A test's build (one with debug assertions, which a release is not) takes the
    /// milliseconds in `STICKER_KIT_TEST_TRIAL_MS` for the limit instead.
    pub fn new(program: PathBuf, argument: &str, kept: Option<PathBuf>, stamp: &str) -> Self {
        let short = std::env::var("STICKER_KIT_TEST_TRIAL_MS").ok().filter(|_| cfg!(debug_assertions));
        let limit = short.and_then(|ms| ms.parse().ok()).map_or(TRIAL_LIMIT, Duration::from_millis);
        Trial { program, arguments: vec![argument.to_string()], kept, stamp: stamp.to_string(), limit }
    }

    /// What the picture costs, when a trial has passed it, or the one line that says why it is refused. The
    /// answer kept for this very text is given with no child started; otherwise the caller waits here for the
    /// trial, up to the limit and for the trials before it. So it is not called on a window's thread.
    pub(super) fn judge(&self, svg: &str) -> Result<ArtCost, String> {
        let _one = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
        let key = key(&self.stamp);
        let folder = self.kept.as_deref().and_then(kept::ours);
        if let Some(answer) = folder.and_then(|folder| kept::read(folder, &key, svg)) {
            return answer.map(|(cost, _)| cost);
        }
        match self.tried(svg) {
            Said::Is(answer) => {
                if let Some(folder) = folder {
                    kept::write(folder, &key, svg, &answer);
                }
                answer.map(|(cost, _)| cost)
            }
            Said::Unsure(why) => Err(why),
        }
    }

    /// Starts the child, gives it the picture and waits for its end. Only what a child said of the picture and
    /// ended well after is an answer. A trial that could not start, was ended by a clock or by the system, or
    /// said anything else may go otherwise next time.
    fn tried(&self, svg: &str) -> Said {
        let limit = self.limit.min(TRIAL_LIMIT);
        let mut child = match self.command(limit).spawn() {
            Ok(child) => child,
            Err(error) => return Said::Unsure(format!("its trial could not be started: {}", error.kind())),
        };
        // A child that does not read must not hold the caller at a full pipe, nor one that never stops writing.
        if let Some(mut stdin) = child.stdin.take() {
            let text = svg.to_owned();
            thread::spawn(move || stdin.write_all(text.as_bytes()));
        }
        let (send, said) = mpsc::channel();
        if let Some(stdout) = child.stdout.take() {
            thread::spawn(move || {
                let mut said = Vec::new();
                let _ = stdout.take(MOST_SAID + 1).read_to_end(&mut said);
                send.send(said)
            });
        }
        let over = format!(
            "its trial took longer than {} ms and was ended, and is tried again at the next start",
            limit.as_millis()
        );
        // The child's own clock may end it a moment before this one would have.
        let status = ended_within(&mut child, limit).filter(|status| status.code() != Some(OVER));
        let Some(status) = status else { return Said::Unsure(over) };
        if !status.success() {
            return Said::Unsure(format!("its trial ended badly ({status})"));
        }
        // The child has ended, so its end of the pipe is shut and the line is here at once.
        let said = said.recv_timeout(Duration::from_secs(1)).ok().and_then(|said| String::from_utf8(said).ok());
        let said = said.and_then(|said| read_said(said.strip_suffix('\n')?));
        said.unwrap_or_else(|| Said::Unsure("its trial did not answer as a trial does".to_string()))
    }

    /// The child: the picture on its input, one line on its output, its limit for the last argument, and
    /// nothing else of the caller's that the system lets a process be started without.
    fn command(&self, limit: Duration) -> Command {
        let mut command = Command::new(&self.program);
        command.args(&self.arguments).arg(limit.as_millis().to_string()).env_clear();
        command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // `CREATE_NO_WINDOW`: no console window opens for the child. Windows loads its own libraries by
            // `SystemRoot`, so that one variable stays.
            command.creation_flags(0x0800_0000);
            if let Some(root) = std::env::var_os("SystemRoot") {
                command.env("SystemRoot", root);
            }
        }
        command
    }
}

/// What an answer is for besides the picture's own text: this kit, the parser and renderer it is built with
/// (`tests/alike.rs` holds the version named here to `Cargo.lock`), what the child did, the size it drew at,
/// which is the renderer's own two pixels a unit, the kit's rules, the build, whose costs are its own, and the
/// caller. A kit that changes a rule or a cost changes the number after `rules`.
pub(super) fn key(stamp: &str) -> String {
    let how = if cfg!(feature = "gpui") { "read and drawn at twice its size" } else { "read" };
    let build = if cfg!(debug_assertions) { "debug" } else { "release" };
    format!("sticker-kit {}, usvg {USVG}, {how}, rules 2, {build}, for {}", env!("CARGO_PKG_VERSION"), one_line(stamp))
}

/// The version of the parser and the renderer a kept answer is for.
pub const USVG: &str = "0.46.0";

/// The line a child says, and for an answer the line kept.
pub(super) fn write_said(said: &Said) -> String {
    match said {
        Said::Is(Ok((cost, drawn))) => {
            format!("accepted {} {} {} {} {}", cost.numbers, cost.paint, drawn.pixels, drawn.ms, drawn.kib)
        }
        Said::Is(Err(why)) => format!("refused: {}", one_line(why)),
        Said::Unsure(why) => format!("unsure: {}", one_line(why)),
    }
}

/// What a line says, or `None` for a line that is not exactly one of the three.
pub(super) fn read_said(line: &str) -> Option<Said> {
    let reason = |why: &str| (!why.is_empty() && why == one_line(why)).then(|| why.to_string());
    if let Some(why) = line.strip_prefix("refused: ") {
        return reason(why).map(|why| Said::Is(Err(why)));
    }
    if let Some(why) = line.strip_prefix("unsure: ") {
        return reason(why).map(Said::Unsure);
    }
    let digits = |text: &str| !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
    let written: Vec<&str> = line.strip_prefix("accepted ")?.split(' ').collect();
    let numbers: Vec<u64> = written.iter().filter(|text| digits(text)).flat_map(|number| number.parse()).collect();
    let [numbers, paint, pixels, ms, kib] = numbers[..] else { return None };
    // With a renderer, a picture is accepted for the pixels drawn of it.
    let tried = written.len() == 5 && (pixels > 0 || !cfg!(feature = "gpui"));
    let cost = ArtCost { numbers: usize::try_from(numbers).ok()?, paint };
    tried.then_some(Said::Is(Ok((cost, Drawn { pixels, ms, kib }))))
}

/// A reason as one line of at most 240 characters, none of them a control character: it is drawn and printed
/// as text, wherever it came from.
fn one_line(why: &str) -> String {
    why.chars().filter(|c| !c.is_control()).take(240).collect()
}

/// How the child ended, or `None` when the limit passed first. Either way the child has ended and been waited
/// for when this returns: nothing of a trial is left running.
fn ended_within(child: &mut Child, limit: Duration) -> Option<ExitStatus> {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) if started.elapsed() < limit => thread::sleep(Duration::from_millis(2)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ArtCost, Drawn, Said, one_line, read_said, write_said};

    #[test]
    fn a_child_says_one_of_three_lines_and_nothing_else_is_one() {
        let good = Said::Is(Ok((ArtCost { numbers: 12, paint: 3456 }, Drawn { pixels: 400, ms: 7, kib: 2048 })));
        assert_eq!(write_said(&good), "accepted 12 3456 400 7 2048");
        assert_eq!(read_said("accepted 12 3456 400 7 2048"), Some(good));
        assert_eq!(read_said("accepted 12 3456 0 7 2048").is_none(), cfg!(feature = "gpui"), "nothing was drawn");
        let refused = Said::Is(Err("it holds text or an image".into()));
        assert_eq!(read_said("refused: it holds text or an image"), Some(refused));
        assert_eq!(read_said("unsure: it could not be tried"), Some(Said::Unsure("it could not be tried".into())));
        assert_eq!(write_said(&Said::Is(Err("two\nlines\u{7}".into()))), "refused: twolines");
        assert_eq!(write_said(&Said::Unsure("two\nlines".into())), "unsure: twolines");
        let not = ["", "accepted", "accepted 1", "accepted 1 2 3", "accepted 1 2 3 4", "accepted 1 2 3 4 5 6"];
        let more = [
            "accepted +1 2 3 4 5",
            "accepted 1 -2 3 4 5",
            "accepted 1 2 3 4 5 ",
            "accepted 1 2 3 4 5\nmore",
            "accepted 1  2 3 4 5",
            "Accepted 1 2 3 4 5",
            "refused: ",
            "refused:x",
            "unsure: ",
            "unsure:x",
            "ok",
            "accepted 1e3 2 3 4 5",
        ];
        for line in not.into_iter().chain(more) {
            assert_eq!(read_said(line), None, "{line:?}");
        }
        assert_eq!(read_said(&format!("accepted 1 {}0 3 4 5", u64::MAX)), None, "a number too large is none");
        assert_eq!(read_said("refused: a\tb"), None, "a reason with a control character is not one");
        assert_eq!(one_line(&"x".repeat(300)).len(), 240);
    }
}
