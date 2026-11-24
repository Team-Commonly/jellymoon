//! How far a view has moved the column up its window. Plain data and two rules, with no window in them: the place
//! is held between the column's top and its end, and a view that is at the end stays there when the column grows,
//! so what was just said is under the eyes. The layout knows nothing of this: its places are the column's own.

/// How far the column is moved up, in logical pixels, and whether it is held at its end.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scroll {
    at: f32,
    stuck: bool,
}

impl Default for Scroll {
    /// At the column's end: a page opens on its newest card.
    fn default() -> Self {
        Self { at: 0.0, stuck: true }
    }
}

impl Scroll {
    /// How far the column is moved up.
    pub fn at(&self) -> f32 {
        self.at
    }

    /// A turn of the wheel by `dy` logical pixels, down the window when positive as the platform gives it, which
    /// shows what is higher in the column. `most` is how far the column scrolls ([`crate::scroll_most`]).
    pub fn wheel(&mut self, dy: f32, most: f32) {
        let most = most.max(0.0);
        self.at = if dy.is_finite() { (self.at - dy).clamp(0.0, most) } else { self.at.clamp(0.0, most) };
        self.stuck = self.at >= most - 0.5;
    }

    /// The column was laid out again and now scrolls `most`: a view at its end is at the new end, and any other
    /// is where it was, or at the end when the column got shorter than that.
    pub fn fit(&mut self, most: f32) {
        let most = most.max(0.0);
        self.at = if self.stuck { most } else { self.at.min(most) };
    }
}

#[cfg(test)]
mod tests {
    use super::Scroll;

    #[test]
    fn a_page_opens_at_its_end_and_stays_there_while_the_column_grows() {
        let mut scroll = Scroll::default();
        scroll.fit(300.0);
        assert_eq!(scroll.at(), 300.0);
        scroll.fit(520.0);
        assert_eq!(scroll.at(), 520.0, "a new card is under the eyes");
        scroll.fit(0.0);
        assert_eq!(scroll.at(), 0.0, "a column that fits is not moved");
    }

    #[test]
    fn a_wheel_moves_the_column_between_its_top_and_its_end_and_a_view_moved_up_is_left_where_it_is() {
        let mut scroll = Scroll::default();
        scroll.fit(300.0);
        scroll.wheel(120.0, 300.0);
        assert_eq!(scroll.at(), 180.0);
        scroll.fit(520.0);
        assert_eq!(scroll.at(), 180.0, "the person is reading higher up: a new card does not move the page");
        scroll.wheel(1e9, 520.0);
        assert_eq!(scroll.at(), 0.0, "never past the top");
        scroll.wheel(-1e9, 520.0);
        assert_eq!(scroll.at(), 520.0, "never past the end");
        scroll.fit(700.0);
        assert_eq!(scroll.at(), 700.0, "back at the end, it follows the column again");
        scroll.wheel(100.0, 700.0);
        scroll.fit(40.0);
        assert_eq!(scroll.at(), 40.0, "a column that got shorter is not left past its end");
        scroll.wheel(f32::NAN, 40.0);
        assert_eq!(scroll.at(), 40.0, "a turn that is no number moves nothing");
    }
}
