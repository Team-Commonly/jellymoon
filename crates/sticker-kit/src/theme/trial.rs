//! A picture is tried before it is trusted (`docs/adr/0016-a-picture-is-tried-before-it-is-trusted.md`). The
//! process that owns the window never reads a theme's picture with the XML or SVG parser and never rasterises
//! one, until another process has: the caller's own program, started again as a child that reads the picture,
//! holds it to the written rules and draws it as the page will (`stand.rs`), under a limit of time. What the
//! parser, the stroker or the rasteriser does by the size of a value then costs one ended child at most.
//!
//! This file is the parent's half. It trusts the child for one short line and for ending well, and nothing else.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Mutex, PoisonError, mpsc};
use std::time::{Duration, Instant};
use std::{fs, thread};

use super::ArtCost;
use super::bounds::MAX_ART_BYTES;

/// How long a trial may take, from the child's start to its end. Measured on one busy Mac (`README.md`): the
/// heaviest picture known inside the rules is tried in about 0.11 s in a release and 1.4 s in a debug build, the
/// child's start included, so the limit is some 15 times that. A program the system has not started before took
/// 0.4 s to over 1 s to start in a release, which is why the limit is not nearer.
pub const TRIAL_LIMIT: Duration = Duration::from_millis(if cfg!(debug_assertions) { 20_000 } else { 2_000 });

/// The most bytes of a child's answer that are read. The rest is never looked at.
const MOST_SAID: u64 = 1024;

/// The most kept answers in a folder. One more empties it first, so the folder is at most 64 pictures' size.
const MOST_KEPT: usize = 64;

/// The ending of a kept answer's file name.
const KEPT: &str = "trial";

/// What a child answers: what the picture costs by the rules and how many pixels it drew of it, or why it is
/// refused. A child with a renderer that drew no pixel has not tried the picture.
pub(super) type Answer = Result<(ArtCost, u64), String>;

/// One trial at a time in a process, whichever thread asks: a theme's pictures are tried one after another.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

/// How to start a trial, as plain data from the caller: its own program, and the arguments by which that program
/// knows it is the child (it calls [`super::stand_trial`] with the first, before anything else).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trial {
    pub program: PathBuf,
    pub arguments: Vec<String>,
    /// The folder answers are kept in, so that a picture is tried once and not at every start. Whoever can write
    /// here can write the theme too. `None` keeps nothing.
    pub kept: Option<PathBuf>,
    pub limit: Duration,
}

impl Trial {
    /// A trial of [`TRIAL_LIMIT`]. A test's build (one with debug assertions, which a release is not) takes the
    /// milliseconds in `STICKER_KIT_TEST_TRIAL_MS` for the limit instead.
    pub fn new(program: PathBuf, argument: &str, kept: Option<PathBuf>) -> Self {
        let short = std::env::var("STICKER_KIT_TEST_TRIAL_MS").ok().filter(|_| cfg!(debug_assertions));
        let limit = short.and_then(|ms| ms.parse().ok()).map_or(TRIAL_LIMIT, Duration::from_millis);
        Trial { program, arguments: vec![argument.to_string()], kept, limit }
    }

    /// What the picture costs, when a trial has passed it, or the one line that says why it is refused. The
    /// answer kept for this very text is given with no child started; otherwise the caller waits here for the
    /// trial, up to the limit and for the trials before it. So it is not called on a window's thread.
    pub(super) fn judge(&self, svg: &str) -> Result<ArtCost, String> {
        let _one = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(kept) = self.kept_answer(svg) {
            return kept.map(|(cost, _)| cost);
        }
        let (answer, lasting) = self.tried(svg);
        if lasting {
            self.keep(svg, &answer);
        }
        answer.map(|(cost, _)| cost)
    }

    /// Starts the child, gives it the picture and waits for its end. The second value is whether the answer is
    /// one to keep: only what a child said and ended well after is. A trial that could not start, ran over on a
    /// busy machine or was ended by the system may go otherwise next time.
    fn tried(&self, svg: &str) -> (Answer, bool) {
        let mut child = match self.command().spawn() {
            Ok(child) => child,
            Err(error) => return (Err(format!("its trial could not be started: {}", error.kind())), false),
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
        let Some(status) = ended_within(&mut child, self.limit) else {
            return (Err(format!("its trial took longer than {} ms and was ended", self.limit.as_millis())), false);
        };
        if !status.success() {
            return (Err(format!("its trial ended badly ({status})")), false);
        }
        // The child has ended, so its end of the pipe is shut and the line is here at once.
        let said = said.recv_timeout(Duration::from_secs(1)).unwrap_or_default();
        let answer = String::from_utf8(said).ok().and_then(|said| read_answer(said.strip_suffix('\n')?));
        (answer.unwrap_or_else(|| Err("its trial did not answer as a trial does".to_string())), true)
    }

    /// The child: the picture on its input, one line on its output, and nothing else of the caller's that the
    /// system lets a process be started without.
    fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command.args(&self.arguments).env_clear();
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

    fn kept_file(&self, svg: &str) -> Option<PathBuf> {
        let mut hasher = DefaultHasher::new();
        (key(), svg).hash(&mut hasher);
        Some(self.kept.as_ref()?.join(format!("{:016x}.{KEPT}", hasher.finish())))
    }

    /// The answer kept for this picture, this size and this kit. The file holds the whole picture it answers
    /// for, and is the answer only when that is this picture to the byte: a file that is cut short, written by
    /// another kit or about another picture is no answer, and the picture is tried.
    fn kept_answer(&self, svg: &str) -> Option<Answer> {
        let mut text = String::new();
        let file = fs::File::open(self.kept_file(svg)?).ok()?;
        file.take(MAX_ART_BYTES + 2 * MOST_SAID).read_to_string(&mut text).ok()?;
        let (answer, picture) = text.strip_prefix(&key())?.strip_prefix('\n')?.split_once('\n')?;
        read_answer(answer).filter(|_| picture == svg)
    }

    /// Keeps an answer. Whatever fails here, the answer is only not kept.
    fn keep(&self, svg: &str, answer: &Answer) {
        let (Some(file), Some(folder)) = (self.kept_file(svg), &self.kept) else { return };
        let _ = fs::create_dir_all(folder);
        let ours = |name: &PathBuf| name.extension().is_some_and(|ending| ending == KEPT);
        let kept: Vec<PathBuf> = fs::read_dir(folder)
            .map(|entries| entries.flatten().map(|entry| entry.path()).filter(ours).collect())
            .unwrap_or_default();
        if kept.len() >= MOST_KEPT {
            kept.iter().for_each(|old| drop(fs::remove_file(old)));
        }
        // Written beside and renamed, so no reader finds half of one.
        let beside = file.with_extension(format!("{}.new", std::process::id()));
        let text = format!("{}\n{}\n{svg}", key(), write_answer(answer));
        if fs::write(&beside, text).and_then(|()| fs::rename(&beside, &file)).is_err() {
            let _ = fs::remove_file(&beside);
        }
    }
}

/// What an answer is for besides the picture's own text: this kit, the parser and renderer it is built with
/// (`tests/alike.rs` holds the version named here to `Cargo.lock`), what the child did, and the size it drew at,
/// which is the renderer's own two pixels a unit. A kit that changes its rules changes the last number.
pub(super) fn key() -> String {
    let how = if cfg!(feature = "gpui") { "read and drawn at twice its size" } else { "read" };
    format!("sticker-kit {}, usvg {USVG}, {how}, rules 1", env!("CARGO_PKG_VERSION"))
}

/// The version of the parser and the renderer a kept answer is for.
pub const USVG: &str = "0.46.0";

/// The line a child says for an answer, and the line kept.
pub(super) fn write_answer(answer: &Answer) -> String {
    match answer {
        Ok((cost, drawn)) => format!("accepted {} {} {drawn}", cost.numbers, cost.paint),
        Err(why) => format!("refused: {}", one_line(why)),
    }
}

/// The answer a line holds, or `None` for a line that is not exactly one of the two.
fn read_answer(line: &str) -> Option<Answer> {
    if let Some(why) = line.strip_prefix("refused: ") {
        return (!why.is_empty() && why == one_line(why)).then(|| Err(why.to_string()));
    }
    let (numbers, rest) = line.strip_prefix("accepted ")?.split_once(' ')?;
    let (paint, drawn) = rest.split_once(' ')?;
    let digits = |text: &str| !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
    if !(digits(numbers) && digits(paint) && digits(drawn)) {
        return None;
    }
    let (cost, drawn) = (ArtCost { numbers: numbers.parse().ok()?, paint: paint.parse().ok()? }, drawn.parse().ok()?);
    // With a renderer, a picture is accepted for the pixels drawn of it.
    (drawn > 0 || !cfg!(feature = "gpui")).then_some(Ok((cost, drawn)))
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
    use super::{ArtCost, one_line, read_answer, write_answer};

    #[test]
    fn an_answer_is_one_of_two_lines_and_nothing_else_is_one() {
        let cost = ArtCost { numbers: 12, paint: 3456 };
        assert_eq!(write_answer(&Ok((cost, 400))), "accepted 12 3456 400");
        assert_eq!(read_answer("accepted 12 3456 400"), Some(Ok((cost, 400))));
        assert_eq!(read_answer("accepted 12 3456 0").is_none(), cfg!(feature = "gpui"), "nothing was drawn");
        assert_eq!(read_answer("refused: it holds text or an image"), Some(Err("it holds text or an image".into())));
        assert_eq!(write_answer(&Err("two\nlines\u{7}".into())), "refused: twolines");
        let not =
            ["", "accepted", "accepted 1", "accepted 1 2", "accepted 1 2 3 4", "accepted +1 2 3", "accepted 1 -2 3"];
        let more = [
            "accepted 1 2 3 ",
            "accepted 1 2 3\nmore",
            "Accepted 1 2 3",
            "refused: ",
            "refused:x",
            "ok",
            "accepted 1e3 2 3",
        ];
        for line in not.into_iter().chain(more) {
            assert_eq!(read_answer(line), None, "{line:?}");
        }
        assert_eq!(read_answer(&format!("accepted 1 {}0 3", u64::MAX)), None, "a number too large is none");
        assert_eq!(read_answer("refused: a\tb"), None, "a reason with a control character is not one");
        assert_eq!(one_line(&"x".repeat(300)).len(), 240);
    }
}
