//! Drawing with no window on screen, for tests and snapshots.

use std::path::Path;
use std::sync::Arc;

use gpui::{AppContext as _, HeadlessAppContext, PlatformTextSystem, px, size};

use super::{Page, load_fonts};
use crate::scene::Scene;
use crate::theme::Theme;

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

/// Renders one still frame of the scene and saves it as a PNG.
pub fn snapshot(scene: Scene, theme: Theme, width: f32, height: f32, out: &Path) -> gpui::Result<()> {
    let mut cx =
        HeadlessAppContext::with_platform(text_system(), Arc::new(()), gpui_platform::current_headless_renderer);
    cx.update(|cx| load_fonts(cx))?;
    let window =
        cx.open_window(size(px(width), px(height)), |_, cx| cx.new(|_| Page::new(scene, theme).reduce_motion(true)))?;
    cx.run_until_parked();
    let image = cx.capture_screenshot(window.into())?;
    image.save(out)?;
    Ok(())
}
