//! What the window does with a theme's art, with no window on a screen: GPUI's test platform opens the page
//! `window.rs` makes, and the trials it runs off its thread are run by the test's executor, with this very app
//! for the child. On the Mac only, as the kit's draw tests are.

use std::fs;

use gpui::{TestAppContext, TestDispatcher};
use jellymoon_app::showing::Showing;
use jellymoon_app::window::{arts, page};
use sticker_kit::draw::headless::text_system;
use sticker_kit::draw::load_fonts;
use sticker_kit::theme::Trial;
use sticker_kit::{Scene, Theme, Viewport};

use super::HONEST;

/// The pieces and pictures the page draws of a theme's art, once the window has tried the theme `tried` while
/// the theme `shown` was on show.
fn drawn(tried: &Theme, shown: &Theme) -> Vec<(&'static str, String)> {
    let mut cx = TestAppContext::build_with_text_system(TestDispatcher::new(0), None, text_system());
    cx.update(|cx| load_fonts(cx)).unwrap();
    let viewport = Viewport { w: 1440.0, h: 900.0 };
    let window = cx.update(|cx| page(Scene::sample(), shown.clone(), viewport, Showing::of(None), cx)).unwrap();
    let view = window.root(&mut cx).unwrap();
    // Nothing is kept: the picture is tried, here and now, by the app started again.
    let trial = Trial::new(env!("CARGO_BIN_EXE_jellymoon-app").into(), jellymoon_app::TRIAL, None, "a test");
    let tried = tried.clone();
    cx.update(|cx| cx.spawn(async move |cx| arts(tried, trial, window, cx).await).detach());
    cx.run_until_parked();
    view.read_with(&cx, |page, _| page.art().iter().map(|art| (art.piece(), art.svg().to_string())).collect())
}

#[test]
fn the_window_draws_the_art_its_trials_accepted_and_only_for_the_theme_on_show() {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let scripted = HONEST.replace("<circle", "<script/><circle");
    fs::write(dir.path().join("star.svg"), HONEST).unwrap();
    fs::write(dir.path().join("badge.svg"), scripted).unwrap();
    fs::write(dir.path().join("theme.toml"), "[art]\nstar = \"star.svg\"\nbadge = \"badge.svg\"\n").unwrap();
    let theme = Theme::load(&dir.path().join("theme.toml")).unwrap().theme;

    // The picture that passed is the page's, and the one that was refused is not.
    assert_eq!(drawn(&theme, &theme), vec![("star", HONEST.to_string())]);
    // Another theme was put on show while this one's art was tried: its art is not this theme's.
    assert_eq!(drawn(&theme, &Theme::sticker()), vec![]);
}
