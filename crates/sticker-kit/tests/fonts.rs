//! The measurer that asks no operating system: the same numbers on every machine.

use sticker_kit::{BundledMeasure, Measure, Role, Theme};

fn near(got: f32, want: f32) -> bool {
    (got - want).abs() < 0.01
}

#[test]
fn a_label_is_as_wide_as_its_advances_in_the_bundled_cut() {
    let theme = Theme::sticker();
    let measure = BundledMeasure::new(&theme);
    // Read from Fredoka-Medium's own advance widths: a fixed number, whatever the machine.
    assert!(near(measure.width("Login page", Role::Label, 14.0), 69.31));
    assert!(near(measure.width("Login page", Role::Label, 28.0), 138.62), "width grows with the size");
    assert!(near(measure.width("", Role::Label, 14.0), 0.0));
    let (strong, quiet) =
        (measure.width("Two pages", Role::RowMain, 15.0), measure.width("Two pages", Role::RowSub, 15.0));
    assert!(strong > quiet, "the bold cut is wider than the semibold: {strong} and {quiet}");
}

#[test]
fn a_character_no_bundled_font_has_follows_the_stated_rule() {
    let theme = Theme::sticker();
    let measure = BundledMeasure::new(&theme);
    let width = |text: &str| measure.width(text, Role::Label, 10.0);
    assert!(near(width("本棚"), 20.0), "East Asian: one em each");
    assert!(near(width("🌙"), 14.0), "an emoji: 1.4 em");
    assert!(near(width("👩\u{200D}🚀"), 28.0), "a joiner adds nothing");
    assert!(near(width("\u{0E01}"), 6.0), "anything else: 0.6 em");
}

#[test]
fn a_family_that_is_not_bundled_is_measured_as_nunito() {
    let (plain, sticker) = (Theme::plain(), Theme::sticker());
    let (system, bundled) = (BundledMeasure::new(&plain), BundledMeasure::new(&sticker));
    let text = "came true today";
    assert!(near(system.width(text, Role::Label, 14.0), bundled.width(text, Role::RowSub, 14.0)));
    assert!(near(system.width(text, Role::Title, 14.0), bundled.width(text, Role::RowMain, 14.0)));
}
