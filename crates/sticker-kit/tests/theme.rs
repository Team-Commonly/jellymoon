//! Themes are data: defaults, ignored keys, and the two refusals.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::PathBuf;

use sticker_kit::theme::{
    MAX_ART_BYTES, MAX_ART_DEPTH, MAX_ART_ELEMENTS, MAX_THEME_BYTES, SVG_ATTRIBUTES, SVG_ELEMENTS, svg_refusal,
};
use sticker_kit::{Color, Scene, Theme, ThemeError, art};

const PICTURE: &str =
    r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><circle r="4" fill="#123456"/></svg>"##;

/// A fresh folder under the system's temporary folder, removed on drop.
struct Folder(PathBuf);

impl Folder {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("sticker-kit-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("temporary folder");
        Folder(path)
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, text).expect("file written");
        path
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn missing_keys_fall_back_to_the_sticker_theme() {
    let loaded = Theme::from_toml("[palette]\nink = \"#102030\"\n\n[motion]\nbob_amplitude = 2.5\n")
        .expect("a partial theme loads");
    let sticker = Theme::sticker();
    assert!(loaded.warnings.is_empty());
    assert_eq!(loaded.theme.palette.ink, Color::rgb(0x102030));
    assert_eq!(loaded.theme.palette.paper, sticker.palette.paper);
    assert_eq!(loaded.theme.palette.hues, sticker.palette.hues);
    assert_eq!(loaded.theme.motion.bob_amplitude, 2.5);
    assert_eq!(loaded.theme.motion.bob_period, sticker.motion.bob_period);
    assert_eq!(loaded.theme.fonts, sticker.fonts);
    assert_eq!(loaded.theme.metrics, sticker.metrics);

    let empty = Theme::from_toml("").expect("an empty theme loads");
    assert_eq!(empty.theme, Theme::default());
}

#[test]
fn an_unknown_key_is_ignored_named_and_defaulted() {
    let text = "sparkle = true\n\n[palette]\nink = \"#102030\"\nglitter = \"#FFFFFF\"\n\n\
[[palette.hues]]\nbase = \"#000000\"\nshine = 3\n\n[future]\nanything = 1\n";
    let loaded = Theme::from_toml(text).expect("a theme from a newer kit still loads");
    let named = |key: &str| loaded.warnings.iter().any(|w| w.starts_with(&format!("{key}:")));
    assert!(named("sparkle"), "{:?}", loaded.warnings);
    assert!(named("palette.glitter"), "{:?}", loaded.warnings);
    assert!(named("palette.hues[0].shine"), "{:?}", loaded.warnings);
    assert!(named("future"), "{:?}", loaded.warnings);
    assert_eq!(loaded.warnings.len(), 4);
    assert_eq!(loaded.theme.palette.ink, Color::rgb(0x102030));
    assert_eq!(loaded.theme.palette.hues[0].base, Color::rgb(0x000000));
    assert_eq!(loaded.theme.palette.hues[0].dark, Theme::sticker().palette.hues[0].dark);
    assert_eq!(loaded.theme.paper, Theme::sticker().paper);
}

#[test]
fn a_missing_piece_uses_the_built_in_art() {
    let folder = Folder::new("missing");
    folder.write("star.svg", PICTURE);
    let path = folder.write("theme.toml", "[art]\nstar = \"star.svg\"\nbadge = \"not-there.svg\"\n");
    let loaded = Theme::load(&path).expect("loads");
    assert_eq!(loaded.theme.name, "theme");
    let only = vec![("star", PICTURE.to_string())];
    assert_eq!(loaded.theme.art_svgs(), only, "not `badge`, a named file that is absent, nor a piece not named");
    assert!(loaded.warnings.iter().any(|w| w.starts_with("art.badge:")), "{:?}", loaded.warnings);
}

#[test]
fn a_path_out_of_the_theme_folder_is_refused() {
    for name in ["../outside.svg", "/outside.svg", "art/../../outside.svg"] {
        let text = format!("[art]\nstar = \"{name}\"\n");
        match Theme::from_toml(&text) {
            Err(ThemeError::PathEscapes { key, path }) => {
                assert_eq!((key.as_str(), path.as_str()), ("art.star", name));
            }
            other => panic!("{name} was not refused: {other:?}"),
        }
    }
}

#[cfg(unix)]
#[test]
fn a_symlink_out_of_the_theme_folder_is_refused() {
    let outside = Folder::new("outside");
    let target = outside.write("secret.svg", PICTURE);
    let folder = Folder::new("linked");
    std::os::unix::fs::symlink(&target, folder.0.join("star.svg")).expect("symlink made");
    let path = folder.write("theme.toml", "[art]\nstar = \"star.svg\"\n");
    assert!(matches!(Theme::load(&path), Err(ThemeError::PathEscapes { .. })));
}

// Each picture states a size and holds a shape, so it is refused for what it names and for nothing else.
#[test]
fn an_svg_that_could_run_code_is_refused() {
    let xml = "it is not XML the renderer reads: ";
    let scripted = [
        (r#"<script>alert(1)</script>"#, "it contains the element `<script`"),
        (r#"<circle r="4" onload="alert(1)"/>"#, "it contains the attribute `onload`"),
        (r#"<circle r="4" OnClick = "x()"/>"#, "it contains the attribute `OnClick`"),
        (r#"<image href="https://example.com/a.png"/>"#, "it contains the element `<image`"),
        (
            r#"<use xmlns:xlink="http://www.w3.org/1999/xlink" xlink:href="other.svg#a"/>"#,
            "it contains the element `<use`",
        ),
        (r#"<use xlink:href="other.svg#a"/>"#, xml),
        (r#"<a href="javascript:x()"><circle r="4"/></a>"#, "it contains the element `<a`"),
        (r#"<foreignObject><p>hi</p></foreignObject>"#, "it contains the element `<foreignObject`"),
        (r#"<style>@import url(x.css);</style>"#, "it contains the element `<style`"),
        (r#"<circle r="4"onclick="x()"/>"#, xml),
        (r#"<circle r="4" xml:base="https://example.com/"/>"#, "it contains the attribute `base`"),
        (r##"<circle r="4"><set attributeName="r" to="9"/></circle>"##, "it contains the element `<set`"),
        (r#"<p xmlns="http://www.w3.org/1999/xhtml">hi</p>"#, "its `<p` is not in SVG's namespace"),
    ];
    let whole = [
        (
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8" onload="alert(1)"><circle r="4"/></svg>"#,
            "it contains the attribute `onload`",
        ),
        (
            r#"<!DOCTYPE svg [<!ENTITY a SYSTEM "file:///etc/hosts">]><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8"><circle r="4"/>&a;</svg>"#,
            xml,
        ),
        (
            r#"<!DOCTYPE svg SYSTEM "https://example.com/svg.dtd"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8"><circle r="4"/></svg>"#,
            xml,
        ),
        (r#"<svg/onload=alert(1) viewBox="0 0 8 8"><circle r="4"/></svg>"#, xml),
        (r#"<svg viewBox="0 0 8 8"><circle r="4"/></svg>"#, "its `<svg` is not in SVG's namespace"),
    ];
    let scripted = scripted.map(|(inside, why)| {
        (format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8"><circle r="4"/>{inside}</svg>"#), why)
    });
    let all = scripted.into_iter().chain(whole.map(|(svg, why)| (svg.to_string(), why)));
    for (i, (svg, why)) in all.enumerate() {
        let said = svg_refusal(&svg).unwrap_or_else(|| panic!("case {i} was accepted"));
        assert!(said.starts_with(why) && (why != xml || !said.contains('\n')), "case {i}: {said}");
        let folder = Folder::new(&format!("scripted-{i}"));
        folder.write("star.svg", &svg);
        let path = folder.write("theme.toml", "[art]\nstar = \"star.svg\"\n");
        match Theme::load(&path) {
            Err(ThemeError::RefusedSvg { key, reason }) => assert_eq!((key.as_str(), &reason), ("art.star", &said)),
            other => panic!("case {i} was not refused: {other:?}"),
        }
    }
}

// The renderer follows nothing out of a file: it has no network, and the only element that names a file to load is
// `image`, which is not on the list. A paint that is an address is a paint it cannot find in the file, and it
// paints a plain colour for it. So the picture is counted as it is drawn: one fill.
#[test]
fn an_address_in_a_value_is_followed_nowhere_and_is_painted_as_a_plain_colour() {
    let picture = |inside: &str| format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8">{inside}</svg>"#);
    for fill in ["url(https://example.com/p)", "url(x.svg#a)", "&#117;rl(x.svg#a)", "url( x.svg#a)", r"\75rl(x.svg#a)"]
    {
        let svg = picture(&format!(r#"<rect width="8" height="8" fill="{fill}"/>"#));
        let cost = sticker_kit::theme::art_cost(&svg).unwrap_or_else(|why| panic!("{fill}: {why}"));
        assert_eq!(cost.paint, 64 + 16 * 16, "{fill}: one fill, and no gradient or pattern");
    }
    let inner = r##"<defs><linearGradient id="g"><stop offset="0" stop-color="#fff"/><stop offset="1"/></linearGradient></defs><rect width="8" height="8" fill="url(#g)"/><?php echo 1 ?>"##;
    assert_eq!(
        sticker_kit::theme::art_cost(&picture(inner)).map(|cost| cost.paint),
        Ok(4 * (64 + 16 * 16)),
        "inside the file"
    );
}

#[test]
fn the_kit_s_own_art_is_small_and_is_not_what_a_theme_file_may_hold() {
    for theme in [Theme::sticker(), Theme::plain()] {
        let p = &theme.palette;
        let pieces = [
            art::character(p, theme.hue(1), sticker_kit::Mood::Cross, true, 92.0, 5.0),
            art::star(p, 10.0, 5.0),
            art::bubble(p, 11.0, 5.0),
            art::badge(p, 15.0, 5.0),
        ];
        for svg in pieces {
            let elements = svg.matches('<').count() - svg.matches("</").count();
            assert!(elements < 40, "{elements} elements");
            assert_eq!(svg_refusal(&svg).as_deref(), Some("it contains the element `<filter`"), "only the kit blurs");
        }
    }
}

#[test]
fn plain_is_level_and_still_when_idle() {
    let plain = Theme::plain();
    assert!(!plain.tilt.enabled);
    let t = plain.tilt;
    assert_eq!([t.date_tape, t.note_tape, t.list_note, t.choice_note, t.plain_note], [0.0; 5]);
    assert_eq!(plain.motion.bob_amplitude, 0.0);
    assert_eq!(plain.motion.twinkle, 0.0);
    assert_eq!(plain.motion.pulse, 0.0);
    assert!(!plain.paper.grid);
    assert!(Theme::sticker().tilt.enabled && Theme::sticker().motion.bob_amplitude > 0.0);
}

#[test]
fn built_in_themes_load_from_their_own_serialised_form() {
    for theme in [Theme::sticker(), Theme::plain()] {
        let text = theme.to_toml().expect("serialises");
        let loaded = Theme::from_toml(&text).expect("loads back");
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
        assert_eq!(loaded.theme, theme);
        assert_eq!(Theme::builtin(&theme.name), Some(theme));
    }
}

#[test]
fn a_scene_survives_serialisation() {
    let scene = Scene::sample();
    let text = toml::to_string(&scene).expect("serialises");
    let back: Scene = toml::from_str(&text).expect("loads back");
    assert_eq!(back, scene);
}

#[test]
fn a_number_that_is_not_finite_or_is_out_of_its_range_is_refused_by_key() {
    let wrong = [
        ("[metrics]\nheight = inf\n", "metrics.height"),
        ("[metrics]\nmargin_y = -inf\n", "metrics.margin_y"),
        ("[metrics]\nwidth = nan\n", "metrics.width"),
        ("[metrics]\nheight = 3e7\n", "metrics.height"),
        ("[metrics]\nheight = 30000000\n", "metrics.height"),
        ("[metrics]\nheight = 199.0\n", "metrics.height"),
        ("[metrics]\ncolumns = 0\n", "metrics.columns"),
        ("[metrics]\ncolumns = 13\n", "metrics.columns"),
        ("[metrics]\nmax_scale = 0.0\n", "metrics.max_scale"),
        ("[metrics]\nnote_width = 10001\n", "metrics.note_width"),
        ("[paper]\ngrid_step = -1.0\n", "paper.grid_step"),
        ("[tilt]\nnote_tape = 90.0\n", "tilt.note_tape"),
        ("[motion]\nbob_period = inf\n", "motion.bob_period"),
        ("[labels]\nmax_chars = 201\n", "labels.max_chars"),
        ("[labels]\nfont_size = -nan\n", "labels.font_size"),
    ];
    for (text, name) in wrong {
        match Theme::from_toml(text) {
            Err(ThemeError::OutOfRange { key, .. }) => assert_eq!(key, name),
            other => panic!("{text:?} was not refused: {other:?}"),
        }
    }
    let edge = "[metrics]\nheight = 10000\nwidth = 200.0\ncolumns = 12\n[tilt]\nnote_tape = -45.0\n";
    assert!(Theme::from_toml(edge).is_ok(), "the ends of a range are inside it");
    let said = Theme::from_toml("[metrics]\nheight = inf\n").unwrap_err().to_string();
    assert_eq!(said, "metrics.height = inf is not a number from 200 to 10000");
}

#[test]
fn a_theme_or_art_file_over_its_size_or_not_a_regular_file_is_refused() {
    let big = format!("# {}\n", "x".repeat(MAX_THEME_BYTES as usize));
    assert!(matches!(Theme::from_toml(&big), Err(ThemeError::Read(why)) if why == "it is over 256 KiB"));
    let folder = Folder::new("sizes");
    let path = folder.write("big.toml", &big);
    assert!(matches!(Theme::load(&path), Err(ThemeError::Read(why)) if why == "it is over 256 KiB"));
    assert!(matches!(Theme::load(&folder.0), Err(ThemeError::Read(why)) if why == "it is not a regular file"));

    let padded = format!("{PICTURE}<!-- {} -->", "x".repeat(MAX_ART_BYTES as usize));
    folder.write("star.svg", &padded);
    fs::create_dir_all(folder.0.join("badge.svg")).unwrap();
    for (key, why) in [("star", "it is over 64 KiB"), ("badge", "it is not a regular file")] {
        let path = folder.write("theme.toml", &format!("[art]\n{key} = \"{key}.svg\"\n"));
        match Theme::load(&path) {
            Err(ThemeError::RefusedSvg { key: named, reason }) => {
                assert_eq!((named, reason.as_str()), (format!("art.{key}"), why))
            }
            other => panic!("{key} was not refused: {other:?}"),
        }
    }
}

#[test]
fn a_bare_file_name_loads_from_the_folder_the_program_runs_in() {
    // Cargo runs a test in its package's folder, where this file always is. It is TOML with no key a theme has.
    let loaded = Theme::load(std::path::Path::new("Cargo.toml")).expect("loads");
    assert_eq!(loaded.theme.folder(), Some(std::path::Path::new(".")));
    assert_eq!(loaded.theme.name, "Cargo");
}

/// One picture with `inside` as its content.
fn svg(inside: &str) -> String {
    format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40" viewBox="0 0 40 40">{inside}</svg>"#)
}

#[test]
fn an_svg_may_hold_only_what_the_built_in_art_is_made_of() {
    let refused = [
        (r##"<defs><circle id="a" r="4"/></defs><use href="#a"/>"##, "the element `<use`"),
        (r#"<symbol id="a"><circle r="4"/></symbol>"#, "the element `<symbol`"),
        (r#"<pattern id="p" width="1" height="1"><circle r="4"/></pattern>"#, "the element `<pattern`"),
        (r#"<mask id="m"><circle r="4"/></mask>"#, "the element `<mask`"),
        (r#"<clipPath id="c"><circle r="4"/></clipPath>"#, "the element `<clipPath`"),
        (r#"<image width="4" height="4"/>"#, "the element `<image`"),
        (r#"<foreignObject><p>hi</p></foreignObject>"#, "the element `<foreignObject`"),
        (r#"<text x="1" y="1">hi</text>"#, "the element `<text`"),
        (r#"<svg width="4" height="4"><circle r="4"/></svg>"#, "a second `<svg`"),
        (r#"<filter id="f"><feGaussianBlur stdDeviation="2"/></filter>"#, "the element `<filter`"),
        (r#"<feTurbulence baseFrequency="0.9"/>"#, "the element `<feTurbulence`"),
        (r#"<g filter="url(#f)"/>"#, "the attribute `filter`"),
        (r#"<g opacity="0.5"/>"#, "the attribute `opacity`"),
        (r#"<g clip-path="url(#c)"/>"#, "the attribute `clip-path`"),
        (r#"<g mask="url(#m)"/>"#, "the attribute `mask`"),
        (r#"<path d="M0 0L4 4" stroke-miterlimit="4000"/>"#, "the attribute `stroke-miterlimit`"),
        (r#"<circle r="5000"/>"#, "reaches more than 4096 past the picture's edge"),
        (r#"<path d="M0 0L-99999 4L0 4"/>"#, "reaches more than 4096 past the picture's edge"),
        (
            r##"<path d="M0 0L4 4" stroke="#000" stroke-width="5E4"/>"##,
            "reaches more than 4096 past the picture's edge",
        ),
        (r#"<style>circle { fill: red }</style>"#, "the element `<style`"),
        (r#"<style>@\69mport 'x.css';</style>"#, "the element `<style`"),
        (r#"<circle r="4" style="fill: red"/>"#, "the attribute `style`"),
        (r#"<circle r="4" style="fill: url/**/(x.svg#a)"/>"#, "the attribute `style`"),
        (r##"<linearGradient id="g" href="#h"/>"##, "the attribute `href`"),
        (r##"<linearGradient id="g" xlink:href="#h"/>"##, "it is not XML the renderer reads"),
        (r##"<path d="M0 0 H40" stroke="#000" stroke-dasharray="0.001"/>"##, "the attribute `stroke-dasharray`"),
        (r#"<circle r=4 />"#, "it is not XML the renderer reads"),
        (r#"<circle r="4" fill/>"#, "it is not XML the renderer reads"),
        (r#"<circle r="4"#, "it is not XML the renderer reads"),
    ];
    for (inside, why) in refused {
        let said = svg_refusal(&svg(inside)).unwrap_or_else(|| panic!("{inside} was accepted"));
        assert!(said.contains(why), "{inside}: {said}");
    }
}

#[test]
fn an_svg_is_held_to_the_size_the_renderer_gives_it_and_a_flat_picture_passes() {
    let sized = |size: &str| format!(r#"<svg xmlns="http://www.w3.org/2000/svg" {size}><circle r="4"/></svg>"#);
    for (size, why) in [
        (r#"width="257" height="40""#, "the renderer sizes it 257 by 40: a picture is at most 256 a side"),
        (r#"width="40" height="100000""#, "the renderer sizes it 40 by 100000: a picture is at most 256 a side"),
        (r#"width="1e3" height="40""#, "the renderer sizes it 1000 by 40: a picture is at most 256 a side"),
        (r#"viewBox="0 0 40 257""#, "the renderer sizes it 40 by 257: a picture is at most 256 a side"),
        (r#"width="0" height="40""#, "it is not a picture the renderer reads: "),
        (r#"width="-40" height="40""#, "it is not a picture the renderer reads: "),
    ] {
        let said = svg_refusal(&sized(size)).unwrap_or_else(|| panic!("{size} was accepted"));
        assert!(said.starts_with(why), "{size}: {said}");
    }
    for size in [
        r#"width="256" height="256px""#,
        r#"viewBox="-128 -128.5 256 8""#,
        r#"width="9" viewBox="0 0 8 8""#,
        // With no `viewBox`, a side left out is as large as what the picture holds: it is counted at that size.
        "",
        r#"width="40""#,
        r#"height="40px""#,
    ] {
        assert_eq!(svg_refusal(&sized(size)), None, "{size}");
    }
    let fine = r##"<title>a &amp; b</title><desc>curl</desc><defs>
<radialGradient id="g" cx="0.5" gradientUnits="objectBoundingBox"><stop offset="0" stop-color="#fff" stop-opacity="0.5"/><stop offset="1" stop-color="#000"/></radialGradient></defs>
<g transform="translate(40 -40) rotate(-30 4.5 .5) scale(2)"><path d="M0 0L4 4.5.5-3Z" fill="url(#g)" fill-rule="evenodd"/><line x1="0" y1="0" x2="4" y2="4" stroke="#1e90ff" stroke-width="3"/>
<polyline points="0,0 1,1 0,1"/><polygon points="0,0 1,1 2,0"/><ellipse rx="2" ry="1"/><rect x="1" y="1" width="2" height="2" rx="1"/></g>"##;
    assert_eq!(svg_refusal(&svg(fine)), None);
    assert_eq!((SVG_ELEMENTS.split(' ').count(), SVG_ATTRIBUTES.split_whitespace().count()), (15, 32));
}

#[test]
fn an_svg_is_held_to_a_number_of_elements_and_a_depth() {
    assert_eq!((MAX_ART_ELEMENTS, MAX_ART_DEPTH), (100, 12));
    // The `<svg` is one element and one level.
    let many = |count: usize| svg(&r#"<circle r="4"/>"#.repeat(count - 1));
    assert_eq!(svg_refusal(&many(100)), None);
    assert_eq!(svg_refusal(&many(101)).as_deref(), Some("it has more than 100 elements"));
    let deep = |levels: usize| svg(&format!("{}{}", "<g>".repeat(levels - 1), "</g>".repeat(levels - 1)));
    assert_eq!(svg_refusal(&deep(12)), None);
    assert_eq!(svg_refusal(&deep(13)).as_deref(), Some("its elements are nested more than 12 deep"));
    let side_by_side = svg(&"<g><g><g></g></g></g>".repeat(30));
    assert_eq!(svg_refusal(&side_by_side), None, "depth is how far in, not how many");
}
