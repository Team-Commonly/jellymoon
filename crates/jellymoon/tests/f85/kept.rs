//! What is kept of a last word, and what is no result (ADR 0023, decisions 1 and 2): a result never fails a run,
//! one that is not the shape or is over its size is no result, and the file a result is kept in is read as
//! anyone's.

use std::fs;

use serde_json::{Value, json};

use super::{DAY, Made, STAR, done, kept_file, result, show, told, told_exit, write_kept};
use crate::world::printed;

/// A run that made `n.txt` and ended with `word`, and what it printed, with `<run>` where it names itself.
fn ended_with(word: &str) -> (Made, Value) {
    let (made, mut said) = told(&[], &[("n.txt", &b"new from the run\n"[..])], &[], word);
    said["why"] = json!(said["why"].as_str().unwrap().replace(&made.run, "<run>"));
    (made, said)
}

fn did(text: &str) -> Value {
    json!({"type": "did", "text": text})
}

/// Fails unless a look prints the work as it did before there were results: six parts, and no `result`.
fn as_before(made: &Made) {
    let shown = printed(&show(made, &[]));
    assert_eq!((shown.as_object().unwrap().len(), &shown["files"][0]["name"]), (6, &json!("n.txt")), "{shown}");
}

#[test]
fn a_run_with_no_result_or_one_that_is_not_the_shape_has_finished_as_before_and_nothing_is_kept() {
    let (plain, said) = ended_with(r#"{"state":"done"}"#);
    assert_eq!((&said["outcome"], &said["star"]), (&json!("completed"), &json!(STAR)), "{said}");
    assert!(!kept_file(&plain).exists());
    as_before(&plain);
    let one = did("I made n.txt.").to_string();
    for word in [
        r#"{"state":"done","result":7}"#.to_string(),
        r#"{"state":"done","result":null}"#.to_string(),
        r#"{"state":"done","result":"I made n.txt."}"#.to_string(),
        r#"{"state":"done","result":[]}"#.to_string(),
        r#"{"state":"done","result":{}}"#.to_string(),
        r#"{"state":"done","result":{"pieces":"none"}}"#.to_string(),
        format!(r#"{{"state":"done","result":{{"pieces":[{one}],"more":1}}}}"#),
        r#"{"state":"done","result":{"pieces":[{"type":"checked","text":"The tests pass."}]}}"#.to_string(),
        r#"{"state":"done","result":{"pieces":[{"type":"did","text":"I made n.txt.","file":"n.txt"}]}}"#.to_string(),
        r#"{"state":"done","result":{"pieces":[{"type":"did"}]}}"#.to_string(),
        r#"{"state":"done","result":{"pieces":[{"type":"change","file":"n.txt","after":"new from the run"}]}}"#
            .to_string(),
        // Two results are not one: neither is taken.
        format!(r#"{{"state":"done","result":{{"pieces":[{one}]}},"result":{{"pieces":[{one}]}}}}"#),
    ] {
        let (made, odd) = ended_with(&word);
        assert_eq!(odd, said, "{word}");
        assert!(!kept_file(&made).exists(), "{word}");
        as_before(&made);
    }
}

#[test]
fn a_result_over_its_size_is_no_result_and_of_one_within_it_the_first_twelve_pieces_are_kept() {
    // Fifty words of five bytes and three characters each: a text within a piece's length.
    let text = "\u{e9}\u{e9} ".repeat(50);
    let pieces = |n: usize| Value::from((0..n).map(|_| json!({"type": "decided", "text": text})).collect::<Vec<_>>());
    let (within, over) = (done(&pieces(27)), done(&pieces(30)));
    assert!(within.len() < 8192 && over.len() > 8192 + 64, "{} {}", within.len(), over.len());
    let (made, said) = ended_with(&over);
    assert_eq!(said["outcome"], "completed", "{said}");
    assert!(!kept_file(&made).exists());
    as_before(&made);
    let (made, said) = ended_with(&within);
    assert_eq!(said["outcome"], "completed", "{said}");
    let kept: Value = serde_json::from_slice(&fs::read(kept_file(&made)).unwrap()).unwrap();
    assert_eq!(kept, json!({"pieces": pieces(12)}));
    let told = result(&made);
    assert_eq!((told["pieces"].as_array().unwrap().len(), &told["dropped"]), (3, &json!(0)), "{told}");
    // The thirteenth piece is not kept, so it is not printed, though its kind had room.
    let mut many = pieces(12).as_array().unwrap().clone();
    many.push(did("I made n.txt."));
    let (made, _) = ended_with(&done(&Value::from(many)));
    assert!(result(&made)["pieces"].as_array().unwrap().iter().all(|piece| piece["type"] == "decided"));
}

#[test]
fn a_result_is_kept_as_it_was_read_for_the_person_alone_and_the_run_prints_what_it_printed() {
    let pieces =
        json!([did("I made n.txt."), {"type": "change", "file": "n.txt", "before": [], "after": ["new from the run"]}]);
    let (made, said) = ended_with(&done(&pieces));
    let (bare, plain) = ended_with(r#"{"state":"done"}"#);
    assert_eq!(said, plain, "a run prints no word of its result");
    let kept = fs::read_to_string(kept_file(&made)).unwrap();
    let want = r#"{"pieces":[{"type":"did","text":"I made n.txt."},{"type":"change","file":"n.txt","after":["new from the run"]}]}"#;
    assert_eq!(kept, format!("{want}\n"));
    // Unix: the file is the person's alone. Windows has no such mode, and a file there is its folder's.
    #[cfg(unix)]
    assert_eq!(
        std::os::unix::fs::PermissionsExt::mode(&fs::metadata(kept_file(&made)).unwrap().permissions()) & 0o777,
        0o600
    );
    // The record of a run with a result is the record of a run without one.
    let parts = |made: &Made| made.record().as_object().unwrap().keys().cloned().collect::<Vec<_>>();
    assert_eq!(parts(&made), parts(&bare));
}

#[test]
fn a_result_beside_an_unfit_last_word_fails_the_run_as_before() {
    let one = did("I made n.txt.").to_string();
    let unfit = "its last word was not `done` alone, or `asks` with one question";
    // A result beside a question is no unfit word since F87: the run asks and the result is dropped (`f87.rs`).
    for word in [
        format!(r#"{{"state":"done","result":{{"pieces":[{one}]}},"more":1}}"#),
        format!(r#"{{"state":"done","state":"done","result":{{"pieces":[{one}]}}}}"#),
        format!(r#"{{"result":{{"pieces":[{one}]}}}}"#),
    ] {
        let (made, said) = ended_with(&word);
        assert_eq!(said["outcome"], "failed", "{word}: {said}");
        assert!(said["why"].as_str().unwrap().contains(unfit), "{said}");
        assert!(!kept_file(&made).exists(), "{word}");
    }
}

#[test]
fn a_file_written_by_hand_is_checked_as_a_run_s_result_is_and_one_that_is_not_a_result_is_not_used() {
    let (made, _) = ended_with(r#"{"state":"done"}"#);
    let piece = json!({"type": "change", "file": "n.txt", "after": ["new from the run"]});
    let one = json!({"pieces": [piece, did("I made n.txt.")]}).to_string();
    let want = json!({"pieces": [did("I made n.txt."),
        {"type": "change", "n": 1, "name": "n.txt", "made": true, "before": [], "after": ["new from the run"]}], "dropped": 0});
    write_kept(&made, &one);
    assert_eq!(result(&made), want);
    // It is never trusted: a file that says what `run show` prints is not the shape, and one that claims a
    // line the work does not hold has that piece dropped.
    for not_it in [
        want.to_string(),
        "not JSON".to_string(),
        String::new(),
        "[]".to_string(),
        format!("{one}{one}"),
        one.replace("did", "checked"),
        one.replace("\"text\"", "\"words\""),
        format!("{one}{}", " ".repeat(8193 - one.len())),
    ] {
        write_kept(&made, &not_it);
        as_before(&made);
    }
    write_kept(&made, &one.replace("new from the run", "new from the person"));
    assert_eq!(result(&made), json!({"pieces": [did("I made n.txt.")], "dropped": 1}));
    // A file of exactly the size is read.
    write_kept(&made, &format!("{one}{}", " ".repeat(8192 - one.len())));
    assert_eq!(result(&made), want);
    // Unix: a file the group or others can write to is not used. Windows has no such mode.
    #[cfg(unix)]
    {
        fs::set_permissions(kept_file(&made), std::os::unix::fs::PermissionsExt::from_mode(0o620)).unwrap();
        as_before(&made);
    }
    // Something else at the name is not opened, and the look is as it was.
    fs::remove_file(kept_file(&made)).unwrap();
    fs::create_dir(kept_file(&made)).unwrap();
    as_before(&made);
    let waiting = printed(&made.w.run(&["run", "waiting", "--date", DAY, "--repo", made.repo.to_str().unwrap()]));
    assert_eq!(waiting["waits"][0].as_object().unwrap().len(), 12, "{waiting}");
}

// A result is kept of a run that finished, and of no other: a run that said it was done and what it did, and
// whose harness then exited with a failure, has failed and keeps none.
#[test]
fn a_run_that_did_not_finish_keeps_nothing_of_what_it_said() {
    let word = done(&json!([did("I made n.txt.")]));
    let (made, said) = told_exit(3, &[], &[("n.txt", &b"new from the run\n"[..])], &[], &word);
    assert_eq!((&said["outcome"], &said["star"]), (&json!("failed"), &Value::Null), "{said}");
    assert!(!kept_file(&made).exists());
}

// Told alone: a run that finished and changed nothing has its result printed by a look, with no file to check a
// change against, so each `change` is dropped and counted, though it quotes a file of the repo as it is. The run
// has no work to wait, so `run waiting` lists none.
#[test]
fn a_run_that_changed_nothing_has_its_result_printed_by_a_look_and_every_change_of_it_dropped() {
    let change = json!({"type": "change", "file": "a.txt", "before": ["a.txt as it was"], "after": ["it is"]});
    let unsure = json!({"type": "unsure", "text": "It could not see the shop's hours."});
    let (made, said) = told(&[], &[], &[], &done(&json!([change, did("The shop opens at nine."), unsure.clone()])));
    assert_eq!((&said["outcome"], &said["star"]), (&json!("completed"), &json!(STAR)), "{said}");
    assert!(said["why"].as_str().unwrap().contains("finished and changed nothing"), "{said}");
    let shown = printed(&show(&made, &[]));
    assert_eq!((&shown["work"], &shown["files"]), (&json!("waits"), &json!([])), "{shown}");
    assert_eq!(shown["result"], json!({"pieces": [did("The shop opens at nine."), unsure], "dropped": 1}));
    let waiting = printed(&made.w.run(&["run", "waiting", "--date", DAY, "--repo", made.repo.to_str().unwrap()]));
    assert_eq!(waiting["waits"], json!([]), "{waiting}");
    // Tossed, the run is no longer said to wait, and a look prints nothing of what it said.
    let asked = ["run", "toss", "--wish", &made.wish, "--date", DAY, "--repo", made.repo.to_str().unwrap()];
    assert_eq!(printed(&made.w.run(&asked))["tossed"], json!(true));
    let shown = printed(&show(&made, &[]));
    assert_eq!((&shown["work"], &shown["result"]), (&json!("tossed"), &Value::Null), "{shown}");
}
