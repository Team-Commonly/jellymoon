//! What a keep makes in the repo's store before it touches the checkout (`keep.rs`): the tree of the person's
//! commits and the run's work together, and the one new commit that holds it. Nothing names either until the
//! commit lands, and a keep that stops before then leaves them for git to tidy away.

use super::fits;
use super::{At, SAID_MAX, Unkept, Unread, is_id, line};
use crate::copy::JELLYMOON;

/// The tree of the person's commits and the run's work together, merged in the store: no file of the checkout is
/// read or written, and a merge driver of the repo's config is not started (`git.rs`).
pub(super) fn merged(at: &At, tip: &str) -> Result<String, Unkept> {
    let said = line(at.repo, &[], &["version"])?.ok_or(Unread::Repo)?;
    if !fits::new_enough(&said) {
        return Err(Unkept::Old);
    }
    // A follow-up's work on followed work that was kept as a new commit: the two have the followed work's own
    // commit as the base to merge from, which no history of the branch holds. A git from before 2.40 cannot be
    // told a base, and merges from the one it finds: that brings back whatever of the followed work the person
    // has taken out since, so it is not asked.
    if at.of.held.is_some() && !fits::at_least(&said, 40) {
        return Err(Unkept::NoBase);
    }
    let base = at.of.held.map(|_| format!("--merge-base={}", at.of.named.start));
    let mut merge = vec!["merge-tree", "--write-tree", "--no-messages"];
    merge.extend(base.as_deref());
    merge.extend([tip, at.of.named.end]);
    let (code, printed) = at.repo.answer(None, &[], &merge, &[], SAID_MAX).ok_or(Unread::Repo)?;
    let tree = printed.split(|byte| *byte == b'\n').next().and_then(|tree| std::str::from_utf8(tree).ok());
    match (code, tree) {
        (Some(0), Some(tree)) if is_id(tree) => Ok(tree.to_string()),
        (Some(1), _) => Err(Unkept::Clash),
        // The merge writes a tree, asked or not: in a store that takes no new object git ends with an error.
        _ => Err(Unkept::Unmerged),
    }
}

/// Who the one new commit is by: the person's name and address, as git reads them for this checkout at this
/// moment, through the checkout's own git folder, so a linked worktree's own config is read. Where either is
/// missing it is Jellymoon's own, both of them, as a run's commit is where the repo names nobody (`clean.rs`):
/// a person who never told git who they are keeps the second of two changes made together as they kept the
/// first. One that git would cut down to nothing is missing: git takes spaces, control characters and
/// `,:;<>"\'` off both ends of a name (measured on git 2.50.1, which refuses a name of only those; an older git
/// takes a `.` off too).
pub(super) fn author(at: &At) -> Result<(String, String), Unkept> {
    let own = |key: &str| line(at.mine, &[], &["config", "--get", key]);
    let said = |text: &str| text.chars().any(|c| c > ' ' && !",:;<>\"\\'".contains(c));
    Ok(match (own("user.name")?, own("user.email")?) {
        (Some(name), Some(address)) if said(&name) && said(&address) => (name, address),
        _ => (JELLYMOON.0.to_string(), JELLYMOON.1.to_string()),
    })
}

/// Makes the one new commit: the merged tree on top of the person's last commit, by them, with the message a
/// run's commit has. git's plumbing writes it, which has no hook, and nothing names it yet.
pub(super) fn commit(at: &At, who: &(String, String), tip: &str, tree: &str) -> Result<String, Unkept> {
    let (name, address) = (who.0.as_str(), who.1.as_str());
    let who = [
        ("GIT_AUTHOR_NAME", name),
        ("GIT_AUTHOR_EMAIL", address),
        ("GIT_COMMITTER_NAME", name),
        ("GIT_COMMITTER_EMAIL", address),
    ];
    let message = format!("Jellymoon run {}", at.of.run);
    let made = line(at.repo, &who, &["commit-tree", "-p", tip, "-m", &message, tree])?;
    made.filter(|made| is_id(made)).ok_or(Unread::Repo.into())
}
