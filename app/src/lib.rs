//! The shell (ADR 0011, section 4): it draws page data with the kit and decides nothing.
//!
//! - [`read`]: a page-data file into core's page data, plus what the file held that this app has no piece for.
//! - [`scene`]: page data into the kit's scene. Pure, and no GPUI.
//! - [`lines`]: the scene laid out and printed, one placed piece a line, measured from the kit's bundled fonts.
//! - [`taps`]: what a tap on a day's page does: a ruling through core, or the delivery hook turned on.
//! - [`wishing`]: a wish typed in the window: kept by core, its run asked of core, and the page looked at again.
//! - [`today`]: the day's page, kept in the user's own folder and read again through core when the day has moved on.
//! - [`showing`]: what the window does with a key, a tap, Enter, a reading step and a look, with no window.
//! - [`window`]: the kit's view of the scene in a window (feature `window`): wiring only.

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

pub mod lines;
pub mod read;
pub mod scene;
pub mod showing;
pub mod stopping;
pub mod taps;
pub mod today;
#[cfg(feature = "window")]
pub mod window;
pub mod wishing;

#[cfg(test)]
mod tests {
    use super::{LONGEST_LINE, file_part, one_line};

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
