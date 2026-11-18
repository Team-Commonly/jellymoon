//! A quoted line is held to the file as a look would print it, and to a side that is a text file (the gate of
//! F85): no line of a private key block is quoted, wherever it is wrapped and whether the work made the file or
//! removed it, and nothing is quoted from a side that is a link or holds a zero byte, though the other side is
//! text.

use serde_json::json;

use super::checks::{change, dropped, passed};
use super::{moved_to, show, told};
use crate::world::printed;

/// The lines of a private key block as a file holds one, its body made up and wrapped at `width` characters. The
/// header is joined here, so that no scanner reads a key in this file.
fn block(width: usize) -> Vec<String> {
    let (body, label) = ("Mk9".repeat(50), ["PRIV", "ATE KEY"].concat());
    let wrapped = body.as_bytes().chunks(width).map(|line| String::from_utf8_lossy(line).into_owned());
    [format!("-----BEGIN {label}-----")].into_iter().chain(wrapped).chain([format!("-----END {label}-----")]).collect()
}

// A body line of a key block is no credential's shape by itself when it is short: one wrapped at 39 characters,
// and the last line of one wrapped at 64. `run show --file` prints the block as the mark, and no line of it is
// quoted: as a line the work put in a file, or as one it took with a file it removed. The plain lines around the
// blocks are quoted as before.
#[test]
fn no_line_of_a_private_key_block_is_quoted_from_a_file_the_work_made_or_removed() {
    let (narrow, wide) = (block(39), block(64));
    assert_eq!((narrow[1].len(), narrow[4].len(), wide[1].len(), wide[3].len()), (39, 33, 64, 22));
    let keys = format!("top line\n{}\nbetween\n{}\nplain after\n", narrow.join("\n"), wide.join("\n"));
    let (had, left) = ([("old-key.txt", keys.as_bytes())], [("new-key.txt", keys.as_bytes())]);
    let (made, said) = told(&had, &left, &["old-key.txt"], r#"{"state":"done"}"#);
    assert_eq!(said["outcome"], "completed", "{said}");
    let file = printed(&show(&made, &["--file", "1"]));
    assert_eq!(file["text"], "top line\n[redacted]\nbetween\n[redacted]\nplain after", "{file}");
    for plain in ["top line", "between", "plain after"] {
        assert_eq!(passed(&made, &change("new-key.txt", &[], &[plain]))["after"], json!([plain]));
        assert_eq!(passed(&made, &change("old-key.txt", &[plain], &[]))["before"], json!([plain]));
    }
    for line in [&narrow[1], &narrow[4], &wide[3], &narrow[0], &wide[4]] {
        dropped(&made, &change("new-key.txt", &[], &[line]));
        dropped(&made, &change("old-key.txt", &[line], &[]));
        dropped(&made, &change("new-key.txt", &[], &["plain after", line]));
    }
}

// The kind of a file is read from the side the work left. A file the work turned into a link has no text there,
// and a file that held a zero byte before the work is text there: neither is quoted from, on either side.
#[test]
fn nothing_is_quoted_from_a_file_that_became_a_link_or_from_one_that_was_no_text_before_the_work() {
    let had = [("turned.txt", &b"was a file\n"[..]), ("was-bin.txt", &b"gone line\n\0\n"[..])];
    let (made, said) = told(&had, &[("was-bin.txt", &b"text line\n"[..])], &[], r#"{"state":"done"}"#);
    assert_eq!(said["outcome"], "completed", "{said}");
    // The run's one commit, with `turned.txt` in it as a link: the entry is written into git's index, so no
    // link is made in a folder, which not every system lets a test do.
    let git = |args: &[&str]| made.w.git(&made.repo, args);
    git(&["checkout", "-q", "-B", "made-up", &made.w.commit(&made.repo, &made.branch())]);
    let leads = git(&["hash-object", "-w", "a.txt"]);
    git(&["update-index", "--cacheinfo", &format!("120000,{leads},turned.txt")]);
    git(&["commit", "-q", "--amend", "-m", &format!("Jellymoon run {}", made.run)]);
    git(&["checkout", "-q", "-f", "main"]);
    moved_to(&made, "made-up");
    let files = printed(&show(&made, &[]))["files"].clone();
    let listed = |n: usize| (files[n]["name"].clone(), files[n]["change"].clone(), files[n]["kind"].clone());
    assert_eq!(listed(0), (json!("turned.txt"), json!("changed"), json!("link")), "{files}");
    assert_eq!(listed(1), (json!("was-bin.txt"), json!("changed"), json!("text")), "{files}");
    dropped(&made, &change("turned.txt", &["was a file"], &[]));
    dropped(&made, &change("was-bin.txt", &["gone line"], &[]));
    dropped(&made, &change("was-bin.txt", &[], &["text line"]));
    dropped(&made, &change("was-bin.txt", &["gone line"], &["text line"]));
}
