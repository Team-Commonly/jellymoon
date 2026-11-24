//! A finished change is kept by itself (F92, ADR 0028; `docs/development/keep.md`, "Kept by itself"): when a
//! run's end is noted, its work goes into the person's folder with no yes asked, wherever a keep would take no
//! further yes. It is the keep of `keep.rs` and nothing else, with three differences: it lands only work that
//! is moved forward or joined; it lands no file the person's tools obey and no change to a `.gitattributes`; and
//! it leaves the run's branch and writes down the commit the person's branch named before, which is all an undo
//! goes by (`undo.rs`). Whatever refuses it prints nothing: the work waits as it always did, and the reason the
//! keep came to is written with the run's record ([`unkept`]; ADR 0030, decision 10), for a page to say at once.

use std::path::Path;

use serde::{Deserialize, Deserializer};

use super::super::{RunOptions, WorkState, record};
use super::{Yes, keep};
use crate::copy::{self, Keep, Named, Unkept};

/// Whether a test's build was told that every change waits (`JELLYMOON_TEST_WAITS` is 1), as before F92: the
/// tests of everything that is done with work that waits. A release holds no read of it: it is not compiled in.
fn off() -> bool {
    #[cfg(debug_assertions)]
    return std::env::var_os("JELLYMOON_TEST_WAITS").is_some_and(|waits| waits == "1");
    #[cfg(not(debug_assertions))]
    false
}

/// Keeps the work of the wish's run by itself, and says whether it is in the person's folder now.
pub(in super::super) fn keep_itself(home: &Path, options: &RunOptions) -> bool {
    !off() && keep(home, options, false, Yes::Itself).is_ok_and(|kept| kept.how.is_some_and(meant))
}

/// Writes with the run's record why its keep by itself kept nothing: core's typed reason and no sentence, under
/// the keep's own lock. It is what was true when the run ended, and no look asks the repo for it again. Nothing
/// of any other keep; a record that cannot be written leaves the work waiting with no reason, as before.
pub(super) fn unkept(of: &Keep, record: &mut record::Record, why: Unkept) {
    if of.itself {
        record.unkept = Some(why);
        let _ = record::write(of.home, of.run, record);
    }
}

/// The reason in a record, as it is read: none where the word is one this build does not know, so a record a
/// later build wrote is still a record here.
pub(in super::super) fn known<'de, D: Deserializer<'de>>(from: D) -> Result<Option<Unkept>, D::Error> {
    let word = serde_json::Value::deserialize(from)?;
    Ok(serde_json::from_value(word).ok())
}

/// Whether a keep landed as a keep by itself means to: the one way it can be undone.
pub(super) fn meant(how: copy::How) -> bool {
    matches!(how, copy::How::Forward | copy::How::Joined)
}

/// Whether a path as git keeps it is a `.gitattributes`, in any folder: it changes how git writes other files.
pub(super) fn attributes(path: &str) -> bool {
    path.rsplit('/').next() == Some(".gitattributes")
}

/// What is done once a change was kept by itself, under the same lock: of every change of the same repo that
/// was kept by itself on an earlier day, the run's branch goes and the record forgets the commit before it. Such
/// a change is undone no more, so that the branches kept for an undo are those of one day.
pub(super) fn sweep(of: &Keep, day: &str) {
    for (run, mut record) in record::all(of.home) {
        let old = record.work == Some(WorkState::Kept) && record.before.is_some() && record.back.is_none();
        let theirs = record.repo.as_deref() == Some(of.named.repo) && record.day.as_str() < day;
        let Some(end) = record.end.clone().filter(|end| old && theirs && copy::is_id(end)) else { continue };
        record.before = None;
        if record::write(of.home, &run, &record).is_ok() {
            let named = Named { repo: of.named.repo, start: &end, end: &end };
            copy::swept(&Keep { run: &run, named: &named, was: None, held: None, ..*of });
        }
    }
}
