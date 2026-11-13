//! The name of the branch a keep lands on (`docs/development/keep.md`, "What the record takes"): whatever a keep
//! writes in a record, a keep reads back. A branch whose name the record's reader would not take is refused
//! before anything is written, so no keep leaves a record that the next one calls someone else's.

use serde_json::json;

use super::cut::cut;
use super::refused::unfit;
use super::support::skipped;
use super::{BEFORE, FORWARD, JOINED, Made, finished, kept, printed};

const UNNAMED: &str = "the checkout stands on a branch whose name Jellymoon does not write in a run's record: the \
                       name holds a space of some kind, a line break or a control character, or is longer than 1024 \
                       bytes";
const FINISHED: &str = "run {run} was kept: a keep that was cut off had landed its work, and this one finished it";

/// The person stands on a branch of this name, at the commit `main` names.
fn stand_on(made: &Made, name: &str) -> String {
    let whole = format!("refs/heads/{name}");
    made.git(&["update-ref", &whole, "main"]);
    made.git(&["symbolic-ref", "HEAD", &whole]);
    assert_eq!(made.status(), "", "the checkout is clean on {whole:?}");
    whole
}

/// Names git takes for a branch and the record's reader does not: a no-break space, an ideographic space, a line
/// separator, and the two C1 controls NEL and CSI.
const UNREAD: [&str; 5] = ["a\u{a0}b", "a\u{3000}b", "a\u{2028}b", "a\u{85}b", "a\u{9b}b"];

/// A name of 300 bytes in two parts, since a disk takes 255 for one. None where a path that long cannot be
/// made (Windows without long paths).
fn long() -> Option<String> {
    (!cfg!(windows)).then(|| format!("{}/{}", "x".repeat(150), "y".repeat(149)))
}

// The gate's third finding: the first keep landed on such a branch, and every later keep called the record
// someone else's.
#[test]
fn a_branch_whose_name_a_record_would_not_read_back_is_refused_with_nothing_changed() {
    for name in UNREAD {
        for joined in [false, true] {
            let made = finished();
            if joined {
                made.moved_on();
            }
            stand_on(&made, name);
            unfit(&made, UNNAMED);
            // Nothing was written, so on a branch with a plain name the work is kept as ever.
            made.git(&["symbolic-ref", "HEAD", "refs/heads/main"]);
            let ran = made.keep(&[]);
            kept(&made, &ran, &made.w.commit(&made.repo, "main"), "main", if joined { JOINED } else { FORWARD });
        }
    }
}

#[test]
fn whatever_name_a_keep_writes_in_a_record_the_next_keep_reads() {
    let mut names = vec!["ok-ünï".to_string(), "@".to_string(), "a\u{202e}b".to_string(), "a/b.c-d_e".to_string()];
    match long() {
        Some(long) => names.push(long),
        None => skipped("a branch name of 300 bytes: this system makes no path that long"),
    }
    for name in names {
        let made = finished();
        let (whole, end) = (stand_on(&made, &name), made.end());
        let said = printed(&made.keep(&[]));
        assert_eq!((&said["kept"], &said["why"]), (&json!(end), &json!(FORWARD.replace("{run}", &made.run))));
        assert_eq!(made.w.commit(&made.repo, &whole), end, "{name:?}");
        assert_eq!(made.record()["onto"], json!(whole));
        // The name is printed cleaned, as every name is: no character that turns the line around gets out.
        let onto = said["onto"].as_str().unwrap();
        assert!(!onto.contains('\u{202e}') && !onto.is_empty(), "{onto:?}");
        // The record is read back: the next keep says the work was kept, and says the same name.
        let before = made.held();
        let again = printed(&made.keep(&[]));
        assert_eq!((&again["why"], &again["onto"]), (&json!(BEFORE.replace("{run}", &made.run)), &said["onto"]));
        assert!(before == made.held());
    }
}

#[test]
fn a_keep_cut_off_on_a_branch_of_an_odd_name_is_finished_by_the_next() {
    for name in ["ok-ünï", "a\u{202e}b"] {
        let made = finished();
        let (whole, end) = (stand_on(&made, name), made.end());
        cut(&made, 3);
        assert_eq!((made.record()["onto"].clone(), made.record()["work"].clone()), (json!(whole), json!(null)));
        let said = printed(&made.keep(&[]));
        assert_eq!((&said["kept"], &said["why"]), (&json!(end), &json!(FINISHED.replace("{run}", &made.run))));
        assert_eq!(made.record()["work"], json!("kept"));
        assert!(made.w.branches(&made.repo).is_empty());
    }
    // And on a name no record holds, a keep never comes as far as a step to be cut at.
    let made = finished();
    stand_on(&made, UNREAD[0]);
    let before = made.held();
    let ran = made.keep_in(&made.repo, &[], &[("JELLYMOON_TEST_KEEP_CUT", "3".into())]);
    made.refuses(&ran, &before, &format!("{UNNAMED}: {}", super::UNCHANGED));
}
