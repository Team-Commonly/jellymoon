//! What a finished run made, read from the repo's store and from nowhere else (ADR 0017, decision 2;
//! `docs/development/show.md`). The run's work is the one commit its copy ended as (`clean.rs`), on the branch of
//! the run's name: the files that commit changes from its one parent. Which commit that is, the run's record
//! names (F48, [`Named`]), and the repo must say the same; of a record from before there is only the branch's
//! shape to go by. Nothing is checked out, no file of the work is written anywhere, and git is told an object's
//! id and never a file's name, so a name is only ever data that git printed. Every read of what git prints has a
//! size it stops at; what git says beside it is heard out, and no more of it is kept (`call::run_most`).

use std::fs;
use std::path::Path;

use super::git::{Git, Repo};
use super::{Places, branch};

/// The most files of a run's work that are listed, and so the most that can be asked for by number.
pub(crate) const LISTED: usize = 200;
/// The most bytes of one file that are read.
pub(crate) const TEXT_MAX: usize = 64 * 1024;
/// The most bytes of git's list of changed files that are read: over a hundred thousand files of everyday names.
pub(super) const LIST_MAX: usize = 16 * 1024 * 1024;
/// The most bytes of the run's commit that are read: its message is one line. A commit of that many or more is
/// not one a run's copy ended as.
const COMMIT_MAX: usize = 64 * 1024;
/// The most bytes of git's answer about the run's branch that are read: one line, of a name and an id.
pub(super) const REF_MAX: usize = 16 * 1024;

/// Why a run's work cannot be shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unread {
    /// No git was found, or the folder of the home that git is started in holds something (`Git::find`).
    Home,
    /// The folder is no checkout git opens, or git did not answer, or an object the work names is not there.
    Repo,
    /// The repo has no branch of the run's name.
    Missing,
    /// The branch is a name for another branch.
    Symbolic,
    /// The branch's last commit is not the one commit a run's copy ends as, or not the end the record names.
    Moved,
    /// The repo found is not the repo the record names.
    Another,
    /// The end's one parent is not the start the record names.
    Start,
    /// More files than are read.
    Many,
}

/// What was done to a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Done {
    Made,
    Changed,
    Removed,
}

/// What a tree's entry is, by its mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Entry {
    File,
    Link,
    /// A commit of another repo. Nothing of it is in this repo's store, and nothing of it is asked for.
    Repo,
}

/// One file of the work: as it now is, or as it was when the work removed it.
pub(crate) struct WorkFile {
    /// The path as git keeps it: any bytes but a zero.
    pub path: Vec<u8>,
    pub done: Done,
    pub entry: Entry,
    pub size: u64,
    /// The file's first [`TEXT_MAX`] bytes. Nothing for a link or another repo's entry.
    pub head: Vec<u8>,
    id: String,
}

/// What a run's record names of its work (`run/record.rs`): the git folder the repo's checkouts share, the commit
/// the copy was made from and the commit it ended as. They are a file's words. The folder is compared with the
/// one found and is never opened or looked up, and each id is compared with an id git printed and never handed
/// to git.
pub(crate) struct Named<'a> {
    pub repo: &'a Path,
    pub start: &'a str,
    pub end: &'a str,
}

/// The first [`LISTED`] files of a run's work, in git's own order, and how many more there are. No word of git's
/// is kept of any failure here. With `only`, a file's number from 1, that file alone is read past its size: one
/// file is printed without a read of any other file's bytes.
///
/// With what the record `named`, the work is the change from that start to that end in that repo, and is read
/// only when the repo found at `folder` is that repo, the run's branch names that end, and the end's one parent
/// is that start. The ids git is then given are the ones git printed, which are those. With none named, by a
/// record from before, the work is the branch's last commit against its one parent, by its shape alone.
pub(crate) fn work_of(
    home: &Path,
    folder: &Path,
    run: &str,
    named: Option<&Named>,
    only: Option<usize>,
) -> Result<(Vec<WorkFile>, usize), Unread> {
    let git = Git::find(&Places::of(home).runs, true).map_err(|_| Unread::Home)?;
    let found = git.discover(folder).map_err(|_| Unread::Repo)?;
    // A bare repo has no checkout to be inside of.
    let [_, inside, below, _, shared] = &found[..] else { return Err(Unread::Repo) };
    if inside != "true" || !below.is_empty() {
        return Err(Unread::Repo);
    }
    if named.is_some_and(|named| !same(named.repo, Path::new(shared))) {
        return Err(Unread::Another);
    }
    let repo = git.open(Path::new(shared)).map_err(|_| Unread::Repo)?;
    let tip = tip(&repo, run)?;
    if named.is_some_and(|named| named.end != tip) {
        return Err(Unread::Moved);
    }
    let parent = parent(&repo, run, &tip)?;
    if named.is_some_and(|named| named.start != parent) {
        return Err(Unread::Start);
    }
    // No rename is ever listed: git's plumbing finds one only when told to, whatever the repo's config says.
    let list = ["diff-tree", "-r", "--no-abbrev", "-z", &parent, &tip];
    let list = repo.bytes(&list, &[], LIST_MAX).ok_or(Unread::Repo)?;
    if list.len() >= LIST_MAX {
        return Err(Unread::Many);
    }
    let mut files = listed(&list).ok_or(Unread::Repo)?;
    let more = files.len().saturating_sub(LISTED);
    files.truncate(LISTED);
    read(&repo, &mut files, only).ok_or(Unread::Repo)?;
    Ok((files, more))
}

/// Whether the folder git `found` is the one a record `named`. One folder has many spellings, on Windows and on a
/// Mac (case, a link, `..`, a short name, either separator, `\\?\`), and the system gives one resolved path for
/// all of them: a run writes that one (`MadeFrom`), and here the folder found is resolved the same way and must be
/// it, part for part. The record's own path is only compared. Nothing is asked of the system about it, since it
/// is a file's word and could name any place, one on another machine too.
pub(super) fn same(named: &Path, found: &Path) -> bool {
    fs::canonicalize(found).is_ok_and(|found| found == named)
}

/// Whether a word is a whole object id: 40 or 64 hex digits and nothing else, so never an option, a name or
/// anything git would work out a commit from.
pub(crate) fn is_id(word: &str) -> bool {
    matches!(word.len(), 40 | 64) && word.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// The commit the run's branch names. The branch is asked for by its whole name, and only the line of that whole
/// name is taken: a tag or a remote's branch of the same short name is another name, and so is a branch below it.
/// The answer is a line or two; one as long as reading stops at, or that is not text, is no answer.
pub(super) fn tip(repo: &Repo, run: &str) -> Result<String, Unread> {
    tip_of(repo, &format!("refs/heads/{}", branch(run)))
}

/// [`tip`] for any branch, by its whole name: the one the person stands on, when a run's work is kept (`keep.rs`).
pub(super) fn tip_of(repo: &Repo, name: &str) -> Result<String, Unread> {
    let format = "--format=%(refname) %(objectname) %(objecttype) %(symref)";
    let refs = repo.bytes(&["for-each-ref", format, name], &[], REF_MAX).filter(|refs| refs.len() < REF_MAX);
    let refs = refs.and_then(|refs| String::from_utf8(refs).ok()).ok_or(Unread::Repo)?;
    let ours = refs.lines().find_map(|line| line.strip_prefix(name)?.strip_prefix(' '));
    let mut words = ours.ok_or(Unread::Missing)?.split(' ');
    let (id, kind, leads_to) = (words.next().unwrap_or(""), words.next(), words.next().unwrap_or(""));
    if !leads_to.is_empty() {
        return Err(Unread::Symbolic);
    }
    if kind == Some("commit") && is_id(id) { Ok(id.to_string()) } else { Err(Unread::Moved) }
}

/// The one parent of the run's commit: the commit its copy was made of. A commit with another message, or with no
/// parent or two, is not one a run's copy ended as: someone put it there, and its change is not shown as the
/// run's. Of a record from before, which names neither commit, this shape is all that is checked. A commit that
/// was not read whole is not judged by its start: what was read could end just where a run's message does.
pub(super) fn parent(repo: &Repo, run: &str, tip: &str) -> Result<String, Unread> {
    let commit = repo.bytes(&["cat-file", "commit", tip], &[], COMMIT_MAX).ok_or(Unread::Repo)?;
    if commit.len() >= COMMIT_MAX {
        return Err(Unread::Moved);
    }
    let commit = String::from_utf8(commit).map_err(|_| Unread::Moved)?;
    let (head, message) = commit.split_once("\n\n").ok_or(Unread::Moved)?;
    let mut parents = head.lines().filter_map(|line| line.strip_prefix("parent "));
    match (parents.next(), parents.next()) {
        (Some(parent), None) if is_id(parent) && message == format!("Jellymoon run {run}\n") => Ok(parent.to_string()),
        _ => Err(Unread::Moved),
    }
}

/// The files of `diff-tree -r -z`: for each, a line of the two modes, the two ids and a letter, then the path, each
/// ended by a zero byte. A removed file is described as it was. None when the list is not in that form.
fn listed(list: &[u8]) -> Option<Vec<WorkFile>> {
    let (mut files, mut parts) = (Vec::new(), list.split(|byte| *byte == 0));
    while let Some(line) = parts.next().filter(|line| !line.is_empty()) {
        let words: Vec<&str> = std::str::from_utf8(line).ok()?.strip_prefix(':')?.split(' ').collect();
        let [was_mode, mode, was_id, id, letter] = words[..] else { return None };
        let done = match letter {
            "A" => Done::Made,
            "M" | "T" => Done::Changed,
            "D" => Done::Removed,
            _ => return None,
        };
        let (mode, id) = if done == Done::Removed { (was_mode, was_id) } else { (mode, id) };
        let entry = match mode {
            "120000" => Entry::Link,
            "160000" => Entry::Repo,
            _ => Entry::File,
        };
        let path = parts.next().filter(|path| !path.is_empty() && is_id(id))?.to_vec();
        files.push(WorkFile { path, done, entry, size: 0, head: Vec::new(), id: id.to_string() });
    }
    Some(files)
}

/// Reads each file's size, and the start of each that is a file: of every one, for the list, which says of each
/// whether it is text, or with `only`, a number from 1, of that file alone. Sizes are asked for in one call, and
/// so is every file that is read whole: git prints each as a line of its id, kind and size, then its bytes and a
/// line break. A longer file is read alone, to the size reading stops at. Whatever is not a blob of that id is no
/// file.
fn read(repo: &Repo, files: &mut [WorkFile], only: Option<usize>) -> Option<()> {
    let ids = |of: &[&WorkFile]| of.iter().flat_map(|file| [file.id.as_bytes(), b"\n"].concat()).collect::<Vec<u8>>();
    let kept: Vec<&WorkFile> = files.iter().filter(|file| file.entry != Entry::Repo).collect();
    let sizes = repo.bytes(&["cat-file", "--batch-check"], &ids(&kept), kept.len() * 128 + 1)?;
    let sizes: Vec<u64> = String::from_utf8(sizes).ok()?.lines().map(sized).collect::<Option<_>>()?;
    if sizes.len() != kept.len() {
        return None;
    }
    let mut sizes = sizes.into_iter();
    for file in files.iter_mut().filter(|file| file.entry != Entry::Repo) {
        file.size = sizes.next()?;
    }
    let asked = |at: usize, file: &WorkFile| file.entry == Entry::File && only.is_none_or(|only| only == at + 1);
    let whole = |(at, file): &(usize, &WorkFile)| asked(*at, file) && file.size <= TEXT_MAX as u64;
    let short: Vec<&WorkFile> = files.iter().enumerate().filter(whole).map(|(_, file)| file).collect();
    let most = short.iter().map(|file| usize::try_from(file.size).unwrap_or(TEXT_MAX) + 130).sum::<usize>() + 1;
    let printed = repo.bytes(&["cat-file", "--batch"], &ids(&short), most)?;
    let mut rest = &printed[..];
    for (_, file) in files.iter_mut().enumerate().filter(|(at, file)| asked(*at, file)) {
        if file.size > TEXT_MAX as u64 {
            file.head = repo.bytes(&["cat-file", "blob", &file.id], &[], TEXT_MAX)?;
            continue;
        }
        let size = usize::try_from(file.size).ok()?;
        let line = rest.iter().position(|byte| *byte == b'\n')?;
        if sized(std::str::from_utf8(&rest[..line]).ok()?) != Some(file.size) {
            return None;
        }
        file.head = rest.get(line + 1..line + 1 + size)?.to_vec();
        rest = rest.get(line + 2 + size..)?;
    }
    Some(())
}

/// The size in git's line of an object: `<id> blob <size>`. None for what is missing or is no blob.
fn sized(line: &str) -> Option<u64> {
    let mut words = line.split(' ');
    let (id, kind, size) = (words.next()?, words.next()?, words.next()?);
    (is_id(id) && kind == "blob" && words.next().is_none()).then(|| size.parse().ok())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_git_prints_is_read_by_its_zero_bytes_and_a_removed_file_is_as_it_was() {
        let (a, b, none) = ("a".repeat(40), "b".repeat(40), "0".repeat(40));
        let list = format!(
            ":000000 100644 {none} {a} A\0-rf\nx\0:100644 000000 {b} {none} D\0gone\0:100644 120000 {a} {b} T\0l\0\
             :000000 160000 {none} {a} A\0sub\0"
        );
        let files = listed(list.as_bytes()).unwrap();
        let seen: Vec<_> = files.iter().map(|file| (&file.path[..], file.done, file.entry, file.id.as_str())).collect();
        let want = [
            (&b"-rf\nx"[..], Done::Made, Entry::File, a.as_str()),
            (b"gone", Done::Removed, Entry::File, b.as_str()),
            (b"l", Done::Changed, Entry::Link, b.as_str()),
            (b"sub", Done::Made, Entry::Repo, a.as_str()),
        ];
        assert_eq!(seen, want);
        assert!(listed(b"").unwrap().is_empty());
        let broken = [
            format!(":000000 100644 {none} {a} A\0"),
            format!(":000000 100644 {none} {a} R100\0x\0y\0"),
            format!(":000000 100644 {none} main A\0x\0"),
            format!("000000 100644 {none} {a} A\0x\0"),
        ];
        for list in broken {
            assert!(listed(list.as_bytes()).is_none(), "{list:?}");
        }
    }

    #[test]
    fn a_size_is_taken_only_of_a_blob_git_found() {
        let id = "c".repeat(40);
        assert_eq!(sized(&format!("{id} blob 12")), Some(12));
        for line in [format!("{id} missing"), format!("{id} tree 12"), format!("{id} blob -1"), "main blob 1".into()] {
            assert_eq!(sized(&line), None, "{line}");
        }
    }
}
