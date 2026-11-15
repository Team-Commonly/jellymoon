//! The third step: the words so far, a heard wish's words, a project's name and a run's question are text and
//! nothing else. Each is cut to the bubble with an ellipsis, between two characters a reader sees; a question is
//! drawn apart from Jellymoon's own words; and markup, a link's shape, an escape or a control character in any of
//! them changes the text that is drawn and no piece.

use jellymoon_core::EarCannot;
use serde_json::Value;

use crate::jelly::{ASKS, TAKE_BACK, asks, at, cannot, drawn, heard, inside, listening};
use crate::{QUESTION, SAID};

/// The lines a text of one role is drawn on.
fn lines(line: &Value, role: &str) -> Vec<String> {
    let shown = drawn(line);
    let of_role = shown.runs().into_iter().filter(|run| run["role"] == role);
    of_role.map(|run| run["text"].as_str().unwrap().to_string()).collect()
}

/// A drawn text that was cut: its lines joined as they were broken, without the ellipsis it ends in.
fn cut(lines: &[String]) -> String {
    let last = lines.last().unwrap();
    assert!(last.ends_with('…') && lines.iter().filter(|line| line.contains('…')).count() == 1, "{lines:?}");
    lines.concat().trim_end_matches('…').to_string()
}

#[test]
fn words_a_name_and_a_question_longer_than_the_bubble_are_cut_with_an_ellipsis() {
    let long = "the uploader should retry twice and then give up and say so ".repeat(4);
    let long = long.trim_end();
    // The words so far and a heard wish's words: three lines. A project's name: one. A question: three.
    let cases = [
        (listening(long), "row_main", 3),
        (heard(long, "alpha", 4000), "row_main", 3),
        (heard(SAID, &"long-project-name-".repeat(5), 4000), "row_sub", 1),
        (asks(long), "quoted", 3),
    ];
    for (line, role, most) in cases {
        let drawn = lines(&line, role);
        assert_eq!(drawn.len(), most, "{role}: {drawn:?}");
        let kept = cut(&drawn);
        let whole = if role == "row_sub" { "long-project-name-".repeat(5) } else { long.to_string() };
        let spaceless = |text: &str| text.replace(' ', "");
        assert!(spaceless(&whole).starts_with(&spaceless(&kept)) && kept.len() * 2 > whole.len() / 3, "{kept}");
    }
    // What is cut is still said whole by what follows it: a heard wish keeps its project and its sentence.
    let shown = drawn(&heard(long, &"long-project-name-".repeat(5), 4000));
    assert_eq!(shown.text("body"), TAKE_BACK);
    assert_eq!(drawn(&asks(long)).text("body"), ASKS);
    // Every run of a cut text is inside the bubble: nothing is drawn past its edge.
    let bubble = shown.of("bubble")[0].rect();
    shown.runs().iter().for_each(|run| assert!(inside(at(run), bubble), "{run}"));
    // Short words are drawn whole, with no ellipsis.
    assert_eq!(lines(&listening(SAID), "row_main").concat().replace(' ', ""), SAID.replace(' ', ""));
}

#[test]
fn a_cut_never_falls_inside_a_character_a_reader_sees() {
    // A letter with its accent written as two code points, a family joined of four, and a flag of two.
    let (accented, family, flag) = ("e\u{301}", "👩\u{200d}👩\u{200d}👧\u{200d}👦", "🇯🇵");
    for (one, least) in [(accented, 60), (family, 6), (flag, 12)] {
        let many = one.repeat(380 / one.chars().count());
        for (line, role) in [(listening(&many), "row_main"), (asks(&many), "quoted")] {
            let drawn = lines(&line, role);
            assert_eq!(drawn.len(), 3, "{drawn:?}");
            for text in &drawn {
                let text = text.trim_end_matches('…');
                let whole = text.len() / one.len();
                assert_eq!(text, one.repeat(whole), "a line holds whole characters only");
            }
            assert!(cut(&drawn).len() / one.len() >= least, "{drawn:?}");
        }
    }
}

#[test]
fn a_question_is_a_text_of_its_own_apart_from_jellymoon_s_words() {
    let shown = drawn(&asks(QUESTION));
    let runs = shown.runs();
    let (own, asked): (Vec<&Value>, Vec<&Value>) = runs.iter().partition(|run| run["role"] == "body");
    assert!(!own.is_empty() && asked.iter().all(|run| run["role"] == "quoted") && !asked.is_empty(), "{runs:?}");
    // No run holds a word of both, and every line of the question is under every line of the sentence.
    let text = |runs: &[&Value]| runs.iter().map(|run| run["text"].as_str().unwrap()).collect::<Vec<_>>().join(" ");
    assert_eq!((text(&own), text(&asked)), (ASKS.to_string(), QUESTION.to_string()));
    let (last_own, first_asked) = (at(own[own.len() - 1]), at(asked[0]));
    let between = first_asked[1] - (last_own[1] + last_own[3]);
    let line_gap = if own.len() > 1 { at(own[1])[1] - (at(own[0])[1] + at(own[0])[3]) } else { 0.0 };
    assert!(between >= 8.0 && between > line_gap, "{between} between them, {line_gap} between two lines");
    // A question that reads like Jellymoon's own words is still drawn as the run's.
    let posed = drawn(&asks(TAKE_BACK));
    assert_eq!((posed.text("quoted"), posed.text("body")), (TAKE_BACK.to_string(), ASKS.to_string()));
}

#[test]
fn markup_a_link_an_escape_or_a_control_character_changes_nothing_but_the_text() {
    // What a log or an agent could hold: tags, a link in two shapes, an escape sequence, a bell, a line break,
    // a tab, and a mark that turns a line round.
    let nasty = "<b onclick=\"go()\">now</b> [open](https://example.test/x) \u{1b}[31mred\u{7} a\nb\tc \u{202e}tfel";
    let drawn_as = "<b onclick=\"go()\">now</b> [open](https://example.test/x) [31mred a b c tfel";
    let cases = [
        (listening(nasty), listening("plain words"), "row_main"),
        (heard(nasty, "alpha", 4000), heard("plain words", "alpha", 4000), "row_main"),
        (heard(SAID, nasty, 4000), heard(SAID, "plain", 4000), "row_sub"),
        (asks(nasty), asks("plain words"), "quoted"),
        (cannot(nasty), cannot("plain words"), "body"),
    ];
    for (line, plain, role) in cases {
        let (shown, as_plain) = (drawn(&line), drawn(&plain));
        // The same lines of the same kinds, the same windows, and no piece that a tap or a link could be.
        assert_eq!(shown.kinds(), as_plain.kinds(), "{role}");
        assert_eq!(
            (shown.window("jelly"), shown.window("bubble")),
            (as_plain.window("jelly"), as_plain.window("bubble"))
        );
        assert_eq!(shown.roles(), as_plain.roles(), "{role}");
        let text = lines(&line, role);
        let joined = if text.last().is_some_and(|last| last.ends_with('…')) { cut(&text) } else { text.concat() };
        let (joined, whole) = (joined.replace(' ', ""), drawn_as.replace(' ', ""));
        assert!(whole.starts_with(&joined) && joined.len() > 20, "{role}: {joined}");
        for run in shown.runs() {
            let unseen = |c: char| c.is_control() || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2028}' | '\u{2029}');
            assert!(!run["text"].as_str().unwrap().contains(unseen), "{run}");
            assert!(
                run.as_object()
                    .unwrap()
                    .keys()
                    .all(|key| ["role", "text", "size", "x", "y", "w", "h"].contains(&key.as_str()))
            );
        }
    }
}

#[test]
fn every_sentence_of_core_s_for_the_ear_is_drawn_whole() {
    for why in EarCannot::ALL {
        let shown = drawn(&cannot(why.says()));
        assert_eq!(shown.text("body"), why.says(), "{why:?}");
        assert_eq!(shown.roles(), ["body"]);
    }
}

// F56's review, note 6: every sentence a hearing has of its own to refuse with, the ear's among them, is held
// whole by the bubble in each built-in theme. One that grows past it fails here, and is shortened in core.
#[test]
fn every_sentence_a_hearing_refuses_with_is_drawn_whole() {
    let sentences = jellymoon_core::Hearing::refusals();
    assert!(sentences.len() >= EarCannot::ALL.len() + 6, "{sentences:?}");
    assert!(EarCannot::ALL.iter().all(|why| sentences.contains(&why.says())), "the ear's are among them");
    for sentence in sentences {
        for theme in ["sticker", "plain"] {
            let shown = crate::jelly::drawn_in(&cannot(sentence), theme);
            assert_eq!(shown.text("body"), sentence, "{theme}");
            assert!(!shown.text("body").contains('…'), "{theme}: {sentence}");
        }
    }
}
