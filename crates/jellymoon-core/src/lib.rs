//! The journal app without a screen (ADR 0011). [`day`] is the reader: one local day of what a person typed to Claude
//! Code. [`page`] turns that day into page data with one reading call to the person's own harness and keeps only what the
//! person said. [`rule`] keeps the person's choice on a clash; [`deliver`], run by the hook that [`install_hook`] adds
//! to the harness settings, hands it to each running session once. [`uninstall`] takes off a machine what the install
//! script put there. A session log and a model's answer are both untrusted input: nothing here panics on either.

mod call;
mod error;
mod hook;
mod reader;
mod reading;
mod redact;
mod ruling;
mod session;
mod shapes;
mod uninstall;

pub use error::Error;
pub use hook::{install_hook, remove_hook};
pub use reader::{DayOptions, day};
pub use reading::{PageOptions, page};
pub use ruling::{deliver, rule};
pub use shapes::{Clash, Harness, HookState, PageData, PageRead, ReaderDay, ReaderTurn, Ruling, Wish, WishState};
pub use uninstall::{UninstallOptions, Uninstalled, uninstall};
