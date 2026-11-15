//! Where an answer's words are: in the harness's input and in the made wish's own file, as the door left them,
//! and nowhere else. Not on a command line, in the run's record, on the day's page, or in anything printed.

use std::path::PathBuf;

use serde_json::json;

use super::{DAY, Made, asking, printed};
use crate::support::{Ran, key_id, map_key, shows, tree};

/// Words no other text of a test holds, so they can be looked for in every file.
const WORDS: &str = "walnut-meridian";

/// The files under the test's folder that hold `words`, by their names from that folder.
fn holding(made: &Made, words: &str) -> Vec<PathBuf> {
    let found = |bytes: &[u8]| bytes.windows(words.len()).any(|window| window == words.as_bytes());
    let root = made.w.dir.path();
    let held = tree(root).into_iter().filter(|(_, bytes)| found(bytes));
    held.map(|(path, _)| path.strip_prefix(root).unwrap().to_path_buf()).collect()
}

/// Every command that reads the wish or its run, and what each printed.
fn looked(made: &Made) -> Vec<Ran> {
    let (wish, repo, logs) = (made.wish.as_str(), made.repo.to_str().unwrap(), made.w.at("logs"));
    let on = ["--wish", wish, "--date", DAY];
    let within = ["--date", DAY, "--repo", repo];
    vec![
        made.w.run(&["page", "--date", DAY, "--logs", logs.to_str().unwrap()]),
        made.w.run(&[&["run", "stop"], &on[..]].concat()),
        made.w.run(&[&["run", "show"], &on[..], &within[2..]].concat()),
        made.w.run(&[&["run", "keep", "--ask"], &on[..], &within[2..]].concat()),
        made.w.run(&[&["run", "waiting"], &within[..]].concat()),
        made.w.run(&[&["run"], &on[..], &within[2..]].concat()),
    ]
}

#[test]
fn an_answer_s_words_are_in_the_harness_s_input_and_the_wish_s_own_file_and_nowhere_else() {
    let key = map_key("kLmNo");
    let typed = format!("use the {WORDS} one,\n with \"{key}\" as its key\r\n");
    let kept = format!("use the {WORDS} one, with '[redacted]' as its key");
    // Three ends: the run finishes, asks again, and fails.
    for script in ["edits", "asks", "exit-3"] {
        let made = asking();
        made.w.play(script, 0);
        let ran = made.answer(&typed);
        let mut all = looked(&made);
        all.push(ran);
        for ran in &all {
            let said = format!("{}{}", ran.stdout, ran.stderr);
            assert!(!said.contains(WORDS) && !shows(&said, &key), "{script}: {said}");
        }
        // The harness was given the answer as the door left it, on its input alone.
        assert_eq!(made.w.rec("stdin").unwrap(), format!("{kept}\n"), "{script}");
        let wishes = PathBuf::from("jm").join("pages").join(format!("{DAY}.wishes.json"));
        assert_eq!(holding(&made, WORDS), [wishes, PathBuf::from("rec").join("stdin")], "{script}");
        assert_eq!(made.w.kept(&made.wish)["answer"], json!(kept), "{script}");
        // The credential's shape is in no file at all: not the home's, not the agent's, not the repo's.
        let secret =
            tree(made.w.dir.path()).into_iter().find(|(_, bytes)| shows(&String::from_utf8_lossy(bytes), &key));
        assert!(secret.is_none(), "{script}: {:?}", secret.map(|held| held.0));
    }
}

// A credential that a line break splits is one once the answer is on one line: the door finds it there. A word
// that touches it across one more break does not hide it (the second gate's texts, with a key id).
#[test]
fn a_credential_split_by_a_line_break_in_an_answer_never_reaches_the_harness() {
    let (key, id) = (map_key("kLmNo"), key_id());
    let ((head, tail), (first, last)) = (key.split_at(17), id.split_at(10));
    let split = ["\n", "\r\n", "\r", "\u{2028}"].map(|split| {
        (format!("the key is {head}{split}{tail} and no other"), "the key is [redacted] and no other", [head, tail])
    });
    let touched = [
        (format!("the id is\n{first}\n{last} for the upload"), "the id [redacted] for the upload", [first, last]),
        (format!("{first}\r\n{last}\r\nis the key id"), "[redacted] the key id", [first, last]),
        (format!("use {first}\u{2028}{last}\u{2028}please"), "use [redacted]", [first, last]),
    ];
    for (typed, kept, parts) in split.into_iter().chain(touched) {
        let made = asking();
        made.w.play("nothing", 0);
        assert_eq!(printed(&made.answer(&typed))["outcome"], "completed", "{typed:?}");
        assert_eq!(made.w.rec("stdin").unwrap(), format!("{kept}\n"), "{typed:?}");
        for part in parts {
            let held =
                tree(made.w.dir.path()).into_iter().find(|(_, bytes)| shows(&String::from_utf8_lossy(bytes), part));
            assert!(held.is_none(), "{typed:?}: {:?}", held.map(|held| held.0));
        }
    }
}
