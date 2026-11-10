//! Motion as arithmetic: where a piece is at time `t`, given the theme's
//! amounts. Zero amounts give zero motion.

use std::f32::consts::PI;

use crate::theme::Motion;

impl Motion {
    /// True when nothing moves unless the page itself changes.
    pub fn is_idle_still(&self) -> bool {
        (self.bob_amplitude == 0.0 || self.bob_period <= 0.0)
            && (self.hop_height == 0.0 || self.hop_period <= 0.0)
            && self.twinkle == 0.0
            && self.pulse == 0.0
    }

    /// True when the theme asks for no motion of any kind.
    pub fn is_still(&self) -> bool {
        self.is_idle_still() && self.stiffness == 0.0 && self.press == 0.0
    }
}

fn cycle(t: f32, period: f32, phase: f32) -> f32 {
    (t / period + phase).rem_euclid(1.0)
}

/// A slow rise and fall; the vertical offset in pixels (up is negative).
pub fn bob(motion: &Motion, t: f32, phase: f32) -> f32 {
    if motion.bob_amplitude == 0.0 || motion.bob_period <= 0.0 {
        return 0.0;
    }
    let u = cycle(t, motion.bob_period, phase);
    -motion.bob_amplitude * (0.5 - 0.5 * (2.0 * PI * u).cos())
}

/// A quick hop, then a rest; the vertical offset in pixels.
pub fn hop(motion: &Motion, t: f32, phase: f32) -> f32 {
    if motion.hop_height == 0.0 || motion.hop_period <= 0.0 {
        return 0.0;
    }
    let u = cycle(t, motion.hop_period, phase);
    if u < 0.6 { -motion.hop_height * (PI * u / 0.6).sin() } else { 0.0 }
}

/// A star's scale: still most of the time, with a short swell.
pub fn twinkle(motion: &Motion, t: f32, phase: f32) -> f32 {
    swell(motion.twinkle, cycle(t, 2.6, phase), 0.7)
}

/// A badge's scale.
pub fn pulse(motion: &Motion, t: f32, phase: f32) -> f32 {
    swell(motion.pulse, cycle(t, 1.7, phase), 0.55)
}

fn swell(amount: f32, u: f32, from: f32) -> f32 {
    if amount == 0.0 || u < from {
        return 1.0;
    }
    1.0 + amount * (PI * (u - from) / (1.0 - from)).sin()
}

/// A damped spring pulling one number toward a target.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Spring {
    pub value: f32,
    pub velocity: f32,
}

impl Spring {
    pub fn at(value: f32) -> Self {
        Self { value, velocity: 0.0 }
    }

    /// Advances by `dt` seconds. Zero stiffness jumps straight to the target.
    pub fn step(&mut self, target: f32, motion: &Motion, dt: f32) {
        if motion.stiffness <= 0.0 {
            *self = Spring::at(target);
            return;
        }
        let mut left = dt.clamp(0.0, 0.1);
        while left > 0.0 {
            let h = left.min(1.0 / 240.0);
            let pull = motion.stiffness * (target - self.value) - motion.damping * self.velocity;
            self.velocity += pull * h;
            self.value += self.velocity * h;
            left -= h;
        }
        if self.is_settled(target) {
            *self = Spring::at(target);
        }
    }

    pub fn is_settled(&self, target: f32) -> bool {
        (self.value - target).abs() < 0.02 && self.velocity.abs() < 0.5
    }
}
