//! Turns a reader's day into page data with one reading call to the person's own harness, and checks what comes back
//! (ADR 0011, section 2). The model is not trusted: a day can hold any text an agent ever printed. So the call can do
//! nothing but return text (`call.rs`), and here code keeps a wish or a clash only when its words are the person's own,
//! writes the day, the ids and what was read itself, and cleans and cuts the few words that are the model's.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Duration;

use serde::Deserialize;

use crate::redact::line;
use crate::{Clash, Error, PageData, PageRead, ReaderDay, Wish, call};

/// The limits of page data.
const WISHES_MAX: usize = 8;
const CLASHES_MAX: usize = 2;
/// How many of a reading's wishes, and of its clashes, are looked at. A model can write any number of them.
const SEEN_MAX: usize = 64;
const STARS_MAX: usize = 4;
const LABEL_MAX: usize = 40;
const STAR_MAX: usize = 60;
const QUOTE_MAX: usize = 200;
/// The shortest quote kept, in characters, unless it is everything the person typed in one turn.
const QUOTE_MIN: usize = 8;
/// The same for a quote whose every character is at or above U+2E80 (Chinese, Japanese, Korean), which say more in one.
const QUOTE_MIN_CJK: usize = 4;

/// One reading.
#[derive(Debug, Clone)]
pub struct PageOptions {
    /// The model the harness is asked for. It must be a name: letters, digits, dot, dash and underscore.
    pub model: String,
    /// The folder the call's own empty folder is made in, and removed from afterwards.
    pub dir: PathBuf,
    /// How long the call may take before it is stopped.
    pub limit: Duration,
    /// The harness program. None is `claude`, found on `PATH` or in a well-known place (`find.rs`).
    pub harness: Option<PathBuf>,
    /// The person's home folder, where Claude Code's own installers put `claude`.
    pub home: Option<PathBuf>,
    /// The folders looked in after the home's. None is the system's usual ones.
    pub system: Option<Vec<PathBuf>>,
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
    let read = PageRead { turns: day.turns.len(), projects: projects(day) };
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

/// The day's projects, each once, in the order first typed in.
fn projects(day: &ReaderDay) -> Vec<String> {
    let mut seen = HashSet::new();
    day.turns.iter().filter(|turn| seen.insert(turn.project.as_str())).map(|turn| turn.project.clone()).collect()
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
/// page's ids, then the first clashes between two of those wishes whose quotes they typed. Only the first
/// [`SEEN_MAX`] of each are looked at, and the looking stops when the page is full.
fn keep(day: &ReaderDay, reading: Reading, page: &mut PageData) {
    let mut ids: HashMap<String, String> = HashMap::new(); // the model's id of a kept wish, to the page's
    for wish in reading.wishes.into_iter().take(SEEN_MAX) {
        if page.wishes.len() == WISHES_MAX {
            break;
        }
        let label = line(&wish.label, LABEL_MAX);
        let Some(said) = typed(day, &wish.said) else {
            continue;
        };
        if ids.contains_key(&wish.id) || label.is_empty() || !page.read.projects.contains(&wish.project) {
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
    for clash in reading.clashes.into_iter().take(SEEN_MAX) {
        if page.clashes.len() == CLASHES_MAX {
            break;
        }
        let (Some(a), Some(b)) = (ids.get(&clash.a), ids.get(&clash.b)) else {
            continue;
        };
        let (Some(quote_a), Some(quote_b)) = (typed(day, &clash.quote_a), typed(day, &clash.quote_b)) else {
            continue;
        };
        if a != b {
            page.clashes.push(Clash { a: a.clone(), b: b.clone(), quote_a, quote_b, settled: clash.settled });
        }
    }
}

/// Returns the quote when it is the person's own words: an exact substring of one thing they typed that day, of whole
/// words, and long enough to mean something or else all of what they typed. That stops the model inventing words, and
/// passing off a letter or half a word as a quote. It does not stop it choosing: "delete the database" is whole words
/// of "do not delete the database". A quote over its length is then cut, with no mark added, so it is still theirs.
/// A quote longer than what was typed is not searched for, so a quote of any length costs what a short one does.
fn typed(day: &ReaderDay, quote: &str) -> Option<String> {
    let quote = quote.trim();
    let min = if quote.chars().all(|c| c >= '\u{2e80}') { QUOTE_MIN_CJK } else { QUOTE_MIN };
    let long_enough = quote.chars().count() >= min;
    let own = |said: &str| {
        quote.len() <= said.len()
            && (long_enough || said == quote)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Harness, ReaderTurn, WishState};

    fn day(turns: usize) -> ReaderDay {
        let turn = |n: usize| ReaderTurn {
            at: "09:00".into(),
            tool: Harness::ClaudeCode,
            project: format!("project-{n}"),
            said: format!("please build the thing numbered {n} today"),
            reply: String::new(),
        };
        ReaderDay { day: "2026-11-09".into(), turns: (0..turns).map(turn).collect() }
    }

    fn wish(n: usize, said: String) -> Wish {
        let (id, project) = (format!("m{n}"), "project-0".to_string());
        Wish { id, label: "a thing".into(), said, project, state: WishState::Working, stars: Vec::new() }
    }

    fn kept(day: &ReaderDay, reading: Reading) -> PageData {
        let read = PageRead { turns: day.turns.len(), projects: projects(day) };
        let mut page = PageData { day: day.day.clone(), wishes: Vec::new(), clashes: Vec::new(), read };
        keep(day, reading, &mut page);
        page
    }

    // A reading can hold any number of wishes and clashes; only the first SEEN_MAX of each are looked at.
    #[test]
    fn only_the_first_wishes_and_clashes_of_a_reading_are_looked_at() {
        let day = day(3);
        let typed = "build the thing numbered 0".to_string();
        let mut wishes: Vec<Wish> = (0..SEEN_MAX - 1).map(|n| wish(n, "words nobody typed".into())).collect();
        wishes.push(wish(SEEN_MAX - 1, typed.clone()));
        wishes.push(wish(SEEN_MAX, "build the thing numbered 1".into()));
        let page = kept(&day, Reading { wishes, clashes: Vec::new() });
        assert_eq!(page.wishes.iter().map(|w| w.said.as_str()).collect::<Vec<_>>(), [typed.as_str()]);

        let wishes = vec![wish(0, typed.clone()), wish(1, "build the thing numbered 1".into())];
        let clash = |a: &str, quote_a: &str| Clash {
            a: a.into(),
            b: "m1".into(),
            quote_a: quote_a.into(),
            quote_b: "build the thing numbered 1".into(),
            settled: false,
        };
        let mut clashes: Vec<Clash> = (0..SEEN_MAX).map(|_| clash("m0", "words nobody typed")).collect();
        clashes.push(clash("m0", &typed));
        let page = kept(&day, Reading { wishes: wishes.clone(), clashes });
        assert!(page.clashes.is_empty(), "the clash after the last one looked at is not kept");
        let page = kept(&day, Reading { wishes, clashes: vec![clash("m0", &typed)] });
        assert_eq!(page.clashes.len(), 1);
    }

    // What a reading costs does not grow with what the model wrote: a quote longer than what the person typed is
    // refused unread, a full page stops the looking, and projects are told apart through a set.
    #[test]
    fn a_huge_quote_and_a_full_page_cost_nothing_on_a_huge_day() {
        let day = day(4000);
        let start = std::time::Instant::now();
        let page = kept(&day, Reading { wishes: vec![wish(0, "aB3 ".repeat(1 << 20))], clashes: Vec::new() });
        assert!(page.wishes.is_empty());
        assert_eq!(page.read.projects.len(), 4000);
        let full = (0..20_000).map(|n| wish(n, "build the thing numbered 0".into())).collect();
        assert_eq!(kept(&day, Reading { wishes: full, clashes: Vec::new() }).wishes.len(), WISHES_MAX);
        assert!(start.elapsed() < Duration::from_secs(10), "{:?}", start.elapsed());
    }

    // Every wish here costs a search of everything typed that day, 4 MB of it, so the time is the number of wishes
    // looked at: 64 take a fraction of a second, and all 5000 took 27 s where this was measured.
    #[test]
    fn a_reading_of_thousands_of_wishes_costs_what_the_first_ones_do() {
        let mut day = day(4000);
        for turn in &mut day.turns {
            turn.said.push_str(&" and then some more words about it".repeat(30));
        }
        let wishes = (0..5000).map(|n| wish(n, format!("words nobody typed {n}"))).collect();
        let clash = |n: usize| Clash {
            a: "m0".into(),
            b: "m1".into(),
            quote_a: format!("words nobody typed {n}"),
            quote_b: "build the thing numbered 1".into(),
            settled: false,
        };
        let start = std::time::Instant::now();
        assert!(kept(&day, Reading { wishes, clashes: Vec::new() }).wishes.is_empty());
        let wishes = vec![wish(0, "build the thing numbered 0".into()), wish(1, "build the thing numbered 1".into())];
        let page = kept(&day, Reading { wishes, clashes: (0..5000).map(clash).collect() });
        assert!(page.clashes.is_empty());
        assert!(start.elapsed() < Duration::from_secs(10), "{:?}", start.elapsed());
    }
}
