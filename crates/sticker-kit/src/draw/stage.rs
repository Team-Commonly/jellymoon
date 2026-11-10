//! Per-piece motion state: a spring back to the laid-out place, a spring on
//! scale for appearing and pressing, and the idle motion its kind calls for.

use std::collections::HashMap;

use crate::layout::{PieceKind, Placed};
use crate::motion::{Spring, bob, hop, pulse, twinkle};
use crate::scene::{Scene, State};
use crate::theme::Motion;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Idle {
    None,
    Bob,
    Hop,
    Twinkle,
    Pulse,
}

/// Where a piece is drawn relative to its laid-out rectangle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Pose {
    pub dx: f32,
    pub dy: f32,
    pub scale: f32,
}

pub(crate) const REST: Pose = Pose { dx: 0.0, dy: 0.0, scale: 1.0 };

#[derive(Clone, Copy, Debug)]
struct Actor {
    dx: Spring,
    dy: Spring,
    /// In percent, so the spring's settling threshold suits it.
    scale: Spring,
    idle: Idle,
    phase: f32,
}

#[derive(Default)]
pub(crate) struct Stage {
    actors: Vec<Actor>,
}

pub(crate) fn is_art(kind: PieceKind) -> bool {
    use PieceKind::*;
    matches!(kind, Hero | Character | Star | Bubble | Badge)
}

fn idle_for(piece: &Placed, scene: &Scene) -> Idle {
    let state = |index: usize| scene.characters.get(index).map(|c| c.state);
    match piece.piece {
        PieceKind::Hero => Idle::Bob,
        PieceKind::Character => match state(piece.index) {
            Some(State::Busy) => Idle::Hop,
            Some(State::Dimmed) | None => Idle::None,
            Some(_) => Idle::Bob,
        },
        PieceKind::Star => Idle::Twinkle,
        PieceKind::Bubble => Idle::Hop,
        PieceKind::Badge => Idle::Pulse,
        _ => Idle::None,
    }
}

impl Stage {
    /// Adopts a new layout. With `animate`, pieces that moved start from
    /// where they were and new art pops in; otherwise everything is in place.
    pub fn retarget(&mut self, old: &[Placed], new: &[Placed], scene: &Scene, animate: bool) {
        let before: HashMap<(PieceKind, &str, usize), (&Placed, &Actor)> = old
            .iter()
            .zip(&self.actors)
            .map(|(piece, actor)| ((piece.piece, piece.id.as_str(), piece.index), (piece, actor)))
            .collect();
        let actors = new
            .iter()
            .enumerate()
            .map(|(i, piece)| {
                let mut actor = Actor {
                    dx: Spring::default(),
                    dy: Spring::default(),
                    scale: Spring::at(100.0),
                    idle: idle_for(piece, scene),
                    phase: (i as f32 * 0.27).fract(),
                };
                if !animate {
                    return actor;
                }
                match before.get(&(piece.piece, piece.id.as_str(), piece.index)) {
                    Some((was, old_actor)) => {
                        actor.dx = old_actor.dx;
                        actor.dy = old_actor.dy;
                        actor.dx.value += was.rect.x - piece.rect.x;
                        actor.dy.value += was.rect.y - piece.rect.y;
                        actor.scale = old_actor.scale;
                    }
                    None if is_art(piece.piece) => actor.scale = Spring::at(60.0),
                    None => {}
                }
                actor
            })
            .collect();
        self.actors = actors;
    }

    /// Steps every spring. Returns true while another frame is needed.
    pub fn advance(&mut self, dt: f32, motion: &Motion, pressed: Option<usize>, still: bool) -> bool {
        let mut moving = false;
        for (i, actor) in self.actors.iter_mut().enumerate() {
            let rest = if pressed == Some(i) { 100.0 * (1.0 - motion.press) } else { 100.0 };
            if still {
                actor.dx = Spring::default();
                actor.dy = Spring::default();
                actor.scale = Spring::at(100.0);
                continue;
            }
            actor.dx.step(0.0, motion, dt);
            actor.dy.step(0.0, motion, dt);
            actor.scale.step(rest, motion, dt);
            let settled = actor.dx.is_settled(0.0) && actor.dy.is_settled(0.0) && actor.scale.is_settled(rest);
            moving |= !settled || (actor.idle != Idle::None && !motion.is_idle_still());
        }
        moving
    }

    pub fn pose(&self, index: usize, t: f32, motion: &Motion, still: bool) -> Pose {
        let Some(actor) = self.actors.get(index) else {
            return REST;
        };
        if still {
            return REST;
        }
        let mut pose = Pose { dx: actor.dx.value, dy: actor.dy.value, scale: actor.scale.value / 100.0 };
        match actor.idle {
            Idle::None => {}
            Idle::Bob => pose.dy += bob(motion, t, actor.phase),
            Idle::Hop => pose.dy += hop(motion, t, actor.phase),
            Idle::Twinkle => pose.scale *= twinkle(motion, t, actor.phase),
            Idle::Pulse => pose.scale *= pulse(motion, t, actor.phase),
        }
        pose
    }
}
