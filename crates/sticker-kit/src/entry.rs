//! What is typed into a scene's entry, as plain data: one line of text, the part of it an input method is still
//! composing (marked), a caret and the chosen option. Nothing here draws or knows a window, so every rule is tested
//! with no display. Places are bytes into the text, always between characters; the ranges a platform's input method
//! speaks are UTF-16 and are turned into bytes here and nowhere else.

use std::ops::Range;

use crate::layout::MAX_TEXT_CHARS;
use crate::scene::Entry;
use crate::text::{ends, is_unseen};

/// How many options of an entry are offered: the first ones. The rest are not drawn and cannot be chosen.
pub const MAX_ENTRY_OPTIONS: usize = 6;

/// The most characters an entry holds, whatever its scene asks for.
pub const MAX_ENTRY_CHARS: usize = MAX_TEXT_CHARS;

/// What `enter` gives: the text with no space at either end, and the chosen option when the entry has options.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Made {
    pub text: String,
    pub option: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntryState {
    text: String,
    marked: Option<Range<usize>>,
    caret: usize,
    chosen: usize,
    options: usize,
    most: usize,
}

impl Default for EntryState {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

impl EntryState {
    /// An empty entry with `options` options, the first chosen, that holds at most `most` characters. Both are held
    /// to the kit's own bounds; a cap of 0 is the kit's cap.
    pub fn new(options: usize, most: usize) -> Self {
        let most = if most == 0 { MAX_ENTRY_CHARS } else { most.min(MAX_ENTRY_CHARS) };
        Self { text: String::new(), marked: None, caret: 0, chosen: 0, options: options.min(MAX_ENTRY_OPTIONS), most }
    }

    pub fn of(entry: &Entry) -> Self {
        Self::new(entry.options.len(), entry.most)
    }

    /// Holds what is typed to a scene's entry as it is now: text over the new cap is cut and a composition ended.
    pub fn fit(&mut self, entry: &Entry) {
        let bounds = Self::of(entry);
        (self.options, self.most) = (bounds.options, bounds.most);
        self.chosen = self.chosen.min(self.options.saturating_sub(1));
        let kept = prefix(&self.text, self.most);
        if kept < self.text.len() {
            self.text.truncate(kept);
            self.marked = None;
        }
        self.caret = self.caret.min(self.text.len());
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// Where the next character goes, in bytes.
    pub fn caret(&self) -> usize {
        self.caret
    }

    /// The text an input method is still composing, in bytes.
    pub fn marked(&self) -> Option<Range<usize>> {
        self.marked.clone()
    }

    /// The chosen option; `None` when the entry has no options.
    pub fn chosen(&self) -> Option<usize> {
        (self.options > 0).then_some(self.chosen)
    }

    pub fn most(&self) -> usize {
        self.most
    }

    /// Puts `new` in place of a UTF-16 range, or of the composition, or at the caret, and ends the composition.
    pub fn replace(&mut self, range_utf16: Option<Range<usize>>, new: &str) {
        self.put(range_utf16, new);
        self.marked = None;
    }

    /// Like [`replace`](Self::replace), with what was put in left marked as the composition. `caret_utf16` is where
    /// the input method wants the caret, counted from the start of the marked text; without it the caret is at its end.
    pub fn mark(&mut self, range_utf16: Option<Range<usize>>, new: &str, caret_utf16: Option<Range<usize>>) {
        let put = self.put(range_utf16, new);
        if let Some(wanted) = caret_utf16 {
            let inside = Self { text: self.text[put.clone()].to_string(), ..Self::new(0, 0) };
            self.caret = put.start + inside.from_utf16(wanted).end;
        }
        self.marked = (!put.is_empty()).then_some(put);
    }

    /// Ends the composition and keeps its text.
    pub fn unmark(&mut self) {
        self.marked = None;
    }

    /// Replaces a range with as much of `new`, cleaned, as the cap leaves room for. Returns where it now is.
    fn put(&mut self, range_utf16: Option<Range<usize>>, new: &str) -> Range<usize> {
        let caret = self.caret..self.caret;
        let range = range_utf16.map(|range| self.from_utf16(range)).or(self.marked.clone()).unwrap_or(caret);
        // One character more than can ever fit is enough to tell where the last whole one ends, however much is pasted.
        let new: String = new.chars().filter_map(one_line).take(self.most + 1).collect();
        let stays = self.text.chars().count() - self.text[range.clone()].chars().count();
        let new = &new[..prefix(&new, self.most.saturating_sub(stays))];
        self.text.replace_range(range.clone(), new);
        self.caret = range.start + new.len();
        range.start..self.caret
    }

    pub fn backspace(&mut self) {
        let before = self.before(self.caret);
        self.cut(before..self.caret);
    }

    pub fn delete(&mut self) {
        let after = self.after(self.caret);
        self.cut(self.caret..after);
    }

    fn cut(&mut self, range: Range<usize>) {
        if self.marked.is_none() {
            self.text.replace_range(range.clone(), "");
            self.caret = range.start;
        }
    }

    pub fn left(&mut self) {
        self.go(self.before(self.caret));
    }

    pub fn right(&mut self) {
        self.go(self.after(self.caret));
    }

    pub fn home(&mut self) {
        self.go(0);
    }

    pub fn end(&mut self) {
        self.go(self.text.len());
    }

    /// Moves the caret to the start of the character `at` is in. Like every editing key, it leaves a composition
    /// alone: the input method owns the keys until it ends.
    pub fn set_caret(&mut self, at: usize) {
        self.go(self.before(at.saturating_add(1)));
    }

    fn go(&mut self, to: usize) {
        if self.marked.is_none() {
            self.caret = to.min(self.text.len());
        }
    }

    /// The last place between two characters that is before `at`, or the start.
    fn before(&self, at: usize) -> usize {
        ends(&self.text).into_iter().rev().find(|end| *end < at).unwrap_or(0)
    }

    /// The first place between two characters that is after `at`, or the end.
    fn after(&self, at: usize) -> usize {
        ends(&self.text).into_iter().find(|end| *end > at).unwrap_or(self.text.len())
    }

    pub fn next_option(&mut self) {
        self.chosen = (self.chosen + 1) % self.options.max(1);
    }

    pub fn previous_option(&mut self) {
        self.chosen = (self.chosen + self.options.max(1) - 1) % self.options.max(1);
    }

    pub fn choose(&mut self, option: usize) {
        if option < self.options {
            self.chosen = option;
        }
    }

    /// What was entered, and the entry is empty again with the same option chosen. Nothing while a composition is
    /// marked (that Enter belongs to the input method, to confirm it) and nothing for text that is only spaces.
    pub fn enter(&mut self) -> Option<Made> {
        if self.marked.is_some() || self.text.trim().is_empty() {
            return None;
        }
        let text = self.text.trim().to_string();
        self.text.clear();
        self.caret = 0;
        Some(Made { text, option: self.chosen() })
    }

    pub fn len_utf16(&self) -> usize {
        self.text.encode_utf16().count()
    }

    /// A range of bytes, as the UTF-16 range a platform's input method counts in.
    pub fn to_utf16(&self, bytes: Range<usize>) -> Range<usize> {
        let units = |at: usize| -> usize {
            self.text.char_indices().take_while(|(i, _)| *i < at).map(|(_, c)| c.len_utf16()).sum()
        };
        units(bytes.start)..units(bytes.end)
    }

    /// A UTF-16 range as bytes. A place inside a character is after it, a place past the end is the end, and a range
    /// given backwards is the same range, so the result always lies between characters of the text.
    pub fn from_utf16(&self, range_utf16: Range<usize>) -> Range<usize> {
        let bytes = |at: usize| {
            let (mut units, mut bytes) = (0, 0);
            for c in self.text.chars() {
                if units >= at {
                    break;
                }
                units += c.len_utf16();
                bytes += c.len_utf8();
            }
            bytes
        };
        bytes(range_utf16.start.min(range_utf16.end))..bytes(range_utf16.end.max(range_utf16.start))
    }
}

/// A character as an entry holds it: a line break or a tab is a space, and what nobody sees is nothing.
fn one_line(c: char) -> Option<char> {
    match c {
        '\n' | '\r' | '\t' => Some(' '),
        c if c.is_control() || is_unseen(c) => None,
        c => Some(c),
    }
}

/// How many bytes of `text` are its first whole characters of at most `most` `char`s together. A character a reader
/// sees as one (a letter with its accent, a joined emoji) is kept whole or left out.
fn prefix(text: &str, most: usize) -> usize {
    let (mut kept, mut count) = (0, 0);
    for end in ends(text) {
        count += text[kept..end].chars().count();
        if count > most {
            break;
        }
        kept = end;
    }
    kept
}
