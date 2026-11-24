//! The chips of a day nothing was typed on: a name is drawn as text, cut to its chip by the kit's own measure, and
//! a list of a hundred projects is six chips and a count of the rest.

use std::fs;

use super::{Logs, list, nothing_typed, of, words};
use crate::support::day::called;
use crate::support::rect;
use crate::support::wished::World;

/// A folder `work/<name>` that holds a `.git`: what a project is listed and offered by. Nothing starts in it here.
fn folder(w: &World, name: &str) {
    fs::create_dir_all(w.at("work").join(name).join(".git")).unwrap();
}

/// The names `jellymoon project list` prints, in the list's order.
fn listed(w: &World) -> Vec<String> {
    let list = w.jellymoon(&["project", "list"]);
    list["projects"].as_array().unwrap().iter().map(|one| one["name"].as_str().unwrap().to_string()).collect()
}

#[test]
fn a_name_is_drawn_as_text_and_cut_to_its_chip() {
    // Shapes a page that ran text would act on: here each is a folder's name, and a chip's words.
    let odd = ["[link](x)", "&lt;b&gt;bold", "{{name}}", "`ls`"];
    let long = format!("a-project-{}-with-its-own-end", "with-a-very-long-name".repeat(3));
    for logs in [Logs::Absent, Logs::Empty] {
        let w = nothing_typed(logs);
        for name in [long.as_str()].into_iter().chain(odd) {
            folder(&w, name);
            list(&w, name);
        }
        let placed = w.placed();
        let chips = words(&placed, "chip");
        assert_eq!(chips[1..], odd, "{logs:?}: each name as it is, and nothing else");
        // The long name is cut in the middle, so its start and its end are both read.
        let cut = &chips[0];
        let (start, end) = cut.split_once('…').unwrap_or_else(|| panic!("{cut:?} is not cut"));
        assert!(cut.chars().count() < long.chars().count() && !start.is_empty() && !end.is_empty(), "{cut:?}");
        assert!(long.starts_with(start) && long.ends_with(end), "{cut:?} is not {long:?} with its middle left out");
        // The field's row: the field, and the choice that ends it.
        let (field, go) = (rect(&of(&placed, "entry")[0].0), rect(&of(&placed, "go")[0].0));
        for chip in of(&placed, "chip") {
            let ([x, y, w, h], [tx, ty, tw, th]) = (chip.rect(), rect(&chip.runs()[0]));
            assert!(x <= tx && tx + tw <= x + w + 0.01, "{chip:?}: its words leave it");
            assert!(y - 0.5 <= ty && ty + th <= y + h + 0.5, "{chip:?}: its words leave it");
            let under = field[0] <= x && x + w <= go[0] + go[2] + 0.01 && y >= field[1] + field[3];
            assert!(under, "{chip:?} is not under the field's row");
        }
        assert!(of(&placed, "more").is_empty() && !called(&w.day), "{logs:?}");
    }
}

#[test]
fn a_hundred_projects_are_six_chips_and_a_count_of_the_rest() {
    let w = nothing_typed(Logs::Absent);
    for n in 0..7 {
        folder(&w, &format!("p{n:02}"));
        list(&w, &format!("p{n:02}"));
    }
    let placed = w.placed();
    assert_eq!(words(&placed, "chip"), listed(&w)[..6], "the first six of the list, in its order");
    assert_eq!(words(&placed, "more"), ["+1 more"]);

    for n in 7..100 {
        folder(&w, &format!("p{n:02}"));
        list(&w, &format!("p{n:02}"));
    }
    let names = listed(&w);
    assert_eq!(names.len(), 100);
    let placed = w.placed();
    assert_eq!(words(&placed, "chip"), names[..6]);
    let [more] = of(&placed, "more")[..] else { panic!("one count of the rest: {placed:?}") };
    assert_eq!((more.said().as_str(), more.id(), more.0["index"].as_u64()), ("+94 more", "wish", Some(94)));
    // The count stands in the entry's band, after the last chip of its row, and covers none.
    let (field, last) = (rect(&of(&placed, "entry")[0].0), of(&placed, "chip")[5].rect());
    let [x, y, w_, h] = more.rect();
    assert!(x >= last[0] + last[2] && x + w_ <= field[0] + field[2] + 0.01, "{more:?} after {last:?}");
    assert!((y - last[1]).abs() < 0.01 && (h - last[3]).abs() < 0.01, "{more:?} is not in the row of {last:?}");
    assert!(!called(&w.day));
}
