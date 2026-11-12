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
mod hook;
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
pub use copy::{Outcome, RunCopy, clean_copy, make_copy};
pub use error::Error;
pub use find::{own_folder, user_home};
pub use hook::{hook_installed, install_hook, remove_hook};
pub use reader::{DayOptions, day, skip_names};
pub use reading::{PageOptions, page};
pub use ruling::{deliver, rule};
pub use run::{Ran, RunOptions, RunOutcome, run, stop_run};
pub use shapes::{Clash, Harness, HookState, PageData, PageRead, ReaderDay, ReaderTurn, Ruling, Wish, WishState};
pub use tap::{CLASHES_MAX, Settled, Side, Standing, keep_private, kept_page, settle, settle_kept, standing};
pub use uninstall::{UninstallOptions, Uninstalled, uninstall};
pub use wish::{MADE_MAX, MadeWish, Wished, made, wish, with_made};
