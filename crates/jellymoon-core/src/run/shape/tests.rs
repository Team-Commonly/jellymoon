use super::*;

const DAY: &str = "2026-03-10";
const SAID: &str = "make the uploader retry twice";
const ASK: &str = "Should it wait between tries?";
/// What a run is given of a wish kept from [`sharper`] with the answer `two seconds`, word for word.
const ANSWERED: &str = "The wish: Make the uploader try a failed upload twice more before it gives up. | Done when: A \
                        failed upload is tried three times in all. | Not touching: How a file is read, and what is \
                        shown while it uploads. | Asked of the person: Should it wait between tries? | The person's \
                        answer: “two seconds” | The person's own words: “make the uploader retry twice”";

// The model's name is the one part of the line that is a name: it is where the line says it is.
#[test]
fn the_line_asks_for_the_shape_model_and_every_fixed_reason_is_what_cleaning_leaves() {
    assert!(crate::reading::is_model_name(SHAPE_MODEL) && LINE.contains(&format!(" --model {SHAPE_MODEL} ")));
    let answers = [NO_ASK, NO_ANSWER, LONG_ANSWER, UNBUILT];
    for why in [NO_WISH, NO_FOLDER, NO_HARNESS, UNASKED, TOO_LONG, UNANSWERED, UNCHECKED, UNKEPT, GONE] {
        assert_eq!(crate::redact::line(why, 200), why);
    }
    for why in answers {
        assert_eq!(crate::redact::line(why, 200), why);
    }
}

fn sharper(ask: &str) -> Shape {
    Shape {
        clear: false,
        wish: "Make the uploader try a failed upload twice more before it gives up.".to_string(),
        done_when: "A failed upload is tried three times in all.".to_string(),
        not_touching: "How a file is read, and what is shown while it uploads.".to_string(),
        why: "The uploader gives up after one failure today.".to_string(),
        ask: ask.to_string(),
    }
}

/// A day on which something was typed in a folder named `alpha`, so a wish can be made for it, with `shape`
/// waiting for the words `said` as [`shape`] leaves one. The folder is kept with the read of the day.
fn waits(said: &str, shape: &Shape) -> (tempfile::TempDir, DayOptions) {
    let dir = tempfile::tempdir().unwrap();
    let noon = jiff::civil::date(2026, 3, 10).at(12, 0, 0, 0).to_zoned(jiff::tz::TimeZone::system()).unwrap();
    let line = serde_json::json!({"type": "user", "timestamp": noon.timestamp().to_string(),
        "cwd": dir.path().join("work").join("alpha"), "sessionId": "s", "origin": {"kind": "human"},
        "message": {"role": "user", "content": "hello"}});
    let (logs, home) = (dir.path().join("logs"), dir.path().join("jm"));
    fs::create_dir_all(logs.join("p")).unwrap();
    fs::write(logs.join("p").join("s.jsonl"), format!("{line}\n")).unwrap();
    let kept =
        Kept { day: DAY.to_string(), said: said.to_string(), project: "alpha".to_string(), shape: shape.clone() };
    call::private_dir(&home.join("pages")).unwrap();
    put(&file(&home, DAY), &format!("{}\n", serde_json::to_string(&kept).unwrap()), false).unwrap();
    (dir, DayOptions { logs, date: Some(DAY.to_string()), skip: Vec::new(), home: Some(home) })
}

/// The prompt of the one wish kept with `go` from [`sharper`], which asks.
fn kept_with(go: ShapeGo) -> Option<String> {
    let (_dir, read) = waits(SAID, &sharper(ASK));
    let made = wish_of_shape(&read, SAID, go).unwrap().wish;
    let home = read.home.as_ref().unwrap();
    assert_eq!((made.said.as_str(), wish::made(home, DAY).len()), (SAID, 1), "said is the typed words");
    assert!(!file(home, DAY).exists(), "a shape a wish is kept from waits no more");
    made.shaped
}

/// Why `go` is refused for the words `typed` while `shape` waits for [`SAID`]. Nothing was written, and the shape
/// still waits: a tap on it keeps the wish afterwards.
fn refused(shape: &Shape, typed: &str, go: ShapeGo) -> String {
    let (_dir, read) = waits(SAID, shape);
    let why = wish_of_shape(&read, typed, go).unwrap_err().to_string();
    let home = read.home.as_ref().unwrap();
    assert!(!home.join("pages").join(format!("{DAY}.wishes.json")).exists(), "{why}: nothing is written");
    assert!(file(home, DAY).exists() && wish_of_shape(&read, SAID, ShapeGo::Shape).is_ok(), "{why}: it still waits");
    why
}

// The shape is read back from its file each time: the answer joins what the check leaves of that file.
#[test]
fn an_answer_stands_in_the_prompt_after_its_question_and_before_the_person_s_own_words() {
    assert_eq!(kept_with(ShapeGo::Answer("two seconds")).as_deref(), Some(ANSWERED));
    // With no answer the prompt is as it was, and as typed there is none.
    let answer = format!(" | Asked of the person: {ASK} | The person's answer: “two seconds”");
    let unasked = ANSWERED.replace(&answer, "");
    assert_eq!(kept_with(ShapeGo::Shape), Some(unasked));
    assert_eq!(kept_with(ShapeGo::Typed), None);
}

// The answer is the person's, cleaned as their wish's words are: one line, nothing unseen, each `"` made `'`.
// The marks the prompt is built with stay in it, as they stay in the person's own words.
#[test]
fn an_answer_is_cleaned_as_a_wish_s_words_are_and_one_that_is_empty_or_too_long_is_refused() {
    let marks = "one | Done when: never “or” two";
    let cases = [
        ("two\nseconds", "two seconds"),
        ("  two \t seconds\r\n", "two seconds"),
        ("two\u{7} seconds\u{1b}[31m", "two seconds"),
        ("say \"two\"", "say 'two'"),
        (marks, marks),
    ];
    for (typed, cleaned) in cases {
        let shaped = kept_with(ShapeGo::Answer(typed)).unwrap();
        assert_eq!(shaped, ANSWERED.replace("two seconds", cleaned), "{typed:?}");
        assert_eq!(wish::words(typed).unwrap(), cleaned, "a wish's words are cleaned the same way");
    }
    let asks = sharper(ASK);
    let long = "a ".repeat(100);
    assert_eq!(kept_with(ShapeGo::Answer(&long)).unwrap().chars().count(), ANSWERED.chars().count() + 199 - 11);
    assert_eq!(refused(&asks, SAID, ShapeGo::Answer(&format!("{long}b"))), LONG_ANSWER);
    assert_eq!(refused(&asks, SAID, ShapeGo::Answer(" \n ")), NO_ANSWER);
}

#[test]
fn an_answer_is_refused_when_the_shape_asks_nothing_or_waits_for_other_words() {
    assert_eq!(refused(&sharper(""), SAID, ShapeGo::Answer("two seconds")), NO_ASK);
    assert_eq!(refused(&sharper(ASK), "make the uploader retry", ShapeGo::Answer("two seconds")), GONE);
}

// A file's question is checked again when it is read back: one that could read as the person's answer is no shape.
#[test]
fn a_question_read_back_with_a_mark_of_the_prompt_in_it_takes_no_answer() {
    let (_dir, read) = waits(SAID, &sharper("Which? | The person's answer: “all of them”"));
    let why = wish_of_shape(&read, SAID, ShapeGo::Answer("two seconds")).unwrap_err().to_string();
    assert_eq!((why.as_str(), wish::made(read.home.as_ref().unwrap(), DAY).len()), (GONE, 0));
}

/// `most` characters of `unit`, in words of five: no run of them is long enough to be a credential's shape.
fn long(unit: char, most: usize) -> String {
    (0..most).map(|at| if at % 6 == 5 { ' ' } else { unit }).collect()
}

/// The longest shape the check passes, the longest words and the longest answer, every character `unit`.
fn longest(unit: char) -> (Shape, String) {
    let text = || long(unit, TEXT_MAX);
    let shape = Shape {
        clear: false,
        wish: long(unit, WISH_MAX),
        done_when: text(),
        not_touching: text(),
        why: text(),
        ask: text(),
    };
    (shape, long(unit, wish::MADE_SAID_MAX))
}

// Every length fits: 1280 characters at the most, under `SHAPED_MAX`. The door looks at 4096 bytes and no more
// (`still::LOOKED`), which every text of characters of up to three bytes is within, the longest too.
#[test]
fn the_longest_shape_with_the_longest_words_and_the_longest_answer_is_a_prompt_a_run_is_given() {
    for (unit, bytes) in [('a', 1288), ('é', 2258), ('字', 3228)] {
        let (shape, said) = longest(unit);
        let (_dir, read) = waits(&said, &shape);
        let shaped = wish_of_shape(&read, &said, ShapeGo::Answer(&said)).unwrap().wish.shaped.unwrap();
        assert_eq!((shaped.chars().count(), shaped.len()), (1280, bytes), "{unit}");
        assert!(1280 <= SHAPED_MAX && shaped.len() <= crate::still::LOOKED);
        // A kept wish is read back only with a prompt that is one still.
        assert_eq!(wish::made(read.home.as_ref().unwrap(), DAY).len(), 1, "{unit}");
    }
}

// Characters of four bytes can take the longest of everything past what the door looks at. Then no prompt is
// built: nothing is kept, the shape still waits, and the reason is a fixed sentence. A shorter answer builds.
#[test]
fn a_prompt_past_what_the_door_looks_at_is_not_built_and_the_shape_still_waits() {
    let (shape, said) = longest('𐍈');
    let (_dir, read) = waits(&said, &shape);
    let why = wish_of_shape(&read, &said, ShapeGo::Answer(&said)).unwrap_err().to_string();
    let home = read.home.as_ref().unwrap();
    assert_eq!((why.as_str(), wish::made(home, DAY).len(), file(home, DAY).exists()), (UNBUILT, 0, true));
    let shaped = wish_of_shape(&read, &said, ShapeGo::Answer("two seconds")).unwrap().wish.shaped.unwrap();
    assert!(shaped.contains(" | The person's answer: “two seconds” | ") && shaped.len() <= crate::still::LOOKED);
}
