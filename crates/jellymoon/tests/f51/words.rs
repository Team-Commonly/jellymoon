//! Every last word the made-up agent can end a turn with (`support/fake_agent/asks.rs`), and what core makes of
//! it: a question is kept as the door leaves it, and of a last word that fails no word is kept anywhere.

use serde_json::{Value, json};

use super::{Made, QUESTION, asks, failed, played};
use crate::support::{map_key, shows};

/// Why a run did not finish, by its last word.
const NO_WORD: &str = "its result did not say whether it is done or asks";
const UNFIT: &str = "its last word was not `done` alone, or `asks` with one question";
const LONG: &str = "its question was longer than Jellymoon reads";
const EMPTY: &str = "nothing was left of its question once it was cleaned";
const CHANGING: &str = "its question could not be cleaned";

/// The question in each of the four places a run that asks leaves it.
fn everywhere(made: &Made, said: &Value) -> [Value; 4] {
    let (record, wish, on_page) = (made.record(), made.w.kept(&made.wish), made.w.on_page(&made.wish));
    [said["question"].clone(), record["question"].clone(), wish["question"].clone(), on_page["question"].clone()]
}

#[test]
fn a_question_is_kept_as_the_door_leaves_it_on_one_line_and_cut_at_300_characters() {
    let cut = format!("Which {}uploa…", "uploader ".repeat(32));
    let kept = [
        (0, QUESTION.to_string()),
        (4, "Which uploader did you mean?".to_string()),
        (5, cut),
        (7, format!("{}…", "q".repeat(299))),
        (8, "Is [redacted] the key to use?".to_string()),
        (9, "Which uploader? The old one or the new one?".to_string()),
        (11, "Did you mean 'retry' or 'resume'?".to_string()),
    ];
    for (number, question) in kept {
        let (made, said) = played("asks", number);
        assert_eq!(said, asks(&made, &question), "question {number}");
        assert_eq!(everywhere(&made, &said), [(); 4].map(|()| json!(question)), "question {number}");
        assert!(question.chars().count() <= 300 && !question.contains(['\n', '\r', '\t', '"']), "{question:?}");
        // A credential's shape is in no file of the home and in nothing printed.
        let key = map_key("kLmNo");
        let held = crate::support::tree(&made.w.at("jm"))
            .into_iter()
            .find(|(_, bytes)| shows(&String::from_utf8_lossy(bytes), &key));
        assert!(held.is_none() && !shows(&said.to_string(), &key), "question {number}: {:?}", held.map(|held| held.0));
    }
    // The control of the cut: the over-long question is 300 characters with its mark, and none of its end is kept.
    let (_, said) = played("asks", 5);
    let question = said["question"].as_str().unwrap();
    assert!(question.chars().count() == 300 && !question.contains("mean"), "{question}");
}

#[test]
fn a_last_word_that_fails_is_a_failed_run_and_no_word_of_it_is_kept_or_printed() {
    let ends = [
        ("asks", 2, EMPTY),
        ("asks", 3, EMPTY),
        ("asks", 6, LONG),
        ("asks", 10, CHANGING),
        ("word", 0, NO_WORD),
        ("word", 1, NO_WORD),
        ("word", 2, UNFIT),
        ("word", 3, UNFIT),
        ("word", 4, UNFIT),
        ("word", 5, UNFIT),
        ("word", 6, UNFIT),
        ("word", 7, UNFIT),
        ("word", 8, UNFIT),
        ("word", 9, UNFIT),
        ("asks-failed", 0, "its result was not a success"),
        ("asks-failed", 1, "its result was not a success"),
        ("asks-failed", 2, "Claude Code exited with a failure after its result"),
        ("asks-failed", 3, "it gave more than one result"),
        ("asks-failed", 4, "a line that could not be read came after its result"),
    ];
    for (script, number, kind) in ends {
        let (made, said) = played(script, number);
        assert_eq!(said, failed(&made, kind), "{script} {number}");
        let (record, wish) = (made.record(), made.w.kept(&made.wish));
        assert_eq!((&record["outcome"], &record["why"], &wish["why"]), (&json!("failed"), &said["why"], &said["why"]));
        let none = (record.get("question"), wish.get("question"), made.w.on_page(&made.wish).get("question").cloned());
        assert_eq!(none, (None, None, None), "{script} {number}");
        // No word of the question is in any file of the home: not the plain one, a long one's letters or a key.
        for words in ["two uploaders", "the key", &"q".repeat(16), "kLmNokLmNo", "redacted"] {
            assert_eq!(made.keeps(words), None, "{script} {number}: {words}");
        }
        // It is a failed run as any other: its copy is kept as one, with its work, and nothing was committed.
        let copy = std::fs::read_to_string(made.w.jm("runs").join(&made.run).join("copy.json")).unwrap();
        assert!(serde_json::from_str::<Value>(&copy).unwrap()["failed"].is_u64(), "{script} {number}: {copy}");
        assert_eq!(made.w.commit(&made.repo, &made.branch()), made.w.commit(&made.repo, "main"));
        // Its end is one core wrote, in these words: a toss goes by the record, and does not refuse it.
        assert_eq!(crate::printed(&made.toss())["tossed"], true, "{script} {number}");
    }
    // The control: of a question that is kept, the same search finds the record and the wishes.
    let (made, _) = played("asks", 0);
    assert!(made.keeps("two uploaders").is_some());
    let (made, _) = played("asks", 8);
    assert!(made.keeps("redacted").is_some());
}

// A run whose last word is `done` finishes only as it did before: by a success, an exit of 0 and nothing after.
#[test]
fn done_finishes_a_run_only_with_a_success_and_an_exit_of_0() {
    for (script, kind) in [
        ("not-success", "its result was not a success"),
        ("is-error", "its result was not a success"),
        ("exit-3", "Claude Code exited with a failure after its result"),
    ] {
        let (made, said) = played(script, 0);
        assert_eq!(said, failed(&made, kind), "{script}");
    }
    let (_, said) = played("nothing", 0);
    assert_eq!(said["outcome"], "completed");
}
