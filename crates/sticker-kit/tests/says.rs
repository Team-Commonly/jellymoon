//! What a busy character says (live work): one line on a small bubble over it, cut to fit, over no other piece,
//! and nothing of a character that is not busy. A fixed-advance measurer and no GPUI.

use sticker_kit::{Character, FixedAdvance, Mood, PieceKind, Placed, Scene, State, Theme, Viewport, layout};

const SIZES: [Viewport; 2] = [Viewport { w: 1440.0, h: 900.0 }, Viewport { w: 960.0, h: 640.0 }];

fn character(n: usize, state: State, says: &str) -> Character {
    let (id, label, says) = (format!("c{n}"), format!("wish {n}"), says.to_string());
    Character { id, label, hue: 1, stars: 0, state, mood: Mood::Happy, says }
}

fn placed(characters: Vec<Character>, theme: &Theme, size: Viewport) -> Vec<Placed> {
    layout(&Scene { characters, ..Scene::sample() }, theme, size, &FixedAdvance(0.56))
}

#[test]
fn a_busy_character_says_one_line_over_itself_and_over_no_other_piece() {
    let long = format!("changing {}", "a-very-long-folder/".repeat(12));
    for theme in [Theme::sticker(), Theme::plain()] {
        for (count, size) in [1, 3, 8].into_iter().flat_map(|count| SIZES.map(|size| (count, size))) {
            let says = |n: usize| if n.is_multiple_of(2) { long.as_str() } else { "reading hours.txt" };
            let mut characters: Vec<Character> = (0..count).map(|n| character(n, State::Busy, says(n))).collect();
            characters.push(character(count, State::Idle, "reading hours.txt"));
            characters.push(character(count + 1, State::NeedsYou, "reading hours.txt"));
            characters.push(character(count + 2, State::Busy, ""));
            characters.push(Character { stars: 2, ..character(count + 3, State::Busy, "reading hours.txt") });
            let all = placed(characters, &theme, size);
            let said: Vec<&Placed> = all.iter().filter(|piece| piece.piece == PieceKind::Says).collect();
            // Every busy one with words and no star says them where there is room, and one alone always has it; no
            // other says any: not one that is idle or needs you, not one with no words, not one with stars.
            let ids: Vec<String> = (0..count).map(|n| format!("c{n}")).collect();
            assert!(said.iter().all(|piece| ids.contains(&piece.id)), "{count} at {size:?}: {said:?}");
            assert!(count > 1 || said.len() == count, "{count} at {size:?}: {} said", said.len());
            for piece in &said {
                let [run] = &piece.text[..] else { panic!("one line: {piece:?}") };
                assert!(run.rect.inside(&piece.rect.grown(0.5)), "{count} at {size:?}: the line leaves its bubble");
                let whole = says(piece.id[1..].parse().unwrap());
                let start = run.text.trim_end_matches('…');
                assert!(!start.is_empty() && whole.starts_with(start), "{count} at {size:?}: {}", run.text);
                let own = |other: &Placed| other.id == piece.id && other.piece == PieceKind::Character;
                // The paper and its punched holes are under every piece; a label reaches over a hole as well.
                let own = |other: &Placed| own(other) || matches!(other.piece, PieceKind::Paper | PieceKind::Hole);
                for other in all.iter().filter(|other| !std::ptr::eq(*other, *piece) && !own(other)) {
                    let other_rect = other.rect.turned_bounds(other.tilt_degrees);
                    assert!(
                        !piece.rect.overlaps(&other_rect),
                        "{count} at {size:?}: over {:?} {}",
                        other.piece,
                        other.id
                    );
                }
            }
        }
    }
}
