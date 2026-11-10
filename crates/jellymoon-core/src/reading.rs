//! Turns a reader's day into page data with one reading call to the person's own harness, and checks what comes back
//! (ADR 0011, section 2). The model is not trusted: a day can hold any text an agent ever printed. So the call can do
//! nothing but return text (`call.rs`), and here code keeps a wish or a clash only when its words are the person's own,
//! writes the day, the ids and what was read itself, and cleans and cuts the few words that are the model's.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use serde::Deserialize;

use crate::redact::line;
use crate::{Clash, Error, PageData, PageRead, ReaderDay, Wish, call};

/// The limits of page data.
const WISHES_MAX: usize = 8;
const CLASHES_MAX: usize = 2;
const STARS_MAX: usize = 4;
const LABEL_MAX: usize = 40;
const STAR_MAX: usize = 60;
const QUOTE_MAX: usize = 200;
/// The shortest quote kept, in characters, unless it is everything the person typed in one turn.
const QUOTE_MIN: usize = 8;

/// One reading.
#[derive(Debug, Clone)]
pub struct PageOptions {
    /// The model the harness is asked for. It must be a name: letters, digits, dot, dash and underscore.
    pub model: String,
    /// The folder the call's own empty folder is made in, and removed from afterwards.
    pub dir: PathBuf,
    /// How long the call may take before it is stopped.
    pub limit: Duration,
    /// The harness program. None is `claude`, found on `PATH`.
    pub harness: Option<PathBuf>,
}

/// What the model proposes: wishes and clashes under ids of its own. A missing field, a wrong type or an unknown state
/// anywhere in it is refused whole; a field it adds is ignored.
#[derive(Deserialize)]
struct Reading {
    wishes: Vec<Wish>,
    clashes: Vec<Clash>,
}

/// Returns the day's page data. A day with no turns is an empty page and makes no call. A reading that is not page data
/// is an error: there is no page to print.
pub fn page(day: &ReaderDay, options: &PageOptions) -> Result<PageData, Error> {
    let mut projects: Vec<String> = Vec::new();
    for turn in &day.turns {
        if !projects.contains(&turn.project) {
            projects.push(turn.project.clone());
        }
    }
    let read = PageRead { turns: day.turns.len(), projects };
    let mut page = PageData { day: day.day.clone(), wishes: Vec::new(), clashes: Vec::new(), read };
    if !is_model_name(&options.model) {
        return Err(Error::Model);
    }
    if day.turns.is_empty() {
        return Ok(page);
    }
    let text = call::reading(day, options)?;
    let reading: Reading =
        serde_json::from_str(unfence(&text)).map_err(|e| Error::NotPageData { line: e.line(), column: e.column() })?;
    keep(day, reading, &mut page);
    Ok(page)
}

/// The only part of the harness's command line a caller chooses: a name, which cannot start a flag or hold a space.
fn is_model_name(model: &str) -> bool {
    let plain = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-');
    model.len() <= 64 && model.starts_with(|c: char| c.is_ascii_alphanumeric()) && model.chars().all(plain)
}

/// Takes the model's text out of a Markdown code fence, when it put one around it.
fn unfence(text: &str) -> &str {
    let text = text.trim();
    match text.split_once('\n') {
        Some((first, rest)) if first.starts_with("```") => rest.trim().strip_suffix("```").unwrap_or(rest.trim()),
        _ => text,
    }
}

/// Moves into the page what of the reading is the person's own: the first wishes whose words they typed, under the
/// page's ids, then the first clashes between two of those wishes whose quotes they typed.
fn keep(day: &ReaderDay, reading: Reading, page: &mut PageData) {
    let mut ids: HashMap<String, String> = HashMap::new(); // the model's id of a kept wish, to the page's
    for wish in reading.wishes {
        let label = line(&wish.label, LABEL_MAX);
        let Some(said) = typed(day, &wish.said) else {
            continue;
        };
        let known = page.read.projects.contains(&wish.project);
        if ids.contains_key(&wish.id) || label.is_empty() || !known || page.wishes.len() == WISHES_MAX {
            continue;
        }
        let stars = wish
            .stars
            .iter()
            .map(|star| line(star, STAR_MAX))
            .filter(|star| !star.is_empty())
            .take(STARS_MAX)
            .collect();
        let id = format!("w{}", page.wishes.len() + 1);
        ids.insert(wish.id, id.clone());
        page.wishes.push(Wish { id, label, said, project: wish.project, state: wish.state, stars });
    }
    for clash in reading.clashes {
        let (Some(a), Some(b)) = (ids.get(&clash.a), ids.get(&clash.b)) else {
            continue;
        };
        let (Some(quote_a), Some(quote_b)) = (typed(day, &clash.quote_a), typed(day, &clash.quote_b)) else {
            continue;
        };
        if a != b && page.clashes.len() < CLASHES_MAX {
            page.clashes.push(Clash { a: a.clone(), b: b.clone(), quote_a, quote_b, settled: clash.settled });
        }
    }
}

/// Returns the quote when it is the person's own words: an exact substring of one thing they typed that day, of whole
/// words, and long enough to mean something or else all of what they typed. That stops the model inventing words, and
/// passing off a letter or half a word as a quote. It does not stop it choosing: "delete the database" is whole words
/// of "do not delete the database". A quote over its length is then cut, with no mark added, so it is still theirs.
fn typed(day: &ReaderDay, quote: &str) -> Option<String> {
    let quote = quote.trim();
    let long_enough = quote.chars().count() >= QUOTE_MIN;
    let own = |said: &str| {
        (long_enough || said == quote)
            && said
                .match_indices(quote)
                .any(|(at, _)| !inside_a_word(said, at) && !inside_a_word(said, at + quote.len()))
    };
    if quote.is_empty() || !day.turns.iter().any(|turn| own(&turn.said)) {
        return None;
    }
    let end = quote.char_indices().nth(QUOTE_MAX).map_or(quote.len(), |(at, _)| at);
    Some(quote[..end].to_string())
}

/// Whether `at` splits a word of `said`: a letter or digit on both sides, in a script that puts spaces between words.
/// Every letter below U+2E80 is taken for one; Chinese, Japanese and Korean, above it, are cut anywhere.
fn inside_a_word(said: &str, at: usize) -> bool {
    let spaced = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() && c < '\u{2e80}');
    spaced(said[..at].chars().next_back()) && spaced(said[at..].chars().next())
}
