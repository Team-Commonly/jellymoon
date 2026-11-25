//! The journal app without a screen (ADR 0011). [`day`] is the reader: one local day of what a person typed to Claude
//! Code. [`page`] turns that day into page data with one reading call to the person's own harness and keeps only what the
//! person said. [`rule`] keeps the person's choice on a clash; [`deliver`], run by the hook that [`install_hook`] adds
//! to the harness settings, hands it to each running session once. [`settle`] is a tap on a clash: the ruling, and
//! which side stands. [`wish`] keeps a wish the person made themselves, and [`with_made`] puts the day's on its page.
//! [`uninstall`] takes off a machine what the install
//! script put there. [`make_copy`] gives a run its own copy of a repo and [`clean_copy`] ends it. [`run`] starts the
//! person's harness on a made wish, in such a copy, and adds the wish's star when the run reports it finished. A session log and a model's answer are both untrusted input: nothing here panics on either.

mod busy;
mod call;
mod command;
mod copy;
mod error;
mod find;
mod hear;
mod hook;
mod lock;
mod no_logs;
mod project;
mod reader;
mod reading;
mod redact;
mod ruling;
mod run;
mod session;
mod shapes;
mod still;
mod tap;
mod uninstall;
mod wish;

pub use command::{installed_command, turn_on_delivery};
pub use copy::{Outcome, RunCopy, Stands, clean_copy, make_copy};
pub use error::Error;
pub use find::{HarnessFound, How, harness_found, own_folder, user_home};
pub use hear::{
    CANNOT_MS, EAR_MS, EarCannot, HEAR_LINE_MAX, HOLD_MS, HearEvent, HearOptions, Hearing, Jelly, JellyDrawn,
    JellyState, JellyText, KeyMove, LOOK_MS, SAID_SO_FAR_MAX, SLIP_MS, Said, Voice, WINDOW_MS, Whose, hear, jellyfish,
    voice,
};
pub use hook::{hook_installed, install_hook, remove_hook};
pub use no_logs::page_with_no_logs;
pub use project::{Known, PROJECTS_MAX, Projects, add_project, forget_project, new_project, projects};
pub use reader::{DayOptions, day, skip_names};
pub use reading::{PageOptions, page};
pub use redact::wish_start;
pub use ruling::{deliver, rule};
pub use run::{
    ANSWER_READ, Alone, FileChange, FileKind, KeepOptions, Kept, KeptItself, Piece, Ran, RunOptions, RunOutcome,
    SHAPE_MODEL, Shape, ShapeGo, ShapeOutcome, ShapeSeen, Shaped, Shown, ShownEntry, ShownFile, Started, Step, Steps,
    Told, Tool, Tossed, Undone, Waiting, Waits, WorkDone, WorkState, WorkTap, answer_run, doing, ended_alone,
    followable, keep_run, kept_itself, run, run_apart, shape, show_file, show_run, stands, stop_run, toss_run,
    undo_run, waiting, waits, wish_after, wish_again, wish_of_shape, wish_shaped, work_tapped,
};
pub use shapes::{Clash, Harness, HookState, PageData, PageRead, ReaderDay, ReaderTurn, Ruling, Wish, WishState};
pub use tap::{CLASHES_MAX, Settled, Side, Standing, keep_private, kept_page, settle, settle_kept, standing};
pub use uninstall::{UninstallOptions, Uninstalled, uninstall};
pub use wish::{
    MADE_MAX, MADE_SAID_MAX, MadeWish, Wished, going, made, offered, wish, wish_offered, with_made, with_made_said,
};
