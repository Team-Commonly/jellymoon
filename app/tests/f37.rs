//! F37 in the app: a made wish is on today's page with the reading's wishes, before a reading and after one. The
//! wish is made by core, as `jellymoon wish` makes it; the built `jellymoon-app` is run as in `f34.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::fs;

use jellymoon_app::today::Today;
use sticker_kit::{Note, State};
use support::day::*;

const MADE: &str = "ship the changelog";

/// The labels drawn, in name order: where each stands on the page is the kit's layout.
fn drawn(world: &World, args: &[&str]) -> Vec<String> {
    let mut labels = labels(&shown(world, args));
    labels.sort();
    labels
}

fn make(world: &World) -> Vec<u8> {
    jellymoon_core::wish(&world.home, Some(DAY), MADE, "alpha").unwrap();
    fs::read(world.home.join("pages").join(format!("{DAY}.wishes.json"))).unwrap()
}

#[test]
fn a_made_wish_is_drawn_with_the_reading_s_wishes_and_a_new_reading_does_not_lose_it() {
    let world = world();
    let made = make(&world);
    for args in [&[][..], &["--fresh"]] {
        assert_eq!(drawn(&world, args), ["Fail fast", "Retry uploads", MADE], "{args:?}");
        assert!(called(&world), "{args:?}: the day was read");
        let page = fs::read_to_string(kept(&world)).unwrap();
        assert!(!page.contains(MADE), "the kept page is the reading's own: {page}");
        assert_eq!(fs::read(world.home.join("pages").join(format!("{DAY}.wishes.json"))).unwrap(), made);
    }
    assert_eq!(drawn(&world, &[]), ["Fail fast", "Retry uploads", MADE], "the kept page, with no reading");
    assert!(!called(&world));
}

// The window's first scene, before a log is opened, and the scene of a day that could not be read.
#[test]
fn a_made_wish_is_drawn_working_before_the_day_is_read_and_when_it_cannot_be() {
    let world = world();
    make(&world);
    let today = Today {
        date: DAY.to_string(),
        logs: world.dir.path().join("nowhere"),
        skip: Vec::new(),
        model: "sonnet".to_string(),
        home: world.home.clone(),
        harness: None,
        fresh: false,
        settings: None,
        bin: None,
    };
    let (first, kept) = today.first();
    let drawn: Vec<_> = first.characters.iter().map(|one| (one.id.as_str(), one.label.as_str(), one.state)).collect();
    assert_eq!(drawn, [("m1", MADE, State::Busy)]);
    assert!(kept.is_none() && matches!(&first.notes[0], Note::Plain { title, .. } if title == "Reading your day…"));

    let nowhere = world.dir.path().join("nowhere");
    let placed = shown(&world, &["--logs", nowhere.to_str().unwrap()]);
    assert_eq!(labels(&placed), [MADE]);
    assert!(notes(&placed)[0].starts_with("Today could not be read."), "{:?}", notes(&placed));
    assert!(!called(&world));
}
