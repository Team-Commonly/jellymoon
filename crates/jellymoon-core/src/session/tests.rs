//! The reader of one log, from inside: what no command can show of it.

use super::*;

// The blocks are cut out of the whole prompt, before anything bounds it, so this step has to grow with its length.
#[test]
fn a_megabyte_of_block_starts_with_whole_blocks_between_them_is_cut_in_seconds() {
    let shapes = [
        "<system-reminder><artifact-content-authored-by-others/>",
        "<artifact-view-context(The JSON line above)",
        "(The JSON line above<system-reminder>x</system-reminder>",
        "<system-reminder></artifact-view-context><artifact-view-context</system-reminder>",
    ];
    for shape in shapes {
        let text = shape.repeat((1 << 20) / shape.len());
        let start = std::time::Instant::now();
        let _ = without_blocks(&text);
        assert!(start.elapsed() < std::time::Duration::from_secs(5), "{shape}: {:?}", start.elapsed());
    }
    let said = "a <system-reminder>LEAK\n</system-reminder>b <artifact-view-context x>LEAK</artifact-view-context>c \
                <artifact-content-authored-by-others />d (The JSON line above is LEAK)e";
    assert_eq!(without_blocks(said), "a b c d e");
}

/// A log that cannot be sought, which counts how often it was asked to.
struct Unsought(usize);

impl Seek for Unsought {
    fn seek(&mut self, _: SeekFrom) -> io::Result<u64> {
        self.0 += 1;
        Err(io::Error::other("no seeking"))
    }
}

#[test]
fn a_log_s_length_is_asked_then_sought_and_only_a_log_that_gives_neither_is_read_to_its_end() {
    let no = || Err(io::Error::other("no length"));
    let mut log = io::Cursor::new(b"abcde".to_vec());
    log.set_position(2);
    assert_eq!((length(no(), &mut log), log.position()), (5, 0), "sought, and left at its start");
    // What the system says is taken as said: the log is not sought, and is left where it is.
    log.set_position(2);
    let mut unsought = Unsought(0);
    assert_eq!((length(Ok(3), &mut log), log.position(), length(Ok(0), &mut unsought), unsought.0), (3, 2, 0, 0));
    // Neither: it is read to its end, never taken for an empty one.
    assert_eq!((length(no(), &mut unsought), unsought.0 > 0), (u64::MAX, true));
}

/// A log with no end: one typed line, over and over. It counts the bytes it gave, and gives no more than `most`.
struct Endless {
    line: Vec<u8>,
    given: usize,
    most: usize,
}

impl Read for Endless {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        assert!(self.given < self.most, "the reading went on past {} bytes of a log with no end", self.most);
        let (at, n) = (self.given % self.line.len(), into.len().min(self.most - self.given));
        for (i, byte) in into[..n].iter_mut().enumerate() {
            *byte = self.line[(at + i) % self.line.len()];
        }
        self.given += n;
        Ok(n)
    }
}

// In any build, with no seam: a reading asks its log for its length in bytes and returns.
#[test]
fn a_reading_of_a_log_with_no_end_returns_having_read_exactly_its_length() {
    let line = br#"{"type":"user","cwd":"/work/alpha","origin":{"kind":"human"},"message":{"content":"typed"}}"#;
    let line = [line.as_slice(), b"\n"].concat();
    // Whole lines, a line cut off at the length, nothing at all, and more than the reader's buffer holds.
    for (len, whole) in [(3 * line.len(), 3), (3 * line.len() + 40, 3), (0, 0), ((1 << 16) + 7, (1 << 16) / line.len())]
    {
        let mut log = Endless { line: line.clone(), given: 0, most: len + (1 << 20) };
        let turns = read(&mut log, len as u64, "-work-alpha", &[], &mut HashSet::new(), 1 << 20);
        assert_eq!((turns.len(), log.given), (whole, len), "a length of {len}");
        assert!(turns.iter().all(|(_, turn, _)| turn.said == "typed" && turn.project == "alpha"));
    }
}

// A folder of a megabyte of slashes is walked once, three segments in hand, with no list of them made.
#[test]
fn a_project_is_named_from_a_folder_of_any_length_in_one_walk() {
    let names = [
        ("/work/alpha", "alpha"),
        ("/work/alpha/", "alpha"),
        ("/work/beta/.claude/worktrees/fix-login/web", "beta"),
        (r"C:\work\beta\.claude\worktrees\fix", "beta"),
        ("/a/.claude/worktrees/x/b/.claude/worktrees/y", "a"),
        ("/.claude/worktrees/x", "x"),
        ("/work/beta/.claude/worktrees", "worktrees"),
        ("", ""),
    ];
    fn project(cwd: &str) -> &str {
        place(cwd).0
    }
    for (cwd, want) in names {
        assert_eq!(project(cwd), want, "{cwd}");
    }
    assert_eq!(place("/work/beta/.claude/worktrees/fix-login/web").1, "/work/beta");
    assert_eq!((place("/work/alpha/").1, place(r"C:\work\beta").1, place("").1), ("/work/alpha", r"C:\work\beta", ""));
    let slashes = "/".repeat(1 << 20);
    let start = std::time::Instant::now();
    assert_eq!(project(&slashes), "");
    assert_eq!(project(&format!("{slashes}gamma{slashes}")), "gamma");
    assert_eq!(project(&format!("{slashes}beta/.claude/worktrees/x{slashes}")), "beta");
    assert!(start.elapsed() < std::time::Duration::from_secs(20), "{:?}", start.elapsed());
}
