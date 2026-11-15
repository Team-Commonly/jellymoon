//! The page of a day with no session logs at all (ADR 0020, decision 5; F63). A day nothing was typed on has a
//! page all the same when it has a made wish, or a project on offer: the made wishes are merged onto it as onto
//! any page (`wish::with_made`), and the entry's chips are the names on offer ([`crate::offered`]). Logs that are
//! there and hold no turn of the day need nothing from here: the reader gives a day of no turns, and `page` an
//! empty page with no call.

use crate::{DayOptions, Error, PageData, PageOptions, ReaderDay, reader, reading, wish};

/// The page of `read`'s day when there are no session logs to read: an empty page that says so (`read.no_logs`),
/// made with no call to a model, since nothing was typed that one could read. The model's name is checked as it
/// is for any page. None when the reader's answer is anything but "no logs", or when the day has no made wish
/// that can be used and no project is on offer: there is nothing to draw, and the reader's answer stands, as it
/// did before a day could have a page without logs. Nothing is written.
pub fn page_with_no_logs(read: &DayOptions, options: &PageOptions) -> Result<Option<PageData>, Error> {
    if !matches!(reader::day(read), Err(Error::NoLogs(_))) {
        return Ok(None);
    }
    let day = wish::day_of(read.date.as_deref())?;
    let made = read.home.as_deref().is_some_and(|home| wish::any_made(home, &day));
    if !made && wish::offered(read).is_empty() {
        return Ok(None);
    }
    let mut page = reading::page(&ReaderDay { day, turns: Vec::new() }, options)?;
    page.read.no_logs = true;
    Ok(Some(page))
}
