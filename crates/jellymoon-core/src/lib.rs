//! The journal app without a screen (ADR 0011). [`day`] is the reader: one local day of what a person typed to Claude
//! Code. [`page`] turns that day into page data with one reading call to the person's own harness and keeps only what the
//! person said. [`uninstall`] takes off a machine what the install script put there. A session log and a model's answer are both untrusted input: nothing here panics on either.

mod call;
mod error;
mod reader;
mod reading;
mod redact;
mod session;
mod shapes;
mod uninstall;

pub use error::Error;
pub use reader::{DayOptions, day};
pub use reading::{PageOptions, page};
pub use shapes::{Clash, Harness, PageData, PageRead, ReaderDay, ReaderTurn, Wish, WishState};
pub use uninstall::{UninstallOptions, Uninstalled, uninstall};
