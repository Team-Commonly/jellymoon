//! What a keep makes in the repo's store before it touches the checkout (`keep.rs`): the tree of the person's
//! commits and the run's work together, and the one new commit that holds it. Nothing names either until the
//! commit lands, and a keep that stops before then leaves them for git to tidy away.

use super::fits;
use super::{At, SAID_MAX, Unkept, Unread, is_id, line};

/// The tree of the person's commits and the run's work together, merged in the store: no file of the checkout is
/// read or written, and a merge driver of the repo's config is not started (`git.rs`).
pub(super) fn merged(at: &At, tip: &str) -> Result<String, Unkept> {
    let said = line(at.repo, &[], &["version"])?.ok_or(Unread::Repo)?;
    if !fits::new_enough(&said) {
        return Err(Unkept::Old);
    }
    let merge = ["merge-tree", "--write-tree", "--no-messages", tip, at.of.named.end];
    let (code, printed) = at.repo.answer(None, &[], &merge, &[], SAID_MAX).ok_or(Unread::Repo)?;
    let tree = printed.split(|byte| *byte == b'\n').next().and_then(|tree| std::str::from_utf8(tree).ok());
    match (code, tree) {
        (Some(0), Some(tree)) if is_id(tree) => Ok(tree.to_string()),
        (Some(1), _) => Err(Unkept::Clash),
        _ => Err(Unread::Repo.into()),
    }
}

/// The person's name and address, as git reads them for this checkout at this moment: through the checkout's own
/// git folder, so a linked worktree's own config is read. None is ever made up, and one that git would cut down
/// to nothing is none: git takes spaces, control characters and `,:;<>"\'` off both ends of a name (measured on
/// git 2.50.1, which refuses a name of only those; an older git takes a `.` off too).
pub(super) fn author(at: &At) -> Result<(String, String), Unkept> {
    let own = |key: &str| line(at.mine, &[], &["config", "--get", key]);
    let said = |text: &str| text.chars().any(|c| c > ' ' && !",:;<>\"\\'".contains(c));
    match (own("user.name")?, own("user.email")?) {
        (Some(name), Some(address)) if said(&name) && said(&address) => Ok((name, address)),
        _ => Err(Unkept::Nobody),
    }
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
