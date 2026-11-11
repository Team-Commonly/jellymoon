//! What a tap on a day's page does (F35). A tap on a clash's option makes one ruling through core, of the clash's two
//! statements as the page data holds them, and the page keeps which one stands. A tap on "Turn on delivery" installs
//! core's hook. Nothing else here writes, and no other text reaches a ruling. The window and the `tap` command both
//! go through [`Today::tapped`]. No GPUI.
//!
//! Which side stands is kept beside the day's page, in `pages/<day>.ruled.json`. It names a clash by its two
//! statements, so it holds for exactly the clash it was made on: a later reading that words the clash differently
//! shows it open again. The file comes back through a door like the kept page's: a size, a shape, the day.

use std::io::Read;
use std::path::PathBuf;
use std::{fs, io};

use jellymoon_core::{Clash, HookState, Ruling};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sticker_kit::{Note, Scene};

use crate::read::{MAX_CLASHES, Page};
use crate::scene::{self, Side};
use crate::today::{Today, write_private};

/// The id of the note that turns delivery on. A clash's note is `clash-<n>`.
pub const DELIVERY: &str = "delivery";

/// The most bytes of a day's rulings file that are read. Eight clashes of two 200-character statements are far less.
const MAX_RULED_BYTES: usize = 64 * 1024;

/// How to turn delivery on when the app cannot do it.
const BY_HAND: &str = "run `jellymoon hook install` in a terminal";

/// The person's rulings on one day's clashes.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Ruled {
    day: String,
    clashes: Vec<One>,
}

/// One clash, by its two statements as the page data held them, and the side the person chose.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct One {
    a: String,
    b: String,
    stands: Side,
}

impl One {
    fn is(&self, clash: &Clash) -> bool {
        self.a == clash.quote_a && self.b == clash.quote_b
    }
}

/// What a tap did.
#[derive(Debug)]
pub enum Done {
    /// A ruling was made and waits for the sessions.
    Ruled(Ruling),
    /// The option tapped already stands by the person's own ruling: nothing was made.
    Stays,
    /// Delivery was turned on.
    Delivery(HookState),
}

/// A tap asked for on the command line. `clash` and `option` count from 1, the options in the order they are drawn.
#[derive(Debug)]
pub enum What {
    Clash { clash: usize, option: usize },
    DeliveryOn,
}

impl Today {
    fn ruled_file(&self) -> PathBuf {
        self.home.join("pages").join(format!("{}.ruled.json", self.date))
    }

    /// The day's kept rulings. A file that is missing, too large, not of this shape or of another day is none.
    fn ruled(&self) -> Vec<One> {
        let mut bytes = Vec::new();
        let read = fs::File::open(self.ruled_file())
            .and_then(|file| file.take(MAX_RULED_BYTES as u64 + 1).read_to_end(&mut bytes));
        if read.is_err() || bytes.len() > MAX_RULED_BYTES {
            return Vec::new();
        }
        let ruled = serde_json::from_slice::<Ruled>(&bytes).ok().filter(|ruled| ruled.day == self.date);
        ruled.map(|ruled| ruled.clashes).filter(|clashes| clashes.len() <= MAX_CLASHES).unwrap_or_default()
    }

    /// The page as it is shown: each clash the person ruled on with the side that stands, and, when the page has a
    /// clash and delivery is off, one note before the clashes that says so.
    pub fn draw(&self, page: &Page) -> Scene {
        let ruled = self.ruled();
        let stands = |clash: &Clash| ruled.iter().find(|one| one.is(clash)).map(|one| one.stands);
        let stands: Vec<Option<Side>> = page.data.clashes.iter().map(stands).collect();
        let mut shown = scene::ruled(page, &stands);
        if !page.data.clashes.is_empty() && !jellymoon_core::hook_installed(&self.home) {
            // Before the clashes, so a full page never shows a clash to tap and hides that no session would hear.
            let clash = |note: &Note| matches!(note, Note::Choice { id, .. } if id.starts_with("clash-"));
            let at = shown.notes.iter().position(clash).unwrap_or(shown.notes.len());
            shown.notes.insert(at, self.delivery_note());
        }
        shown
    }

    /// The note of a page whose rulings would reach no session: one option to tap when the app knows where the
    /// `jellymoon` command and the harness settings are, else how to do it from a terminal.
    fn delivery_note(&self) -> Note {
        if self.command.is_some() && self.settings.is_some() {
            return Note::Choice {
                id: DELIVERY.to_string(),
                title: "your sessions are not told yet".to_string(),
                options: vec!["Turn on delivery".to_string()],
                footer: "adds one line to Claude Code's settings".to_string(),
                struck_first: false,
            };
        }
        let body = format!("To turn delivery on, {BY_HAND}.");
        Note::Plain { title: "Your sessions are not told yet.".to_string(), body }
    }

    /// What a tap on option `index` (from 0, as drawn) of the note `note` does. A refusal has written nothing.
    pub fn tapped(&self, page: &Page, note: &str, index: usize) -> Result<Done, String> {
        if note == DELIVERY && index == 0 {
            return self.delivery_on().map(Done::Delivery);
        }
        let number = note.strip_prefix("clash-").and_then(|number| number.parse::<usize>().ok());
        let clash = number.and_then(|number| page.data.clashes.get(number.checked_sub(1)?));
        let clash = clash.ok_or_else(|| format!("the page of {} has no {note:?} to tap", self.date))?;
        let mut ruled = self.ruled();
        let now = ruled.iter().find(|one| one.is(clash)).map(|one| one.stands);
        let side = scene::order(now).get(index).copied();
        let side = side.ok_or_else(|| format!("a clash has two options, and option {} was tapped", index + 1))?;
        if now == Some(side) {
            return Ok(Done::Stays);
        }
        let (stands, over) = match side {
            Side::A => (&clash.quote_a, &clash.quote_b),
            Side::B => (&clash.quote_b, &clash.quote_a),
        };
        // Core makes each statement one line, replaces credential shapes and cuts it; nothing is added here.
        let ruling = jellymoon_core::rule(&self.home, stands, over).map_err(|e| e.to_string())?;
        // Only the clashes of the page as it is now are kept, this one with its new side.
        ruled.retain(|one| !one.is(clash) && page.data.clashes.iter().any(|clash| one.is(clash)));
        ruled.push(One { a: clash.quote_a.clone(), b: clash.quote_b.clone(), stands: side });
        self.keep_ruled(ruled).map_err(|e| format!("the ruling was made, but the page could not keep it: {e}"))?;
        Ok(Done::Ruled(ruling))
    }

    fn keep_ruled(&self, clashes: Vec<One>) -> io::Result<()> {
        let ruled = Ruled { day: self.date.clone(), clashes };
        write_private(&self.ruled_file(), &format!("{}\n", serde_json::to_string(&ruled)?))
    }

    /// Installs core's hook, as `jellymoon hook install` does, for the installed `jellymoon` command.
    fn delivery_on(&self) -> Result<HookState, String> {
        let (Some(command), Some(settings)) = (&self.command, &self.settings) else {
            return Err(format!("the jellymoon command was not found: {BY_HAND}"));
        };
        jellymoon_core::install_hook(settings, &self.home, command).map_err(|e| e.to_string())
    }

    /// The page after a tap on it, for the window. A tap that did nothing says why in its first note.
    pub fn after_tap(&self, page: &Page, note: &str, index: usize) -> Scene {
        let failed = self.tapped(page, note, index).err();
        let mut shown = self.draw(page);
        if let Some(why) = failed {
            shown.notes.insert(0, Note::Plain { title: "That tap did nothing.".to_string(), body: format!("{why}.") });
        }
        shown
    }

    /// The `tap` command: the same tap, on the day's kept page, and one line of JSON that says what it did.
    pub fn act(&self, what: &What) -> Result<String, String> {
        let done = match what {
            What::DeliveryOn => Done::Delivery(self.delivery_on()?),
            What::Clash { clash, option } => {
                let page = self.kept().ok_or_else(|| format!("no page is kept for {}", self.date))?;
                let index = option.checked_sub(1).ok_or("options count from 1")?;
                self.tapped(&page, &format!("clash-{clash}"), index)?
            }
        };
        let said = match done {
            Done::Ruled(ruling) => json!({ "ruling": ruling }),
            Done::Stays => json!({ "ruling": null }),
            Done::Delivery(state) => json!(state),
        };
        Ok(format!("{said}\n"))
    }
}
