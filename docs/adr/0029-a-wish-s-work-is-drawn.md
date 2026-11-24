# 0029 · A wish's work is drawn

Status: Built in the app, 2026-11-24: a run at work is a track and work that waits is its sheets, with their
sentences behind a tap. Proved with no window by the tests of `app/src/drawing.rs` and `app/src/live.rs`, at the
made-up window by the app's tests of work that waits, and for the field by `crates/sticker-kit/tests/pencil.rs`
and `tests/field_draw.rs`. Nobody has seen it in a real window. It uses the pictures of
[ADR 0027](0027-the-kit-draws-in-pencil.md) in the column of [ADR 0026](0026-the-page-is-one-column.md).

## Why

The kit can draw a wish's progress and its result as small pencil pictures, and the app built none. A person who
made a wish still read a note of sentences to learn that it is being worked on, and another, longer one, under a
third of what the run says it did, to learn that three files wait in a copy. The owner asked that a wish's work
be shown as the pictures, with the words there for whoever wants them.

## Decisions

1. **A run at work is a track.** For the newest working made wish the page holds one drawn note, `steps-<wish>`:
   `Picture::Track` with core's count of the run's steps and the stage of the newest (`Thinking` before the
   first, `Looking` for a read, a glob or a grep, `Making` for an edit or a write). It stands in the place of the
   plain note that says the wish is being worked on, which is not on the page while the track is; that note's
   title and body are the track's first words, then the stage as a sentence and the count, each only when there
   is one. The pills over the little ones are as they were. No file's name is drawn: the track's `file` is always
   empty. A page that has no such note (one with a clash) has no track either, as it had no note of the steps.
2. **Work that waits is its sheets.** `work-<wish>` is a drawn note: `Picture::Sheets` with core's three counts
   (`Waits::made`, `changed`, `removed`; the app counts nothing, and never the names) in the state `Waiting`,
   core's choices in core's order as its options, and core's title and then core's lines as its words. A tap on
   a choice is the tap it was: the choice's words and its place reach core unchanged.
3. **A drawn note can hold a field** (the kit: `field` on `Note::Drawn`). It is a text note's field, placed,
   drawn, typed in and reported by the text note's own code, under the options, open or closed. So "More like
   this" and a follow-up typed under finished work are where they were.
4. **What stays text, and why.** A note that holds someone else's words, or a refusal a person must read, is not
   folded behind a picture: a note with a run's question (its `quoted` words), the note that asks once more
   before a keep (`sure-`, every name a yes is given to), `busy-`, `wish-`, `file-`, work that could not be read
   (its reason is the whole note), and `said-`, core's answer to a keep or a toss.
5. **Sentences on a tap, and what is hidden until then.** A tap on a drawn note's picture or on its `?` opens the
   note; another closes it. Which notes are open is the window's own state (`drawing::Open`, held by `Showing`):
   core is not asked and nothing is written. One pure pass (`drawing::pass`) is applied where every scene goes to
   the kit, just before the column's order (`drawing::column`, the only caller of `column::column`): it sets each
   drawn note open or closed and takes `told-<wish>` off the page while the drawn `work-<wish>` of that wish is
   there and closed. So a finished wish is first one picture with its choices.
   *Why that is safe:* nothing is lost and nothing is decided unseen. Every sentence and the told note are one
   tap away, the picture says with its `?` that there are words, and the choice that changes a person's folder
   names no file: a keep of work that touches files a person's tools obey is still asked once more in a text
   note that is never folded. Where a wish's work is not a drawn note the told note stays on the page.
6. **`Show me` opens the note it was asked of.** What it brings is the rows of the changes in the told note and
   a file's own note; closed, the rows would not be on the page and the tap would seem to do nothing. So when
   core's answer holds them, the wish's drawn note is opened.
7. **A look does not close what a person opened.** A look gives the page its notes again; the set keeps an id
   while a drawn note of that id is on the scene it is asked to draw, and drops it when the note leaves. A note
   that comes back comes back closed.
8. **The print is open.** `layout`, the made-up window's `placed` and the walk's check of what is placed have no
   tap to ask for words with, so they print every drawn note open, with the told note. What a closed page places
   is proved at `drawing::column` and the kit's own layout.
9. **A change kept by itself is its sheets in the folder.** Joined with ADR 0028: the note `kept-<wish>` is a
   drawn note, `Sheets` in the state `In`, with core's counts, core's one choice (`Undo it`, or none under later
   commits) and core's title and lines as its words; closed, what its run says it did is folded as it is for
   work that waits. This needs no sentence compared: core's answer of where the change stands is typed
   (`KeptItself`, `Stands`). Where core says the folder stands somewhere else by now, or the change was not
   read, the note stays sentences and the told note stays beside it: a picture of sheets in the folder would
   claim more than core does.

## Open

- **A keep or a toss done by a tap is still said in words.** Core's answer to a tap is not typed: it is `WorkDone::Said` for
  a done keep, a done toss and a refusal alike, and the app must not tell them apart by their sentences. The
  picture's `Tossed` state, and `In` for a keep by a tap, wait for a typed answer from core.
- **No step's file is drawn.** Core keeps the names; the track's `file` stays empty until a rule says which name
  a page may show.
- **`Bars` has no data.** Core counts no lines.
- **Nobody has seen it in a real window.** The pictures of the snapshot example are the only look. Whether a
  person finds the tap, and whether a closed note says enough, is not known.
- The print of `layout` is not what a window first shows: it is the page with every picture open.
- A picture's parts have no words for a screen reader.
- Windows and Linux were not run.
