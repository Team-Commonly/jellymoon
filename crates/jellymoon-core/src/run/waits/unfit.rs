//! A change that did not fit the person's folder when its run ended, said at once (ADR 0030, decision 10;
//! `docs/development/show.md`, "Work that waits"). The keep by itself that core refused, as the folder had
//! changed in the same place, wrote that reason with the run's record (`run/keep/itself.rs`). The note of such
//! work says it first, in the sentences the refusal of a tap on `Keep it` says, and has `Make it again` where
//! `Keep it` would be: core has just refused that keep, and no later tap would land it. The reason is the run's
//! end's, and the folder can change after it: the change the work clashed with is undone, or the person puts
//! their branch back. So where a record says it, the keep is asked once more when the note is read ([`still`]),
//! and work that a keep would land now has the waiting note, with `Keep it`. Every other reason a record
//! holds, and none, leaves the note as it was.

use std::path::Path;

use super::super::RunOptions;
use super::super::keep::{KeepOptions, keep_run};
use super::{Waits, again, plain};
use crate::copy::Unkept;

/// The reason a run's record holds, as far as it holds now. A clash is asked again, as `run keep --ask` asks:
/// nothing is changed and nothing is written, and the record keeps its word. Where a keep would land the work
/// now, there is no reason; any other answer, a keep at work in this home too, leaves the record's.
pub(super) fn still(home: &Path, options: &RunOptions, written: Option<Unkept>) -> Option<Unkept> {
    let asked = || keep_run(home, options, KeepOptions { ask: true, with_settings: false });
    let fits = written == Some(Unkept::Clash) && asked().is_ok_and(|kept| kept.fits == Some(true));
    if fits { None } else { written }
}

impl Waits {
    /// Whether the run's record says that the work did not fit the folder when the run ended.
    pub fn unfit(&self) -> bool {
        self.unkept == Some(Unkept::Clash)
    }

    /// The first lines of the note of work that was read: that it does not fit and what `Make it again` does,
    /// or else that it waits for a yes and what its choices do.
    pub(super) fn first(&self) -> Vec<String> {
        if self.unfit() {
            vec![plain::CLASH.to_string(), again::DOES.to_string()]
        } else {
            vec![plain::waits(!self.over()).to_string()]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Unkept, Waits};
    use crate::copy::Unread;

    const WAITS: &str = "It waits for your yes: Keep it puts it in; Toss it throws it away.";
    const CLASH: &str =
        "Your folder changed in the same place after this change was made, so the two do not fit together.";
    const DOES: &str = "Make it again throws this one away and makes the wish again on your folder as it is now.";

    fn waits(unkept: Option<Unkept>) -> Waits {
        Waits {
            wish: "m2".to_string(),
            run: "r0123456789abcdef".to_string(),
            said: "make the uploader retry twice".to_string(),
            read: true,
            made: 0,
            changed: 1,
            removed: 0,
            names: vec!["a.txt".to_string()],
            more: 0,
            obeyed: Vec::new(),
            plain: true,
            why: "the work of run r0123456789abcdef waits: files made 0, changed 1, removed 0".to_string(),
            result: None,
            project: "alpha".to_string(),
            unkept,
        }
    }

    #[test]
    fn work_that_did_not_fit_says_so_first_and_has_make_it_again_where_keep_it_was() {
        let unfit = waits(Some(Unkept::Clash));
        let said = "“make the uploader retry twice”";
        assert_eq!(unfit.lines(), [CLASH, DOES, said, "1 file changed.", "a.txt"]);
        assert_eq!(unfit.title(), "Your folder is not changed yet.");
        for sure in [false, true] {
            assert_eq!(unfit.choices(sure), ["Show me", "Make it again", "Toss it"]);
        }
        // Work that was not read has no choice and says why it was not read, whatever its record holds.
        let unread = Waits { read: false, ..unfit };
        assert_eq!((unread.choices(false).len(), unread.lines().contains(&CLASH.to_string())), (0, false));
    }

    #[test]
    fn every_other_reason_and_none_leave_the_note_and_its_choices_as_they_were() {
        let said = "“make the uploader retry twice”";
        let others = [Unkept::InWay, Unkept::NoBranch, Unkept::History, Unkept::Settings, Unkept::Busy];
        let moved = Unkept::Unread(Unread::Moved);
        for unkept in others.into_iter().chain([moved]).map(Some).chain([None]) {
            let as_before = waits(unkept);
            assert_eq!(as_before.lines(), [WAITS, said, "1 file changed.", "a.txt"], "{unkept:?}");
            assert_eq!(as_before.choices(false), ["Show me", "Keep it", "Toss it"], "{unkept:?}");
            assert_eq!(as_before.choices(true), ["Show me", "Keep it, with those files", "Toss it"], "{unkept:?}");
            // What `run waiting` prints holds no word of the reason.
            let printed = serde_json::to_string(&as_before).unwrap();
            assert_eq!(printed, serde_json::to_string(&waits(None)).unwrap(), "{unkept:?}");
        }
        let printed = serde_json::to_string(&waits(Some(Unkept::Clash))).unwrap();
        assert_eq!(printed, serde_json::to_string(&waits(None)).unwrap());
        assert!(printed.contains("\"plain\":true") && !printed.contains("unkept"), "{printed}");
    }
}
