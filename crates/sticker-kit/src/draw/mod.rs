//! The drawing half: a GPUI view that lays a scene out and paints it.
//!
//! Three layers, bottom to top: flat shapes (paper, tape, notes, tags) in
//! one canvas, sticker art as images, then text. Text is always level;
//! at this GPUI version only drawn paths can be turned.
//!
//! The page is drawn twice over: the column, moved up by how far the view has scrolled it and shown only over the
//! entry's band, and then the entry in its band, which never moves (`column.rs`).

mod column;
mod edge;
mod entry;
mod field;
#[cfg(feature = "headless")]
pub mod headless;
mod input;
mod measure;
mod paint;
mod pencil;
mod settle;
mod shapes;
mod sprites;
mod stage;

use std::path::PathBuf;
use std::time::Instant;

use gpui::{
    AnyElement, App, Context, DispatchPhase, DragMoveEvent, ExternalPaths, MouseButton, MouseUpEvent, Window, canvas,
    div, img, prelude::*, px,
};

use crate::geom::{Rect, Viewport};
use crate::layout::{PieceKind, Placed, entry_top, layout, layout_entry, page_scale, scroll_most};
use crate::scene::Scene;
use crate::scroll::Scroll;
use crate::theme::Theme;

use column::text_element;
pub use edge::{EdgePainted, EdgePart, EdgeView, edge_ink};
pub use entry::EntryMade;
use entry::EntryView;
pub use field::MAX_KEPT_FIELDS;
use measure::hsla;
pub use measure::{GlyphMeasure, load_fonts};
use paint::Shape;
use pencil::Sketched;
pub use settle::SETTLE;
use settle::Settled;
use shapes::Brush;
use sprites::Sprites;
use stage::{Stage, is_art};

/// What was tapped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tap {
    Character {
        id: String,
    },
    /// An option of a note with its `words` as the scene that was drawn held them, or, at
    /// [`crate::PICTURE_PLACE`] and with no words, the picture of a drawn note or its `?`.
    Option {
        note_id: String,
        index: usize,
        words: String,
    },
    /// The action of the entry `id` ([`crate::Entry::action`]).
    Action {
        id: String,
    },
    /// The `go` of the entry `id` while it held no words: nothing was entered ([`crate::Entry::go`]).
    Go {
        id: String,
    },
}

type TapHandler = Box<dyn Fn(&Tap, &mut Window, &mut App)>;
type PathsHandler = Box<dyn Fn(&[PathBuf], &mut Window, &mut App)>;

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
    on_paths: Option<PathsHandler>,
    /// What is being dragged over the page from outside the app.
    dragged: Vec<PathBuf>,
    entry: EntryView,
    /// The field of each note of the scene that has one, in the notes' order.
    fields: Vec<EntryView>,
    /// The fields kept apart from the scene (`field.rs`): they are not drawn and take no key.
    apart: Vec<EntryView>,
    placed: Vec<Placed>,
    viewport: Viewport,
    /// How far the column is moved up the window, and how far it can be.
    scroll: Scroll,
    most: f32,
    stale: Stale,
    stage: Stage,
    /// When each picture came, for the pencil that draws it.
    sketched: Sketched,
    /// Where each choice stands and since when: one that has just moved takes no tap (`settle.rs`).
    settled: Settled,
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
        let ((fields, apart), scroll) = (field::fit(&scene, Vec::new()), column::opening(&scene));
        Self {
            scene,
            theme,
            reduce_motion: false,
            on_tap: None,
            on_paths: None,
            dragged: Vec::new(),
            entry: EntryView::default(),
            fields,
            apart,
            placed: Vec::new(),
            viewport: Viewport::default(),
            scroll,
            most: 0.0,
            stale: Stale::Spring,
            stage: Stage::default(),
            sketched: Sketched::default(),
            settled: Settled::default(),
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

    /// Called with the paths of files or folders dropped on the page from outside the app. Like `on_tap`, it is
    /// called while the page is in use.
    pub fn on_paths(mut self, handler: impl Fn(&[PathBuf], &mut Window, &mut App) + 'static) -> Self {
        self.on_paths = Some(Box::new(handler));
        self
    }

    /// A mouse button let go anywhere on the page: a drop from outside the app when something is dragged. GPUI's
    /// own drop asks whether the page is hovered, which it never is after a key until the mouse moves, and a drag
    /// from another app moves no mouse over this window: a folder dropped after typing would be lost.
    fn dropped(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let paths = std::mem::take(&mut self.dragged);
        if let (true, false, Some(handler)) = (cx.has_active_drag(), paths.is_empty(), &self.on_paths) {
            handler(&paths, window, cx);
        }
    }

    pub fn set_scene(&mut self, scene: Scene, cx: &mut Context<Self>) {
        let was = std::mem::take(&mut self.fields).into_iter().chain(std::mem::take(&mut self.apart));
        (self.fields, self.apart) = field::fit(&scene, was.collect());
        self.hold_for(&scene);
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

    /// The layout the last frame painted, in the column's own places: not moved by the scroll.
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
        self.sketched.meet(&placed, self.started.elapsed().as_secs_f32());
        self.most = scroll_most(&placed, &self.theme, viewport);
        let before = self.scroll.at();
        self.scroll.fit(self.most);
        let place = (before, self.scroll.at(), settle::SLACK * page_scale(&self.theme, viewport));
        let moved = self.stale == Stale::Spring;
        self.settled.meet(&placed, &self.scene, place, self.settle_time(), moved);
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

    /// The frame's flat shapes and its elements in paint order (sticker art, then text, then the invisible tap
    /// targets), of the column first and of the entry's band second.
    fn layers(&mut self, t: f32, still: bool, viewport: Viewport, cx: &mut Context<Self>) -> [Layer; 2] {
        let motion = self.theme.motion;
        let k = page_scale(&self.theme, viewport);
        let rim = self.theme.metrics.rim * k;
        let typed = self.entry.state.holds();
        let keyed = self.fields.iter().find(|view| view.lit).and_then(|view| view.what.as_ref());
        let loud = self.entry.what.as_ref().is_some_and(|entry| entry.loud);
        let (chosen, keyed) = (self.entry.state.chosen().filter(|_| !loud), keyed.map(|field| field.id.as_str()));
        let (lit, ready) = (self.entry.lit, typed && !self.entry.state.waiting());
        let brush = Brush { theme: &self.theme, scene: &self.scene, k, chosen, lit, keyed, loud, ready };
        let mut order: Vec<usize> = (0..self.placed.len()).collect();
        order.sort_by_key(|&i| self.placed[i].z);
        let mut flat = [Vec::new(), Vec::new()];
        let mut parts: [[Vec<AnyElement>; 3]; 2] = Default::default();
        self.settled.targets.clear();
        for i in order {
            let piece = &self.placed[i];
            let at = usize::from(piece.piece.in_band());
            let mut pose = self.stage.pose(i, t, &motion, still);
            pose.dy -= self.lift(piece);
            brush.add(piece, pose, &mut flat[at]);
            self.sketched.add(piece, (t, still, k), pose, (&self.theme.palette, &motion), &mut flat[at]);
            if is_art(piece.piece) {
                let (image, rect) = self.sprites.get(piece, &self.scene, &self.theme, rim, cx);
                let rect = rect.moved(pose.dx, pose.dy).scaled(pose.scale);
                let sticker = img(image).absolute().left(px(rect.x)).top(px(rect.y));
                parts[at][0].push(sticker.w(px(rect.w)).h(px(rect.h)).into_any_element());
            }
            // The hint gives way to words: not to spaces, or to other text nobody sees.
            let hidden = match piece.piece {
                PieceKind::Entry => typed,
                PieceKind::Field => self.field(&piece.id).is_some_and(|field| field.holds()),
                _ => false,
            };
            for (n, run) in piece.text.iter().enumerate().filter(|(_, r)| !r.text.is_empty() && !hidden) {
                parts[at][1].push(text_element(&brush, piece, n, run, pose));
            }
            if self.taps(piece) {
                parts[at][2].push(self.tap_target(i, piece, pose, cx));
                // What is moved under the band's top takes no tap, and is no target.
                let room = if at == 0 { entry_top(&self.theme, viewport).max(0.0) } else { viewport.h };
                let seen = Rect::new(0.0, 0.0, viewport.w, room);
                self.settled.targets.push(piece.rect.moved(pose.dx, pose.dy).clipped(&seen));
            }
        }
        let layer = |(flat, parts): (Vec<Shape>, [Vec<AnyElement>; 3])| -> Layer {
            (flat, parts.into_iter().flatten().collect())
        };
        let mut layers = flat.into_iter().zip(parts).map(layer);
        [layers.next().unwrap_or_default(), layers.next().unwrap_or_default()]
    }
}

/// The flat shapes of a layer, and its elements over them.
type Layer = (Vec<Shape>, Vec<AnyElement>);

/// The column's layer, `shown` high from the window's top: the column is shown over the entry's band only, and
/// what is moved under that line is not drawn and takes no tap.
fn column_layer(column: Vec<Shape>, over: Vec<AnyElement>, fields: Vec<AnyElement>, shown: f32) -> gpui::Div {
    let shapes = canvas(|_, _, _| (), move |bounds, _, window, _| paint::paint(&column, bounds.origin, window));
    let layer = div().absolute().top_0().left_0().w_full().h(px(shown)).overflow_hidden();
    layer.child(shapes.absolute().size_full()).children(over).children(fields)
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
        let moving = self.stage.advance(dt, &motion, self.pressed, still);
        if moving || self.sketched.moving(&self.placed, t, still, &motion) {
            window.request_animation_frame();
        }

        let [fields, entry] = self.entry_layer(window, cx);
        let [(column, over), (band, top)] = self.layers(t, still, viewport, cx);
        let shown = entry_top(&self.theme, viewport).max(0.0);
        let dropped_on = cx.weak_entity();
        div()
            .size_full()
            .relative()
            .overflow_hidden()
            .bg(hsla(self.theme.palette.desk))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::entry_left))
            .on_scroll(cx.listener(Self::wheeled))
            .on_drag_move(cx.listener(|page, moved: &DragMoveEvent<ExternalPaths>, _, cx| {
                page.dragged = moved.drag(cx).paths().to_vec();
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|page, _, _, cx| {
                    if page.pressed.take().is_some() {
                        cx.notify();
                    }
                }),
            )
            .child(column_layer(column, over, fields, shown))
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        paint::paint(&band, bounds.origin, window);
                        window.on_mouse_event(move |_: &MouseUpEvent, phase, window, cx| {
                            if phase == DispatchPhase::Bubble {
                                let _ = dropped_on.update(cx, |page, cx| page.dropped(window, cx));
                            }
                        });
                    },
                )
                .absolute()
                .size_full(),
            )
            .children(top)
            .children(entry)
    }
}
