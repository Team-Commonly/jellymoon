//! A tap on a day's page, drawn (F35). The rules are core's (`jellymoon_core::settle`, `turn_on_delivery`): which
//! statement stands, when a tap makes no ruling, what is kept and in which order. Here a tap is only carried to core
//! and the page is drawn again from what core says. The statement a tap sends is the one whose drawn text was
//! tapped: the option's own string, compared with core's, so a page that changed under the person sends nothing.
//! No GPUI: the window holds an [`OnShow`] and calls [`OnShow::tap`].

use jellymoon_core::{CLASHES_MAX, HookState, Settled};
use sticker_kit::{Note, Scene};

use crate::read::Page;
use crate::scene;
use crate::today::{Next, Today};

/// The id of the note that turns delivery on. A clash's note is `clash-<n>`.
pub const DELIVERY: &str = "delivery";

/// How to turn delivery on when the app cannot do it.
const BY_HAND: &str = "run `jellymoon hook install` in a terminal";

/// What a tap did.
#[derive(Debug)]
pub enum Done {
    /// Core's answer to a tap on a clash: the ruling it made, or none when that statement stands already.
    Settled(Settled),
    /// Delivery was turned on.
    Delivery(HookState),
}

/// A day's page on show in the window: the page data and the scene drawn from it, which a tap acts on.
pub struct OnShow {
    pub today: Today,
    pub page: Option<Page>,
    pub scene: Scene,
}

impl OnShow {
    /// The reading gave something new to show.
    pub fn show(&mut self, next: &Next) {
        (self.page, self.scene) = (next.page.clone(), next.scene.clone());
    }

    /// A tap on option `index` (from 0) of the note `note`: the scene to show now. None when no page is on show.
    pub fn tap(&mut self, note: &str, index: usize) -> Option<Scene> {
        let page = self.page.as_ref()?;
        self.scene = self.today.after_tap(page, &self.scene, note, index);
        Some(self.scene.clone())
    }
}

impl Today {
    /// The page as it is shown: each clash in the words a ruling would carry, with the side that stands by the
    /// person's own ruling, and, when the page has a clash and delivery is off, one note before the clashes that
    /// says so.
    pub fn draw(&self, page: &Page) -> Scene {
        let standing = jellymoon_core::standing(&self.home, &self.date, &page.data.clashes);
        let mut shown = scene::ruled(page, &standing);
        if !page.data.clashes.is_empty() && !jellymoon_core::hook_installed(&self.home) {
            // Before the clashes, so a full page never shows a clash to tap and hides that no session would hear.
            let clash = |note: &Note| matches!(note, Note::Choice { id, .. } if id.starts_with("clash-"));
            let at = shown.notes.iter().position(clash).unwrap_or(shown.notes.len());
            shown.notes.insert(at, self.delivery_note());
        }
        shown
    }

    /// The note of a page whose rulings would reach no session: one option to tap when core takes the installed
    /// `jellymoon` command and the settings file has a full path, else how to do it from a terminal.
    fn delivery_note(&self) -> Note {
        let command = self.bin.as_ref().is_some_and(|bin| jellymoon_core::installed_command(bin).is_ok());
        if command && self.settings.as_ref().is_some_and(|settings| settings.is_absolute()) {
            return Note::Choice {
                id: DELIVERY.to_string(),
                title: "your sessions are not told yet".to_string(),
                options: vec!["Turn on delivery".to_string()],
                footer: "adds one line to Claude Code's settings".to_string(),
                struck_first: false,
                whole: false,
            };
        }
        let body = format!("To turn delivery on, {BY_HAND}.");
        Note::Plain { title: "Your sessions are not told yet.".to_string(), body }
    }

    /// What a tap on option `index` (from 0) of the note `note` of the scene `drawn` does. A refusal has written
    /// nothing.
    pub fn tapped(&self, page: &Page, drawn: &Scene, note: &str, index: usize) -> Result<Done, String> {
        let none = || format!("the page of {} has no {note:?} to tap", self.date);
        let options = drawn.notes.iter().find_map(|one| match one {
            Note::Choice { id, options, .. } if id == note => Some(options),
            _ => None,
        });
        let tapped = options.ok_or_else(none)?.get(index);
        let tapped = tapped.ok_or_else(|| format!("{note:?} has no option {}", index + 1))?;
        if note == DELIVERY {
            let (Some(settings), Some(bin)) = (&self.settings, &self.bin) else {
                return Err(format!("no home directory: {BY_HAND}"));
            };
            let on = jellymoon_core::turn_on_delivery(settings, &self.home, bin);
            return on.map(Done::Delivery).map_err(|e| format!("{e}: {BY_HAND}"));
        }
        let clashes = &page.data.clashes;
        let clash = (1..=clashes.len().min(CLASHES_MAX)).find(|n| note == format!("clash-{n}")).ok_or_else(none)?;
        // The statement that stands is the one whose drawn words were tapped, as core words it now.
        let now = jellymoon_core::standing(&self.home, &self.date, &clashes[clash - 1..clash]);
        let option = match now.first() {
            Some(now) if now.a == now.b => None,
            Some(now) if *tapped == scene::quoted(&now.a) => Some(1),
            Some(now) if *tapped == scene::quoted(&now.b) => Some(2),
            _ => None,
        };
        let option = option.ok_or("the page has changed since it was drawn, so nothing was sent")?;
        let settled = jellymoon_core::settle(&self.home, &self.date, clashes, clash, option);
        settled.map(Done::Settled).map_err(|e| e.to_string())
    }

    /// The page after a tap on the scene `drawn`. A tap that did nothing says why in the first note; a ruling the
    /// page could not keep says that.
    pub fn after_tap(&self, page: &Page, drawn: &Scene, note: &str, index: usize) -> Scene {
        let said = match self.tapped(page, drawn, note, index) {
            Err(why) => Some(("That tap did nothing.", format!("{why}."))),
            Ok(Done::Settled(Settled { unkept: Some(why), .. })) => Some((
                "This page could not keep your choice.",
                format!(
                    "Your choice stands and waits for your sessions. Tapping it again sends nothing twice ({why})."
                ),
            )),
            Ok(_) => None,
        };
        let mut shown = self.draw(page);
        if let Some((title, body)) = said {
            shown.notes.insert(0, Note::Plain { title: title.to_string(), body });
        }
        shown
    }
}
