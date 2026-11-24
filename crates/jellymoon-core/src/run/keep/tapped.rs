//! A keep from a page (`waits.rs`), and why it kept nothing, typed. The command line prints a refusal as a
//! sentence and nothing else; a page words two of core's reasons for a person who has never seen git
//! (`waits/plain.rs`), so its keep is told the reason itself, beside that sentence. No sentence is ever matched.

use std::cell::Cell;
use std::path::Path;

use super::{Kept, RunOptions, Yes, keep};
use crate::Error;
use crate::copy::Unkept;

/// [`super::keep_run`], but of the files the person's tools obey it lands only those `named`, by their names as
/// they are printed, and only when they are every such file of the work. Work that would land another, or that
/// has not one of them, is refused whole, in a page's words. A refusal is the command line's sentence, and with
/// it core's reason when the keep itself came to one: none of a refusal before the keep, or of the names.
pub(in crate::run) fn keep_named(
    home: &Path,
    options: &RunOptions,
    named: &[String],
) -> Result<Kept, (Option<Unkept>, Error)> {
    let why = Cell::new(None);
    keep(home, options, false, Yes::To(named, &why)).map_err(|refused| (why.get(), refused))
}

impl Yes<'_> {
    /// `unkept`, left where a page's keep reads it: the reason the sentence that is printed next was made of.
    pub(super) fn told(self, unkept: Unkept) -> Unkept {
        if let Yes::To(_, why) = self {
            why.set(Some(unkept));
        }
        unkept
    }
}
