//! Motion is arithmetic on the theme's amounts.

use sticker_kit::Theme;
use sticker_kit::motion::{Spring, bob, hop, pulse, twinkle};

#[test]
fn zero_amounts_mean_no_motion() {
    let mut motion = Theme::sticker().motion;
    motion.bob_amplitude = 0.0;
    motion.hop_height = 0.0;
    motion.twinkle = 0.0;
    motion.pulse = 0.0;
    motion.stiffness = 0.0;
    motion.press = 0.0;
    assert!(motion.is_still());
    for step in 0..200 {
        let t = step as f32 * 0.05;
        assert_eq!(bob(&motion, t, 0.3), 0.0);
        assert_eq!(hop(&motion, t, 0.3), 0.0);
        assert_eq!(twinkle(&motion, t, 0.3), 1.0);
        assert_eq!(pulse(&motion, t, 0.3), 1.0);
    }
    let mut spring = Spring::at(0.0);
    spring.step(40.0, &motion, 1.0 / 60.0);
    assert_eq!(spring, Spring::at(40.0), "no stiffness: the piece is simply there");
}

#[test]
fn idle_motion_stays_within_the_theme_s_amounts() {
    let sticker = Theme::sticker().motion;
    assert!(!sticker.is_idle_still());
    let mut lowest = 0.0_f32;
    for step in 0..2000 {
        let t = step as f32 * 0.01;
        let (b, h) = (bob(&sticker, t, 0.0), hop(&sticker, t, 0.0));
        assert!((-sticker.bob_amplitude..=0.0).contains(&b));
        assert!((-sticker.hop_height..=0.0).contains(&h));
        assert!((1.0..=1.0 + sticker.twinkle).contains(&twinkle(&sticker, t, 0.0)));
        lowest = lowest.min(b);
    }
    assert!(lowest < -0.99 * sticker.bob_amplitude, "the bob reaches its amplitude");
}

#[test]
fn a_spring_overshoots_then_settles_on_its_target() {
    let motion = Theme::sticker().motion;
    let mut spring = Spring::at(0.0);
    let mut highest = 0.0_f32;
    for _ in 0..240 {
        spring.step(100.0, &motion, 1.0 / 60.0);
        highest = highest.max(spring.value);
    }
    assert!(highest > 100.0, "springy, not eased");
    assert_eq!(spring, Spring::at(100.0));
}
