//! The journal app without a screen (ADR 0011). [`day`] is the reader: one local day of what a person typed to Claude
//! Code. [`page`] turns that day into page data with one reading call to the person's own harness and keeps only what the
//! person said. [`rule`] keeps the person's choice on a clash; [`deliver`], run by the hook that [`install_hook`] adds
//! to the harness settings, hands it to each running session once. [`settle`] is a tap on a clash: the ruling, and
//! which side stands. [`uninstall`] takes off a machine what the install
//! script put there. [`make_copy`] gives a run its own copy of a repo and [`clean_copy`] ends it. A session log and a model's answer are both untrusted input: nothing here panics on either.

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
mod session;
mod shapes;
mod tap;
mod uninstall;

pub use command::{installed_command, turn_on_delivery};
pub use copy::{Outcome, RunCopy, clean_copy, make_copy};
pub use error::Error;
pub use find::{own_folder, user_home};
pub use hook::{hook_installed, install_hook, remove_hook};
pub use reader::{DayOptions, day, skip_names};
pub use reading::{PageOptions, page};
pub use ruling::{deliver, rule};
pub use shapes::{Clash, Harness, HookState, PageData, PageRead, ReaderDay, ReaderTurn, Ruling, Wish, WishState};
pub use tap::{CLASHES_MAX, Settled, Side, Standing, keep_private, kept_page, settle, settle_kept, standing};
pub use uninstall::{UninstallOptions, Uninstalled, uninstall};
