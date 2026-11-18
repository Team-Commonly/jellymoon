//! The page's window: the kit's view of one scene, and of the scene that replaces it when a day has been read. `t`
//! switches between the two built-in themes. A tap is written to stderr as one line: the piece's id, and the option
//! when it was one, never a word of the page. A tap on an option of a day's page then goes to `taps.rs`, and the
//! page it gives is shown. Enter in a day's entry goes to `wishing.rs`, and a tap on a wish's character or on the
//! choice to stop its run to `stopping.rs`: core is asked off this thread, and the page is shown as soon as it has
//! answered. Once a second the day is looked at again, so a run's end is drawn with no tap. A folder dropped on
//! the page, or chosen with the system's picker from the entry's "Choose a folder", and a name entered in the
//! entry of a day with no project on offer, go to `starting.rs`, off the window's thread too.
//!
//! This file is wiring. What each act does is `showing.rs`, which a test calls with no window: here its work is
//! run off the window's thread, and each scene it gives is handed to the kit's view. The view outlives its window
//! (`shell.rs`): what is done here is done to the view, so a page that is closed and shown again is the page it
//! was. No test opens this window on a screen: `tests/f43/window.rs` opens [`page`] on GPUI's test platform and
//! presses Enter in it, `tests/f54/window.rs` presses Enter in a note's field and closes the window before core
//! has answered, and `tests/f57/window.rs` opens all of `shell.rs` there.
//!
//! The view measures text with GPUI's shaped glyphs, not with the bundled advances the `layout` command uses: shaping
//! kerns, and the plain theme draws in the system's font (`docs/development/app.md` has the measured gap).

use std::cell::OnceCell;
use std::io::Write;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    App, Bounds, Entity, KeyBinding, Menu, MenuItem, PathPromptOptions, Pixels, QuitMode, TitlebarOptions, WeakEntity,
    WindowBounds, WindowHandle, WindowOptions, actions, prelude::*, px, size,
};
use gpui_platform::application;
use sticker_kit::draw::{EntryMade, Page, Tap, load_fonts};
use sticker_kit::{Scene, Theme, Viewport};

use crate::jellyfish::Room;
use crate::shell::{Asked, Opened, wire};
use crate::showing::{Entered, Showing, entry_line, other_theme, switches_theme, tap_line};
use crate::starting::{Given, Listing, NEW, Shown};
use crate::today::Later;

actions!(jellymoon, [Quit]);

/// How often a day on show is looked at again, and what core keeps for the jellyfish.
pub(crate) const LOOK_EVERY: Duration = Duration::from_secs(1);

/// Opens the page's window and the jellyfish, and runs until Quit, or until `asked.quit_after` has passed. Closing
/// the page's window ends nothing: opening the app again shows it, as a tap on the jellyfish does. A day's `later`
/// is started at once, off the window's thread; each scene it gives replaces the one shown, and the work after it
/// starts. A page-data file has no day: it is drawn, and a tap on it does nothing.
pub fn open(asked: Asked) {
    // What is opened is known once the app runs; opening the app again may be told before or after.
    let slot: Rc<OnceCell<Opened>> = Rc::default();
    let (again, app) = (slot.clone(), application());
    app.on_reopen(move |cx| {
        if let Some(opened) = again.get() {
            opened.show_page(cx);
        }
    });
    app.run(move |cx: &mut App| {
        if let Err(error) = load_fonts(cx) {
            say(&format!("the bundled fonts did not load: {error}"));
        }
        let opened = match wire(asked, Room::primary(cx), cx) {
            Ok(opened) => opened,
            Err(error) => {
                say(&format!("no window: {error}"));
                std::process::exit(1);
            }
        };
        let _ = slot.set(opened.clone());
        cx.set_quit_mode(QuitMode::Explicit);
        cx.on_action(move |_: &Quit, cx| opened.quit(cx));
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        let items = vec![MenuItem::action("Quit Jellymoon", Quit)];
        cx.set_menus(vec![Menu { name: "Jellymoon".into(), items, disabled: false }]);
        cx.activate(true);
    });
}

/// A key pressed in a window: in the page's, the one that switches the theme does.
pub(crate) fn key(key: &str, window: &mut gpui::Window, cx: &mut App) {
    let Some(Some(page)) = window.root::<Page>() else { return };
    page.update(cx, |page, cx| {
        if switches_theme(key, page.entry_focused(window)) {
            page.set_theme(other_theme(&page.theme().name), cx);
        }
    });
}

/// The entry of the first page, then each scene the day's reading gives with the entry of its page. Which
/// projects have a chip is asked of the session logs, so all of it is off the window's thread: the first page has
/// no entry until core has answered.
pub(crate) async fn reads(shown: Showing, later: Option<Later>, page: WeakEntity<Page>, cx: &mut gpui::AsyncApp) {
    let Some(first) = shown.offering() else { return };
    let entry = cx.background_executor().spawn(async move { first() }).await;
    let _ = page.update(cx, |page, cx| page.set_entry(entry, cx));
    cx.update(|cx| choose_waiting(&shown, &page, cx));
    let mut later = later;
    while let Some(work) = later.take().and_then(|step| shown.reading(step)) {
        let Some((next, entry)) = cx.background_executor().spawn(async move { work() }).await else { break };
        let Some(scene) = shown.read(&next) else { return };
        let _ = page.update(cx, |page, cx| {
            page.set_entry(entry, cx);
            page.set_scene(scene, cx);
        });
        cx.update(|cx| choose_waiting(&shown, &page, cx));
        later = next.later;
    }
}

/// Looks at the day on show once a second, off the window's thread, and shows what a look gives when it differs
/// from what is shown: a run that ended, a wish made in a terminal. It goes on while the page's window is closed,
/// and ends with the app, or at once for a page with no day.
pub(crate) async fn looks(shown: Showing, page: WeakEntity<Page>, cx: &mut gpui::AsyncApp) {
    loop {
        cx.background_executor().timer(LOOK_EVERY).await;
        let Some(look) = shown.look() else { return };
        let Some(seen) = cx.background_executor().spawn(async move { look() }).await else { continue };
        let Some(scene) = shown.seen(seen) else { continue };
        if page.update(cx, |page, cx| page.set_scene(scene, cx)).is_err() {
            return;
        }
    }
}

/// Enter in the entry, or in a note's field. It keeps its words and waits while core is asked off the window's
/// thread; then it is answered and the page shown. Then the asking for the wish's run is run, off the window's
/// thread too, and what it has to say is shown when it ends.
fn entered(made: &EntryMade, shown: &Showing, page: WeakEntity<Page>, cx: &mut App) {
    say(&entry_line(&made.id, &made.text));
    if made.id == NEW {
        return listing(shown.listing(Given::Name(made.text.clone())), true, shown.clone(), page, cx);
    }
    // What is typed under a made wish (F54) is carried as a wish is, and answers the field it was typed in.
    let field = (made.id != crate::wishing::ENTRY).then(|| made.id.clone());
    let (at_once, wishing) = match &field {
        Some(note) => shown.typing(note, &made.text).unzip(),
        None => (None, shown.wishing(&made.text, made.option.as_deref())),
    };
    // An answer's note changes at once (`answering.rs`). The view is in the middle of handling the key: it
    // takes the scene once that is done.
    if let Some(scene) = at_once.flatten() {
        let now = page.clone();
        cx.defer(move |cx| {
            let _ = now.update(cx, |page, cx| page.set_scene(scene, cx));
        });
    }
    let shown = shown.clone();
    cx.spawn(async move |cx| {
        let answer = match wishing {
            Some(wishing) => Some(cx.background_executor().spawn(async move { wishing() }).await),
            None => None,
        };
        let Entered { kept, scene, asking } = shown.answered(answer);
        let answered = page.clone();
        cx.update(|cx| answer_entry(&answered, field.as_deref(), kept, scene, cx));
        let Some(asking) = asking else { return };
        let said = cx.background_executor().spawn(async move { asking() }).await;
        let Some(scene) = shown.asked(said) else { return };
        let _ = page.update(cx, |page, cx| page.set_scene(scene, cx));
    })
    .detach();
}

/// The entry, or the field of the note `field` (F54), is answered in the page's window, which the platform's
/// text input is told in; with the window closed since Enter it is answered all the same, so the page shown
/// again does not wait for ever.
fn answer_entry(page: &WeakEntity<Page>, field: Option<&str>, kept: bool, scene: Option<Scene>, cx: &mut App) {
    let Some(page) = page.upgrade() else { return };
    match (page_window(cx), field) {
        (Some(open), Some(note)) => {
            let _ = open.update(cx, |page, window, cx| page.answer_field(note, kept, window, cx));
        }
        (Some(open), None) => {
            let _ = open.update(cx, |page, window, cx| page.answer_entry(kept, window, cx));
        }
        (None, Some(note)) => page.update(cx, |page, cx| page.answer_field_closed(note, kept, cx)),
        (None, None) => page.update(cx, |page, cx| page.answer_entry_closed(kept, cx)),
    }
    if let Some(scene) = scene {
        page.update(cx, |page, cx| page.set_scene(scene, cx));
    }
}

/// A folder dropped on the page, or chosen with the picker: the first one is given to core.
fn given(paths: &[PathBuf], shown: &Showing, page: WeakEntity<Page>, cx: &mut App) {
    if let Some(folder) = paths.first() {
        listing(shown.listing(Given::Folder(folder.clone())), false, shown.clone(), page, cx);
    }
}

/// The system's folder picker, for one folder: what is chosen goes where a drop goes. Cancelled, it is nothing.
fn choosing(shown: &Showing, page: WeakEntity<Page>, cx: &mut App) {
    let options = PathPromptOptions { files: false, directories: true, multiple: false, prompt: None };
    let (picked, shown) = (cx.prompt_for_paths(options), shown.clone());
    cx.spawn(async move |cx| {
        let Ok(Ok(Some(paths))) = picked.await else { return };
        cx.update(|cx| given(&paths, &shown, page, cx));
    })
    .detach();
}

/// Core is asked off the window's thread at once, while the day is still being read too; then the page shows its
/// answer and the entry the page has now, with the project's chip chosen and the keys. With no entry for it yet,
/// the entry is asked for again, and the chip is chosen when the reading gives a page whose entry offers it.
/// `named`: the name was entered in the entry, which is answered first.
fn listing(work: Option<Listing>, named: bool, shown: Showing, page: WeakEntity<Page>, cx: &mut App) {
    cx.spawn(async move |cx| {
        let listed = match work {
            Some(work) => Some(cx.background_executor().spawn(async move { work() }).await),
            None => None,
        };
        let now = listed.and_then(|listed| shown.listed(listed));
        cx.update(|cx| {
            let kept = now.as_ref().is_some_and(|now| now.name.is_some());
            if named {
                answer_entry(&page, None, kept, None, cx);
            }
            let (Some(Shown { scene, entry, name }), Some(view)) = (now, page.upgrade()) else { return };
            shown.wait_for(name);
            view.update(cx, |page, cx| {
                page.set_entry(entry, cx);
                page.set_scene(scene, cx);
            });
            choose_waiting(&shown, &page, cx);
        });
        let Some(offering) = shown.waiting().and(shown.offering()) else { return };
        let entry = cx.background_executor().spawn(async move { offering() }).await;
        let _ = page.update(cx, |page, cx| page.set_entry(entry, cx));
        cx.update(|cx| choose_waiting(&shown, &page, cx));
    })
    .detach();
}

/// The chip of the project core listed is chosen once an entry offers it, and the entry has the keys.
fn choose_waiting(shown: &Showing, page: &WeakEntity<Page>, cx: &mut App) {
    let (Some(name), Some(view)) = (shown.waiting(), page.upgrade()) else { return };
    if !view.update(cx, |page, cx| page.choose_option(&name, cx)) {
        return;
    }
    shown.wait_for(None);
    if let Some(open) = page_window(cx) {
        let _ = open.update(cx, |page, window, cx| page.focus_entry(window, cx));
    }
}

/// A tap on a character: core is asked off the window's thread, and what it gives is shown.
fn poked(id: &str, shown: &Showing, page: WeakEntity<Page>, cx: &mut App) {
    let (Some(look), shown) = (shown.poked(id), shown.clone()) else { return };
    cx.spawn(async move |cx| {
        let Some(seen) = cx.background_executor().spawn(async move { look() }).await else { return };
        let Some(scene) = shown.seen(seen) else { return };
        let _ = page.update(cx, |page, cx| page.set_scene(scene, cx));
    })
    .detach();
}

/// A tap on an option: its scene is shown at once, and what it left to do (a stop, which waits for the run to
/// end, or a show, a keep or a toss of work that waits) is run off the window's thread.
fn chose(note: &str, index: usize, shown: &Showing, page: WeakEntity<Page>, cx: &mut App) {
    let Some((after, left)) = shown.chose(note, index) else { return };
    // The view is in the middle of handling the tap: it takes the new scene once that is done.
    let now = page.clone();
    cx.defer(move |cx| {
        let _ = now.update(cx, |page, cx| page.set_scene(after, cx));
    });
    let (Some(left), shown) = (left, shown.clone()) else { return };
    cx.spawn(async move |cx| {
        let ended = cx.background_executor().spawn(async move { left.run() }).await;
        let Some(scene) = shown.ended(ended) else { return };
        let _ = page.update(cx, |page, cx| page.set_scene(scene, cx));
    })
    .detach();
}

/// The view of one page, its taps and its Enter wired to `shown`. It is its window's root, and outlives it.
pub(crate) fn view(scene: Scene, theme: Theme, shown: Showing, cx: &mut App) -> Entity<Page> {
    cx.new(|cx| {
        let (page, on_show, on_show_drop) = (cx.weak_entity(), shown.clone(), shown.clone());
        let entry_of = page.clone();
        let tapped = move |tap: &Tap, _: &mut gpui::Window, cx: &mut App| {
            let (id, option) = match tap {
                Tap::Character { id } => (id, None),
                Tap::Option { note_id, index } => (note_id, Some(*index)),
                Tap::Action { id } => {
                    say(&tap_line(id, None));
                    return choosing(&on_show, page.clone(), cx);
                }
            };
            say(&tap_line(id, option));
            match option {
                None => poked(id, &on_show, page.clone(), cx),
                Some(index) => chose(id, index, &on_show, page.clone(), cx),
            }
        };
        let made = move |made: &EntryMade, _: &mut gpui::Window, cx: &mut App| {
            entered(made, &shown, entry_of.clone(), cx);
        };
        let (dropped_on, dropped) = (cx.weak_entity(), on_show_drop);
        let paths = move |paths: &[PathBuf], _: &mut gpui::Window, cx: &mut App| {
            given(paths, &dropped, dropped_on.clone(), cx);
        };
        // The entry comes with the first answer of `reads`.
        Page::new(scene, theme).on_tap(tapped).on_entry(made).on_paths(paths)
    })
}

/// Opens a window on `view`, in the middle of the screen.
pub(crate) fn show(view: Entity<Page>, viewport: Viewport, cx: &mut App) -> gpui::Result<WindowHandle<Page>> {
    let bounds = Bounds::centered(None, size(px(viewport.w), px(viewport.h)), cx);
    cx.open_window(options(bounds), move |_, _| view)
}

/// The window of one page, its taps and its Enter wired to `shown`. A test opens it on GPUI's test platform,
/// where there is no screen.
pub fn page(
    scene: Scene,
    theme: Theme,
    viewport: Viewport,
    shown: Showing,
    cx: &mut App,
) -> gpui::Result<WindowHandle<Page>> {
    let view = view(scene, theme, shown, cx);
    show(view, viewport, cx)
}

/// The page's window, while one is open.
pub fn page_window(cx: &App) -> Option<WindowHandle<Page>> {
    cx.windows().into_iter().find_map(|window| window.downcast::<Page>())
}

/// What the window is asked for: its first place, a title, and never to be smaller than a page is drawn at.
fn options(bounds: Bounds<Pixels>) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        window_min_size: Some(size(px(crate::SMALLEST.w), px(crate::SMALLEST.h))),
        titlebar: Some(TitlebarOptions { title: Some("Jellymoon".into()), ..Default::default() }),
        ..Default::default()
    }
}

pub(crate) fn say(line: &str) {
    let _ = std::io::stderr().write_all(format!("jellymoon-app: {line}\n").as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// This holds what the window is asked for. That the system honours it was seen by hand on the Mac only.
    #[test]
    fn the_window_is_asked_never_to_be_smaller_than_the_smallest_page() {
        let asked = options(Bounds::default());
        assert_eq!(asked.window_min_size, Some(size(px(960.0), px(640.0))));
        assert_eq!((crate::SMALLEST.w, crate::SMALLEST.h), (960.0, 640.0));
    }
}
