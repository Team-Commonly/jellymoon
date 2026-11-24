//! What a tap on an option left to do, and what that gave: the work `window.rs` runs off its thread.

use crate::stopping::{Stopped, Stopping};
use crate::working::{Worked, Working};

/// What a tap on an option left to do: work for another thread.
pub enum After {
    Stop(Stopping),
    Work(Working),
}

/// What an [`After`] gave.
pub enum Ended {
    Stopped(Stopped),
    Worked(Worked),
}

impl After {
    /// The work itself: it asks core, which reads files and a repo, or waits for a run to end.
    pub fn run(self) -> Ended {
        match self {
            After::Stop(stop) => Ended::Stopped(stop()),
            After::Work(work) => Ended::Worked(work()),
        }
    }
}
