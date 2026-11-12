//! The window: the kit's view of one scene, and of the scene that replaces it when a day has been read. `t` switches
//! between the two built-in themes. A tap is written to stderr as one line: the piece's id, and the option when it
//! was one, never a word of the page. A tap on an option of a day's page then goes to `taps.rs`, and the page it
//! gives is shown. Enter in a day's entry goes to `wishing.rs`, and a tap on a wish's character or on the choice to
//! stop its run to `stopping.rs`: core is asked off this thread, and the page is shown as soon as it has answered.
//! Once a second the day is looked at again, so a run's end is drawn with no tap.
//!
//! The view measures text with GPUI's shaped glyphs, not with the bundled advances the `layout` command uses: shaping
//! kerns, and the plain theme draws in the system's font (`docs/development/app.md` has the measured gap).

use std::cell::{Cell, RefCell};
use std::io::Write;
use std::rc::Rc;
use std::time::Duration;

use gpui::{App, Bounds, Pixels, TitlebarOptions, WindowBounds, WindowHandle, WindowOptions, prelude::*, px, size};
use gpui_platform::application;
use sticker_kit::draw::{EntryMade, Page, Tap, load_fonts};
use sticker_kit::{Scene, Theme, Viewport};

use crate::stopping::Chosen;
use crate::taps::OnShow;
use crate::today::Later;

/// The day's page on show, which the tap handler and the entry act on and the reading and a look replace. All run
/// on the window's thread.
type Shown = Rc<RefCell<Option<OnShow>>>;

/// How often a day on show is looked at again.
const LOOK_EVERY: Duration = Duration::from_secs(1);

/// Opens the window on `scene` and runs until it is closed, or until `quit_after` has passed. A day's `later` is
/// started at once, off the window's thread; each scene it gives replaces the one shown, and the work after it
/// starts. A page-data file has no day: it is drawn, and a tap on it does nothing.
pub fn open(
    scene: Scene,
    theme: Theme,
    viewport: Viewport,
    quit_after: Option<Duration>,
    day: Option<(OnShow, Later)>,
) {
    application().run(move |cx: &mut App| {
        if let Err(error) = load_fonts(cx) {
            say(&format!("the bundled fonts did not load: {error}"));
        }
        let (shown, later) = day.unzip();
        let shown: Shown = Rc::new(RefCell::new(shown));
        let window = match page(scene, theme, viewport, shown.clone(), cx) {
            Ok(window) => window,
            Err(error) => {
                say(&format!("no window: {error}"));
                std::process::exit(1);
            }
        };
        cx.on_window_closed(|cx, _| cx.quit()).detach();
        cx.observe_keystrokes(move |event, _, cx| theme_key(&event.keystroke.key, window, cx)).detach();
        let reading = shown.clone();
        cx.spawn(async move |cx| reads(reading, later, window, cx).await).detach();
        cx.spawn(async move |cx| looks(shown, window, cx).await).detach();
        if let Some(limit) = quit_after {
            cx.spawn(async move |cx| {
                cx.background_executor().timer(limit).await;
                cx.update(|cx| cx.quit());
            })
            .detach();
        }
        cx.activate(true);
    });
}

/// `t` switches the theme, unless it was typed in the entry, where it is a letter of a wish.
fn theme_key(key: &str, window: WindowHandle<Page>, cx: &mut App) {
    if key == "t" {
        let _ = window.update(cx, |page, window, cx| {
            if !page.entry_focused(window) {
                switch_theme(page, cx);
            }
        });
    }
}

/// The entry of the first page, then each scene the day's reading gives with the entry of its page. Which
/// projects have a chip is asked of the session logs, so all of it is off the window's thread: the first page has
/// no entry until core has answered.
async fn reads(shown: Shown, later: Option<Later>, window: WindowHandle<Page>, cx: &mut gpui::AsyncApp) {
    let Some((today, first)) = shown.borrow().as_ref().map(|shown| (shown.today.clone(), shown.offering())) else {
        return;
    };
    let entry = cx.background_executor().spawn(async move { first() }).await;
    let _ = window.update(cx, |page, _, cx| page.set_entry(entry, cx));
    let mut later = later;
    while let Some(work) = later.take() {
        let work = today.clone().offered(work);
        let Some((next, entry)) = cx.background_executor().spawn(async move { work() }).await else { break };
        // The notes of a wish or a stop stay over what the reading gave.
        let Some(scene) = shown.borrow_mut().as_mut().map(|shown| shown.show(&next)) else { return };
        let _ = window.update(cx, |page, _, cx| {
            page.set_entry(entry, cx);
            page.set_scene(scene, cx);
        });
        later = next.later;
    }
}

/// Looks at the day on show once a second, off the window's thread, and shows what a look gives when it differs
/// from what is shown: a run that ended, a wish made in a terminal. It ends with the window, or at once for a page
/// with no day.
async fn looks(shown: Shown, window: WindowHandle<Page>, cx: &mut gpui::AsyncApp) {
    loop {
        cx.background_executor().timer(LOOK_EVERY).await;
        let Some(look) = shown.borrow().as_ref().map(OnShow::look) else { return };
        let Some(seen) = cx.background_executor().spawn(async move { look() }).await else { continue };
        let Some(scene) = shown.borrow_mut().as_mut().and_then(|shown| shown.seen(seen)) else { continue };
        if window.update(cx, |page, _, cx| page.set_scene(scene, cx)).is_err() {
            return;
        }
    }
}

/// Enter in the entry: the wish goes to `wishing.rs`. The entry keeps its words and waits. Core is asked off the
/// window's thread, and the page is shown as soon as it has answered: with the wish and an empty entry, or with why
/// it was not kept and the words still in the entry. Then the asking for the wish's run is run, off the window's
/// thread too, and what it has to say is shown when it ends.
fn entered(made: &EntryMade, shown: &Shown, window: WindowHandle<Page>, cx: &mut App) {
    say(&format!("entry id={} characters={}", made.id, made.text.chars().count()));
    let wishing = shown.borrow().as_ref().map(|shown| shown.wishing(&made.text, made.option.as_deref()));
    let shown = shown.clone();
    cx.spawn(async move |cx| {
        let answer = match wishing {
            Some(wishing) => Some(cx.background_executor().spawn(async move { wishing() }).await),
            None => None,
        };
        let answered = answer.and_then(|answer| shown.borrow_mut().as_mut().map(|shown| shown.answered(answer)));
        // The entry is always answered, so it never waits for ever: with nothing answered its words stay.
        let (scene, asking) = answered.unzip();
        let kept = asking.as_ref().is_some_and(Option::is_some);
        let _ = window.update(cx, |page, window, cx| {
            page.answer_entry(kept, window, cx);
            if let Some(scene) = scene {
                page.set_scene(scene, cx);
            }
        });
        let Some(asking) = asking.flatten() else { return };
        let said = cx.background_executor().spawn(async move { asking() }).await;
        let Some(scene) = shown.borrow_mut().as_mut().and_then(|shown| shown.asked(said)) else { return };
        let _ = window.update(cx, |page, _, cx| page.set_scene(scene, cx));
    })
    .detach();
}

/// A tap on a character: when core says it is a made wish with a run going, the choice to stop it is shown. Core is
/// asked off the window's thread.
fn poked(id: &str, shown: &Shown, window: WindowHandle<Page>, cx: &mut App) {
    let Some(look) = shown.borrow().as_ref().map(|shown| shown.poked(id)) else { return };
    let shown = shown.clone();
    cx.spawn(async move |cx| {
        let Some(seen) = cx.background_executor().spawn(async move { look() }).await else { return };
        let Some(scene) = shown.borrow_mut().as_mut().and_then(|shown| shown.seen(seen)) else { return };
        let _ = window.update(cx, |page, _, cx| page.set_scene(scene, cx));
    })
    .detach();
}

/// A tap on an option. The choice to stop a run says so at once, and the stop, which waits for the run to end, is
/// run off the window's thread. Any other option is a tap for `taps.rs`, and a choice to stop that is no longer on
/// show is nothing.
fn chose(note: &str, index: usize, shown: &Shown, window: WindowHandle<Page>, cx: &mut App) {
    let chosen = shown.borrow_mut().as_mut().map(|shown| shown.chose(note, index));
    let (after, stop) = match chosen {
        Some(Chosen::Stop(scene, stop)) => (scene, Some(stop)),
        Some(Chosen::Tapped(scene)) => (scene, None),
        Some(Chosen::Nothing) | None => return,
    };
    // The view is in the middle of handling the tap: it takes the new scene once that is done.
    cx.defer(move |cx| {
        let _ = window.update(cx, |page, _, cx| page.set_scene(after, cx));
    });
    let (Some(stop), shown) = (stop, shown.clone()) else { return };
    cx.spawn(async move |cx| {
        let stopped = cx.background_executor().spawn(async move { stop() }).await;
        let Some(scene) = shown.borrow_mut().as_mut().map(|shown| shown.stopped(stopped)) else { return };
        let _ = window.update(cx, |page, _, cx| page.set_scene(scene, cx));
    })
    .detach();
}

fn page(
    scene: Scene,
    theme: Theme,
    viewport: Viewport,
    shown: Shown,
    cx: &mut App,
) -> gpui::Result<WindowHandle<Page>> {
    let bounds = Bounds::centered(None, size(px(viewport.w), px(viewport.h)), cx);
    // The handler is made before the window it will redraw, so the window's handle reaches it through this.
    let opened: Rc<Cell<Option<WindowHandle<Page>>>> = Rc::default();
    let (handle, on_show) = (opened.clone(), shown.clone());
    let tapped = move |tap: &Tap, _: &mut gpui::Window, cx: &mut App| {
        received(tap);
        let Some(window) = handle.get() else { return };
        match tap {
            Tap::Character { id } => poked(id, &on_show, window, cx),
            Tap::Option { note_id, index } => chose(note_id, *index, &on_show, window, cx),
        }
    };
    let handle = opened.clone();
    let made = move |made: &EntryMade, _: &mut gpui::Window, cx: &mut App| {
        if let Some(window) = handle.get() {
            entered(made, &shown, window, cx);
        }
    };
    // The entry comes with the first answer of `reads`.
    let view =
        move |_: &mut gpui::Window, cx: &mut App| cx.new(|_| Page::new(scene, theme).on_tap(tapped).on_entry(made));
    let window = cx.open_window(options(bounds), view)?;
    opened.set(Some(window));
    Ok(window)
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

fn switch_theme(page: &mut Page, cx: &mut Context<Page>) {
    let next = if page.theme().name == "plain" { Theme::sticker() } else { Theme::plain() };
    page.set_theme(next, cx);
}

/// One line for a tap: the piece's id, and the option when it was one.
fn received(tap: &Tap) {
    match tap {
        Tap::Character { id } => say(&format!("tap id={id}")),
        Tap::Option { note_id, index } => say(&format!("tap id={note_id} option={index}")),
    }
}

fn say(line: &str) {
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
