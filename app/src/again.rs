//! `Make it again`, carried (ADR 0030, decision 10). Core offers the choice under a keep it refused because the
//! change no longer fits the folder, and under work that waits because it did not fit when its run ended, where
//! `Keep it` would be (`working::waiting`); to a tap on either core says which words and which project
//! (`WorkDone::Again`). Here the wish is made as the entry makes a typed one, by the same calls
//! (`jellymoon_core::wish_offered`, `Today::asking`), so every check and limit of a typed wish holds. Only once
//! core kept the new wish is the work that does not fit tossed, by the call a tap on `Toss it` is carried by; a
//! wish core refuses tosses nothing, and neither does a tap with no installed command to run it. The run is started last: a toss asks git where every copy of the repo
//! stands, and core refuses it while a new run's copy is being made. Every word is core's or a typed wish's.

use jellymoon_core::{WorkDone, WorkTap};

use crate::today::Today;
use crate::wishing::{UNKEPT, of};
use crate::working::Ask;

impl Today {
    /// Core's answer to a tap on a choice of `wish`, by `ask`. An answer that the wish is to be made again is
    /// carried on here: the new wish, then the toss, asked of core by `ask` as `Toss it` is, and then the run.
    pub(crate) fn asked(&self, wish: &str, tap: WorkTap, ask: &impl Ask) -> WorkDone {
        let (said, project) = match ask(self, wish, tap) {
            WorkDone::Again(said, project) => (said, project),
            done => return done,
        };
        let unmade = |refused: jellymoon_core::Error| WorkDone::unmade(UNKEPT, of(&said, &format!("{refused}.")));
        // With no installed command no run could be started: nothing is made, and nothing is tossed.
        if let Err(refused) = jellymoon_core::installed_command(&self.bin.clone().unwrap_or_default()) {
            return unmade(refused);
        }
        let made = match jellymoon_core::wish_offered(&self.read(), &said, &project) {
            Ok(made) => made.wish,
            Err(refused) => return unmade(refused),
        };
        let tossed = ask(self, wish, WorkTap::Toss);
        tossed.remade((self.asking(made.id, made.said))())
    }
}
