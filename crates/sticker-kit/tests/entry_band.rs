//! The band an entry stands in is kept free by every page, wherever a theme puts its big character.

use sticker_kit::{Entry, FixedAdvance, PieceKind, Scene, Theme, Viewport, layout, layout_entry};

const SIZES: [Viewport; 2] = [Viewport { w: 1440.0, h: 900.0 }, Viewport { w: 960.0, h: 640.0 }];
const MEASURE: FixedAdvance = FixedAdvance(0.56);

// A small big character high on the page would let the small ones begin beside the title, where the entry is.
#[test]
fn the_small_characters_begin_under_the_entry_wherever_a_theme_puts_its_big_one() {
    let mut high = Theme::sticker();
    (high.metrics.hero_size, high.metrics.hero_top, high.metrics.hero_x) = (40.0, 60.0, 800.0);
    assert_eq!(high.check(), Ok(()));
    // Six names too long for one row: the band is kept free down to the second row of chips.
    let entry = Entry { id: "wish".into(), hint: "wish".into(), options: vec!["alpha".repeat(20); 6], most: 0 };
    for viewport in SIZES {
        let ours = layout_entry(&entry, &high, viewport, &MEASURE);
        assert_eq!(ours.len(), 7, "{viewport:?}: the field and six chips");
        assert!(ours[6].rect.y > ours[1].rect.y, "{viewport:?}: the chips stand in two rows");
        for scene in [Scene::sample(), Scene::crowd(50)] {
            let page = layout(&scene, &high, viewport, &MEASURE);
            let paper = page.iter().find(|p| p.piece == PieceKind::Paper).unwrap().rect;
            let room = 24.0 * paper.w / 1344.0 - 0.5;
            for small in page.iter().filter(|p| [PieceKind::Character, PieceKind::Star].contains(&p.piece)) {
                let under = small.rect.turned_bounds(small.tilt_degrees).y - ours[6].rect.bottom();
                assert!(under >= room, "{viewport:?}: {small:?} is {under} under the chips, {room} is kept");
            }
            let others = page.iter().filter(|p| p.piece != PieceKind::Paper);
            for (mine, other) in others.flat_map(|other| ours.iter().map(move |mine| (mine, other))) {
                let outline = other.rect.turned_bounds(other.tilt_degrees);
                assert!(!mine.rect.overlaps(&outline), "{viewport:?}: {mine:?} covers {other:?}");
            }
        }
    }
}
