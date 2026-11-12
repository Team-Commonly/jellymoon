//! Drawing with no window on screen, for tests and snapshots.

use std::path::Path;
use std::sync::Arc;

use gpui::{AppContext as _, HeadlessAppContext, PlatformTextSystem, px, size};

use super::{Page, load_fonts};
use crate::scene::{Entry, Scene};
use crate::theme::{Accepted, Theme};

/// The platform's real text system, safe to build off the main thread.
pub fn text_system() -> Arc<dyn PlatformTextSystem> {
    #[cfg(target_os = "macos")]
    {
        Arc::new(gpui_apple::AppleTextSystem::new())
    }
    #[cfg(not(target_os = "macos"))]
    {
        gpui_platform::current_platform(true).text_system()
    }
}

/// An entry as a picture shows it: empty with its hint, or, with `typed`, holding those words and the keys (so
/// the caret is drawn) with the option of that place chosen.
pub struct Typing {
    pub entry: Entry,
    pub typed: Option<(String, usize)>,
}

/// Renders one still frame of the scene and saves it as a PNG.
pub fn snapshot(scene: Scene, theme: Theme, width: f32, height: f32, out: &Path) -> gpui::Result<()> {
    snapshot_typing(scene, theme, None, width, height, out)
}

/// [`snapshot`] of a page with an entry.
pub fn snapshot_typing(
    scene: Scene,
    theme: Theme,
    typing: Option<Typing>,
    width: f32,
    height: f32,
    out: &Path,
) -> gpui::Result<()> {
    snapshot_with(scene, (theme, Vec::new()), typing, width, height, out)
}

/// [`snapshot_typing`] of a theme with the art a trial accepted for it ([`Theme::try_art`]). A picture is drawn
/// from no other art of a theme's, with a window or without.
pub fn snapshot_with(
    scene: Scene,
    (theme, art): (Theme, Vec<Accepted>),
    typing: Option<Typing>,
    width: f32,
    height: f32,
    out: &Path,
) -> gpui::Result<()> {
    let mut cx =
        HeadlessAppContext::with_platform(text_system(), Arc::new(()), gpui_platform::current_headless_renderer);
    cx.update(|cx| load_fonts(cx))?;
    let (entry, typed) = typing.map(|typing| (typing.entry, typing.typed)).unzip();
    let window = cx.open_window(size(px(width), px(height)), |_, cx| {
        let page = Page::new(scene, theme).with_art(art).reduce_motion(true);
        cx.new(|_| entry.into_iter().fold(page, Page::with_entry))
    })?;
    if let Some((text, option)) = typed.flatten() {
        cx.update_window(window.into(), |_, window, cx| {
            window.root::<Page>().flatten().map(|page| {
                page.update(cx, |page, cx| {
                    page.entry.state.choose(option);
                    page.focus_entry(window, cx);
                    page.entry.state.replace(None, &text);
                    cx.notify();
                })
            })
        })?;
    }
    cx.run_until_parked();
    let image = cx.capture_screenshot(window.into())?;
    image.save(out)?;
    Ok(())
}
