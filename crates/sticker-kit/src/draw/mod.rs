//! The drawing half: a GPUI view that lays a scene out and paints it.
//!
//! Three layers, bottom to top: flat shapes (paper, tape, notes, tags) in
//! one canvas, sticker art as images, then text. Text is always level;
//! at this GPUI version only drawn paths can be turned.

mod entry;
#[cfg(feature = "headless")]
pub mod headless;
mod measure;
mod paint;
mod shapes;
mod sprites;
mod stage;

use std::time::Instant;

use gpui::{AnyElement, App, Context, MouseButton, Window, canvas, div, img, prelude::*, px};

use crate::geom::Viewport;
use crate::layout::{PieceKind, Placed, TextRun, layout, layout_entry, page_scale};
use crate::scene::Scene;
use crate::text::Role;
use crate::theme::Theme;

pub use entry::EntryMade;
use entry::EntryView;
pub use measure::{GlyphMeasure, load_fonts};
use measure::{family, hsla, weight};
use paint::Shape;
use shapes::Brush;
use sprites::Sprites;
use stage::{Pose, Stage, is_art};

/// What was tapped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tap {
    Character { id: String },
    Option { note_id: String, index: usize },
}

type TapHandler = Box<dyn Fn(&Tap, &mut Window, &mut App)>;

#[derive(Clone, Copy, PartialEq)]
enum Stale {
    Fresh,
    /// The window changed size: pieces go straight to their places.
    Snap,
    /// The scene or theme changed: pieces spring to their places.
    Spring,
}

/// Frames drawn and the longest gap between two, since the last reading.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameStats {
    pub frames: u32,
    pub worst_ms: f32,
}

pub struct Page {
    scene: Scene,
    theme: Theme,
    reduce_motion: bool,
    on_tap: Option<TapHandler>,
    entry: EntryView,
    placed: Vec<Placed>,
    viewport: Viewport,
    stale: Stale,
    stage: Stage,
    sprites: Sprites,
    themed: bool,
    pressed: Option<usize>,
    started: Instant,
    last: Instant,
    stats: FrameStats,
}

impl Page {
    pub fn new(scene: Scene, theme: Theme) -> Self {
        let now = Instant::now();
        Self {
            scene,
            theme,
            reduce_motion: false,
            on_tap: None,
            entry: EntryView::default(),
            placed: Vec::new(),
            viewport: Viewport::default(),
            stale: Stale::Spring,
            stage: Stage::default(),
            sprites: Sprites::default(),
            themed: false,
            pressed: None,
            started: now,
            last: now,
            stats: FrameStats::default(),
        }
    }

    /// Stops all motion; pieces are drawn where the layout puts them.
    pub fn reduce_motion(mut self, reduce: bool) -> Self {
        self.reduce_motion = reduce;
        self
    }

    /// Called with the character or the note option that was tapped.
    pub fn on_tap(mut self, handler: impl Fn(&Tap, &mut Window, &mut App) + 'static) -> Self {
        self.on_tap = Some(Box::new(handler));
        self
    }

    pub fn set_scene(&mut self, scene: Scene, cx: &mut Context<Self>) {
        self.scene = scene;
        self.stale = Stale::Spring;
        cx.notify();
    }

    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.theme = theme;
        self.themed = false;
        self.stale = Stale::Spring;
        cx.notify();
    }

    pub fn set_reduce_motion(&mut self, reduce: bool, cx: &mut Context<Self>) {
        self.reduce_motion = reduce;
        cx.notify();
    }

    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// The layout the last frame painted.
    pub fn placed(&self) -> &[Placed] {
        &self.placed
    }

    /// Reads and clears the frame counters.
    pub fn take_frame_stats(&mut self) -> FrameStats {
        std::mem::take(&mut self.stats)
    }

    fn relayout(&mut self, viewport: Viewport, window: &Window) {
        if viewport != self.viewport && self.stale == Stale::Fresh {
            self.stale = Stale::Snap;
        }
        if self.stale == Stale::Fresh {
            return;
        }
        let measure = GlyphMeasure { text: window.text_system(), theme: &self.theme };
        let mut placed = layout(&self.scene, &self.theme, viewport, &measure);
        if let Some(entry) = &self.entry.what {
            placed.extend(layout_entry(entry, &self.theme, viewport, &measure));
        }
        let animate = self.stale == Stale::Spring && !self.placed.is_empty();
        self.stage.retarget(&self.placed, &placed, &self.scene, animate);
        self.placed = placed;
        self.viewport = viewport;
        self.stale = Stale::Fresh;
        self.pressed = None;
    }

    fn tick(&mut self) -> (f32, f32) {
        let now = Instant::now();
        let dt = now.duration_since(self.last).as_secs_f32();
        self.last = now;
        if self.stats.frames > 0 {
            self.stats.worst_ms = self.stats.worst_ms.max(dt * 1000.0);
        }
        self.stats.frames += 1;
        (now.duration_since(self.started).as_secs_f32(), dt)
    }

    fn tap(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(piece) = self.placed.get(index) else {
            return;
        };
        let tap = match piece.piece {
            PieceKind::Option => Tap::Option { note_id: piece.id.clone(), index: piece.index },
            _ => Tap::Character { id: piece.id.clone() },
        };
        if let Some(handler) = &self.on_tap {
            handler(&tap, window, cx);
        }
    }

    fn tap_target(&self, index: usize, piece: &Placed, pose: Pose, cx: &mut Context<Self>) -> AnyElement {
        let rect = piece.rect.moved(pose.dx, pose.dy);
        div()
            .id(("tap", index))
            .absolute()
            .left(px(rect.x))
            .top(px(rect.y))
            .w(px(rect.w))
            .h(px(rect.h))
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |page, _, _, cx| {
                    page.pressed = Some(index);
                    cx.notify();
                }),
            )
            .on_click(cx.listener(move |page, _, window, cx| page.tap(index, window, cx)))
            .into_any_element()
    }
    /// The frame's flat shapes, and its elements in paint order: sticker
    /// art, then text, then the invisible tap targets.
    fn layers(
        &mut self,
        t: f32,
        still: bool,
        viewport: Viewport,
        cx: &mut Context<Self>,
    ) -> (Vec<Shape>, Vec<AnyElement>) {
        let motion = self.theme.motion;
        let k = page_scale(&self.theme, viewport);
        let rim = self.theme.metrics.rim * k;
        let typed = self.entry.state.holds();
        let brush =
            Brush { theme: &self.theme, scene: &self.scene, k, chosen: self.entry.state.chosen(), lit: self.entry.lit };
        let mut order: Vec<usize> = (0..self.placed.len()).collect();
        order.sort_by_key(|&i| self.placed[i].z);
        let mut flat = Vec::new();
        let (mut art, mut text, mut taps) = (Vec::new(), Vec::new(), Vec::new());
        for i in order {
            let piece = &self.placed[i];
            let pose = self.stage.pose(i, t, &motion, still);
            brush.add(piece, pose, &mut flat);
            if is_art(piece.piece) {
                let (image, rect) = self.sprites.get(piece, &self.scene, &self.theme, rim, cx);
                let rect = rect.moved(pose.dx, pose.dy).scaled(pose.scale);
                let sticker = img(image).absolute().left(px(rect.x)).top(px(rect.y));
                art.push(sticker.w(px(rect.w)).h(px(rect.h)).into_any_element());
            }
            // The hint gives way to words: not to spaces, or to other text nobody sees.
            let hidden = typed && piece.piece == PieceKind::Entry;
            for (n, run) in piece.text.iter().enumerate().filter(|(_, r)| !r.text.is_empty() && !hidden) {
                text.push(text_element(&brush, piece, n, run, pose));
            }
            if matches!(piece.piece, PieceKind::Hero | PieceKind::Character | PieceKind::Option) {
                taps.push(self.tap_target(i, piece, pose, cx));
            }
        }
        art.append(&mut text);
        art.append(&mut taps);
        (flat, art)
    }
}

fn text_element(brush: &Brush, piece: &Placed, n: usize, run: &TextRun, pose: Pose) -> AnyElement {
    let (theme, color) = (brush.theme, brush.ink(piece, run.role));
    let selector = format!("{:?}:{}:{}:{n}", piece.piece, piece.id, piece.index);
    let mut element = div()
        .debug_selector(move || selector)
        .absolute()
        .left(px(run.rect.x + pose.dx))
        .top(px(run.rect.y + pose.dy))
        .h(px(run.rect.h))
        .line_height(px(run.rect.h))
        .whitespace_nowrap()
        .font_family(family(theme, run.role))
        .font_weight(weight(theme, run.role))
        .text_size(px(run.size))
        .text_color(hsla(color));
    if matches!(run.role, Role::LabelStruck | Role::OptionStruck) {
        element = element.line_through();
    }
    element.child(run.text.clone()).into_any_element()
}

impl Render for Page {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let size = window.viewport_size();
        let viewport = Viewport { w: size.width.into(), h: size.height.into() };
        if !self.themed {
            self.sprites.forget(cx);
            self.themed = true;
        }
        self.relayout(viewport, window);
        let (t, dt) = self.tick();
        let still = self.reduce_motion || cx.reduce_motion() || self.theme.motion.is_still();
        let motion = self.theme.motion;
        if self.stage.advance(dt, &motion, self.pressed, still) {
            window.request_animation_frame();
        }

        let entry = self.entry_layer(window, cx);
        let (flat, layers) = self.layers(t, still, viewport, cx);
        div()
            .size_full()
            .relative()
            .overflow_hidden()
            .bg(hsla(self.theme.palette.desk))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::entry_left))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|page, _, _, cx| {
                    if page.pressed.take().is_some() {
                        cx.notify();
                    }
                }),
            )
            .child(
                canvas(|_, _, _| (), move |bounds, _, window, _| paint::paint(&flat, bounds.origin, window))
                    .absolute()
                    .size_full(),
            )
            .children(layers)
            .children(entry)
    }
}
