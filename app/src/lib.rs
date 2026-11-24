//! The shell (ADR 0011, section 4): it draws page data with the kit and decides nothing.
//!
//! - [`read`]: a page-data file into core's page data, plus what the file held that this app has no piece for.
//! - [`scene`]: page data into the kit's scene. Pure, and no GPUI.
//! - [`lines`]: the scene laid out and printed, one placed piece a line, measured from the kit's bundled fonts.
//! - [`taps`]: what a tap on a day's page does: a ruling through core, or the delivery hook turned on.
//! - [`wishing`]: a wish typed in the window: kept by core, its run asked of core, and the page looked at again.
//! - [`working`]: a tap on a little one opens its wish, and work that waits is shown, kept or tossed through core.
//! - `live`: what a made wish's run is doing while it works, drawn from core's steps over its little one.
//! - [`answering`]: what is typed under a made wish: an answer to its run's question, or a follow-up, through core.
//! - [`starting`]: a folder dropped or chosen, or a new project named, given to core as a project.
//! - [`today`]: the day's page, kept in the user's own folder and read again through core when the day has moved on.
//! - [`showing`]: what the window does with a key, a tap, Enter, a reading step and a look, with no window.
//! - [`window`]: the kit's view of the scene in a window (feature `window`): wiring only.
//! - [`jelly`]: the jellyfish and its bubble as printed pieces, and [`popup`]: what its two windows are asked for.
//! - [`jellyfish`] and [`shell`]: those two windows opened, and the app's windows as one thing (feature `window`).

/// The smallest size a page is drawn at. The fullest page core can write fits it; below it the text is too small.
pub const SMALLEST: sticker_kit::Viewport = sticker_kit::Viewport { w: 960.0, h: 640.0 };

/// The longest side `--size` takes, in pixels.
pub const LARGEST_SIDE: f32 = 16384.0;

/// The most characters [`one_line`] says of a text: a theme file's key can be as long as the file.
pub const LONGEST_LINE: usize = 300;

/// The first and the last line of `text` as one line of text and no more, for the terminal: a control character
/// is a space, and what turns a line round or is not seen (the kit's `clean`) is left out. A TOML error is several
/// lines: where it is, the text there, and what is wrong. A theme file's key can hold any character, and as many
/// as the file has room for: past [`LONGEST_LINE`] characters the line is cut, between two characters a reader
/// sees, and ends in an ellipsis.
pub fn one_line(text: &str) -> String {
    let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
    let first = lines.next().unwrap_or_default();
    let line = lines.next_back().map_or(first.to_string(), |last| format!("{first}: {last}"));
    sticker_kit::text::clean(&line.replace(char::is_control, " "), LONGEST_LINE)
}

/// `text` as [`one_line`], and of that only what follows its last `/`, `\` or `:`: a part of a file's name that
/// names no folder and no drive on any system. The picture example names a picture after a theme file's `name`.
pub fn file_part(text: &str) -> String {
    one_line(text).rsplit(['/', '\\', ':']).next().unwrap_or_default().to_string()
}

/// `1440x900` as a size: each side a whole number of pixels written in digits only, from its `least` to
/// [`LARGEST_SIDE`]. The one rule for `--size` and `--screen`.
pub fn sides(given: &str, least: sticker_kit::Viewport) -> Option<sticker_kit::Viewport> {
    let (w, h) = given.split_once('x')?;
    let side = |text: &str, least: f32| {
        let digits = !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
        let side = text.parse::<u16>().ok().filter(|_| digits).map(f32::from)?;
        (least..=LARGEST_SIDE).contains(&side).then_some(side)
    };
    Some(sticker_kit::Viewport { w: side(w, least.w)?, h: side(h, least.h)? })
}

/// The most characters of a wrong `--size` or `--screen` that are said back.
pub const ECHOED: usize = 40;

/// A value a person typed, said back in a refusal: cleaned, and cut at [`ECHOED`] characters with an ellipsis.
pub fn echoed(given: &str) -> String {
    sticker_kit::text::clean(given, ECHOED)
}

pub mod answering;
pub mod column;
pub mod jelly;
#[cfg(feature = "window")]
pub mod jellyfish;
pub mod lines;
mod live;
mod placing;
pub mod popup;
pub mod read;
pub mod scene;
#[cfg(feature = "window")]
pub mod shell;
pub mod showing;
pub mod starting;
pub mod stopping;
pub mod taps;
pub mod today;
mod told;
#[cfg(feature = "window")]
pub mod window;
pub mod wishing;
pub mod working;

#[cfg(test)]
mod tests {
    use sticker_kit::Viewport;

    use super::{LONGEST_LINE, echoed, file_part, one_line, sides};

    #[test]
    fn a_size_is_two_sides_in_digits_each_from_its_least_to_the_largest() {
        let (one, page) = (Viewport { w: 1.0, h: 1.0 }, crate::SMALLEST);
        assert_eq!(sides("1440x900", one), Some(Viewport { w: 1440.0, h: 900.0 }));
        assert_eq!(sides("1x16384", one), Some(Viewport { w: 1.0, h: 16384.0 }));
        assert_eq!(sides("960x640", page), Some(page));
        for wrong in ["", "x", "1440", "0x900", "1440x0", "16385x900", "+5x+5", "1440x+900", " 1x1", "1x1 ", "1.5x2"] {
            assert_eq!(sides(wrong, one), None, "{wrong:?}");
        }
        assert_eq!((sides("959x640", page), sides("960x639", page), sides("1440x900x2", one)), (None, None, None));
        assert_eq!(echoed(&"9".repeat(41)), format!("{}…", "9".repeat(40)));
        assert_eq!(echoed("a\u{1b}[31m\nb"), "a[31m b");
    }

    #[test]
    fn a_name_for_a_file_is_one_clean_line_with_no_folder_or_drive_in_it() {
        assert_eq!(file_part("dusk"), "dusk");
        assert_eq!(file_part("a\u{1b}[31m\u{202e}/../../up"), "up");
        assert_eq!(file_part("..\\..\\up"), "up");
        assert_eq!(file_part("C:up"), "up");
        assert_eq!(file_part("/rooted/"), "");
        assert_eq!(file_part("one\ntwo/three\u{7}"), "three");
        assert_eq!(file_part(&"x".repeat(5000)).chars().count(), LONGEST_LINE + 1);
        assert_eq!(one_line(&"x".repeat(LONGEST_LINE)), "x".repeat(LONGEST_LINE), "a line of the length is whole");
    }
}
