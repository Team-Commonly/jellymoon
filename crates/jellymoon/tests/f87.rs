//! F87: a run is asked to end with its result (ADR 0023, decision 1; `docs/development/run.md`, "What the harness is
//! started with"). `jellymoon run`, `run answer` and `run show` are run as black boxes in F39's test folder
//! (`f39/world.rs`), through the door. The made-up `fake-agent` records the command line it was given; its script
//! `under` answers under the schema it was given, as a harness bound to it does: with the result a test typed when
//! the schema has room for one, and `done` alone when it has none. Every last word here is typed by hand.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;
#[path = "f39/world.rs"]
mod world;

use std::fs;

use serde_json::{Value, json};
use support::started;
pub use world::*;

/// The four kinds of a result's pieces, in the catalogue's order.
const KINDS: [&str; 4] = ["did", "change", "decided", "unsure"];
/// The page the made-up agent leaves, and the last word it ends with: the second worked example of ADR 0023.
const PAGE: &str = "<h1>We bake on Saturdays</h1>\n<p>Come by from 8 in the morning.</p>\n";
const WORD: &str = r#"{"state":"done","result":{"pieces":[{"type":"did","text":"There is a new page, index.html, with a heading and one line about Saturdays."},
 {"type":"change","file":"index.html","before":[],"after":["<h1>We bake on Saturdays</h1>","<p>Come by from 8 in the morning.</p>"]},
 {"type":"decided","text":"I made up the opening time, 8 in the morning."}, {"type":"decided","text":"The page has no colours or pictures yet."}]}}"#;

/// The `result` `run show` prints of that run: the change under the file's number, as a made file.
fn printed_result() -> Value {
    json!({"pieces": [
        {"type": "did", "text": "There is a new page, index.html, with a heading and one line about Saturdays."},
        {"type": "change", "n": 1, "name": "index.html", "made": true, "before": [],
         "after": ["<h1>We bake on Saturdays</h1>", "<p>Come by from 8 in the morning.</p>"]},
        {"type": "decided", "text": "I made up the opening time, 8 in the morning."},
        {"type": "decided", "text": "The page has no colours or pictures yet."},
    ], "dropped": 0})
}

/// Has the made-up agent play `script`, leaving `page` as `index.html` and ending with `word`.
fn plays(w: &World, script: &str, page: &str, word: &str) {
    fs::create_dir_all(w.at("rec").join("work")).unwrap();
    fs::write(w.at("rec").join("work").join("index.html"), page).unwrap();
    fs::write(w.at("rec").join("word"), word).unwrap();
    w.play(script, 0);
}

/// The command line the made-up agent was last started with: its schema, and its fixed words.
fn line(w: &World) -> (Value, String) {
    let args: Vec<String> = (0..).map_while(|n| w.rec(&format!("arg{n}"))).collect();
    let at = args.iter().position(|arg| arg == "--json-schema").expect("no schema was given") + 1;
    (serde_json::from_str(&args[at]).unwrap(), args.last().unwrap().clone())
}

/// `run show` for a wish of the day, in the repo.
fn show(w: &World, wish: &str, repo: &std::path::Path) -> Value {
    printed(&w.run(&["run", "show", "--wish", wish, "--date", DAY, "--repo", repo.to_str().unwrap()]))
}

#[test]
fn a_run_is_given_a_schema_with_a_result_of_four_kinds_and_words_that_ask_for_it_and_nothing_of_the_wish() {
    let w = world();
    plays(&w, "under", PAGE, WORD);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (_, said) = ran(&w.start(&wish, &repo));
    assert_eq!(said["outcome"], "completed", "{said}");
    assert_eq!(w.args().0, ARGS, "the whole command line, in order");
    let (schema, words) = line(&w);
    // Every object of the last word is closed: a part the schema does not name cannot stand beside the others.
    let result = &schema["properties"]["result"];
    let piece = &result["properties"]["pieces"]["items"];
    for closed in [&schema, result, piece] {
        assert_eq!(closed["additionalProperties"], false, "{closed}");
    }
    assert_eq!((&result["required"], &piece["required"]), (&json!(["pieces"]), &json!(["type"])));
    assert_eq!(piece["properties"]["type"]["enum"], json!(KINDS));
    assert_eq!(schema["required"], json!(["state"]), "a result is never required: `done` alone is a last word");
    // The fixed words ask for the result and each of its kinds, and say what a quote must be.
    let parts = ["has only `text`", "only `file`, `before` and `after`", "no leading `./`", "under 160 characters"];
    let kinds = ["`result`", "`did`", "`change`", "`decided`", "`unsure`", "character for character"];
    for asked in kinds.iter().chain(&parts) {
        assert!(words.contains(asked), "the fixed words do not ask for {asked}");
    }
    // Nothing of the wish or the repo is in either.
    let repo_named = [repo.to_str().unwrap(), repo.file_name().unwrap().to_str().unwrap()];
    for text in [schema.to_string(), words] {
        for theirs in [SAID, "uploader", "retry"].iter().chain(&repo_named) {
            assert!(!text.contains(theirs), "{theirs:?} is in what the harness was given");
        }
    }
    // The harness answered under that schema, and `run show` prints the result, checked.
    assert_eq!(show(&w, &wish, &repo)["result"], printed_result());
}

#[test]
fn a_run_that_ends_with_done_alone_finishes_as_before_with_no_result() {
    let w = world();
    plays(&w, "told", PAGE, r#"{"state":"done"}"#);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (run, said) = ran(&w.start(&wish, &repo));
    let why = format!("run {run} finished: its work is one commit on the branch jellymoon/run-{run}");
    assert_eq!(said, json!({"wish": wish, "outcome": "completed", "star": STAR, "denied": 0, "why": why}));
    let shown = show(&w, &wish, &repo);
    assert_eq!((shown.get("result"), &shown["files"][0]["name"]), (None, &json!("index.html")), "{shown}");
}

#[test]
fn a_run_that_asks_its_answer_and_a_follow_up_are_given_one_schema_and_the_answered_run_has_its_result() {
    let w = world();
    w.play("asks", 0);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let asked = printed(&w.start(&wish, &repo));
    assert_eq!(asked["outcome"], "asks", "{asked}");
    let (schema, words) = line(&w);
    assert!(schema["properties"]["result"].is_object());
    // The answer goes on with the session, under the same schema, and ends `done` with its result.
    plays(&w, "under", PAGE, WORD);
    let vars: Vec<_> = w.vars.iter().map(|(name, value)| (*name, value.as_os_str())).collect();
    let answer = ["run", "answer", "--wish", &wish, "--date", DAY];
    let answered = printed(&started(w.dir.path(), &vars, &answer, "the bakery's page\n"));
    assert_eq!(answered["outcome"], "completed", "{answered}");
    assert!(w.rec("arg13").is_some_and(|arg| arg == "--resume"), "the answer went on with the session");
    assert_eq!(line(&w), (schema.clone(), words.clone()), "the answer's start");
    assert_eq!(show(&w, &wish, &repo)["result"], printed_result());
    // A follow-up of that work forks its session, under the same schema.
    let more = printed(&w.run(&["wish", "--date", DAY, "--say", "and say we close at noon", "--after", &wish]));
    let follow = more["wish"]["id"].as_str().unwrap().to_string();
    plays(&w, "under", &format!("{PAGE}<p>We close at noon.</p>\n"), r#"{"state":"done"}"#);
    let (_, followed) = ran(&w.start(&follow, &repo));
    assert_eq!(followed["outcome"], "completed", "{followed}");
    assert!(w.rec("arg15").is_some_and(|arg| arg == "--fork-session"), "the follow-up forked the session");
    assert_eq!(line(&w), (schema, words), "the follow-up's start");
}

// A plain question typed as a wish: the model answered it, and copied the person's question into its `done`.
#[test]
fn a_run_done_with_a_question_beside_it_is_done_with_its_result_and_keeps_no_question() {
    let w = world();
    let question = "What time does the shop open?";
    let word = WORD.replacen(r#"{"state":"done","#, &format!(r#"{{"state":"done","question":"{question}","#), 1);
    plays(&w, "told", PAGE, &word);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let went = w.start(&wish, &repo);
    let (run, said) = ran(&went);
    assert_eq!((&said["outcome"], &said["star"]), (&json!("completed"), &json!(STAR)), "{said}");
    assert_eq!(show(&w, &wish, &repo)["result"], printed_result());
    let none = (w.record(&run).get("question").cloned(), w.kept(&wish).get("question").cloned());
    assert_eq!((none, w.on_page(&wish).get("question").cloned()), ((None, None), None));
    // No word of the question is printed or kept.
    let logs = w.at("logs");
    let page = w.run(&["page", "--date", DAY, "--logs", logs.to_str().unwrap()]);
    let shown = w.run(&["run", "show", "--wish", &wish, "--date", DAY, "--repo", repo.to_str().unwrap()]);
    for out in [&went, &page, &shown] {
        assert!(!format!("{}{}", out.stdout, out.stderr).contains("the shop open"), "{}", out.stdout);
    }
    let held = support::tree(&w.at("jm")).into_iter().find(|kept| String::from_utf8_lossy(&kept.1).contains("shop"));
    assert!(held.is_none(), "{:?}", held.map(|held| held.0));
}

#[test]
fn a_harness_too_old_for_the_schema_is_refused_in_the_words_of_today() {
    let w = world();
    w.play("old-option", 0);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let why = "nothing was started: this Claude Code is too old for runs, as it does not know --json-schema; update \
               Claude Code and run the wish again";
    let want = json!({"run": null, "wish": wish, "outcome": "not_started", "star": null, "denied": 0, "why": why});
    assert_eq!(printed(&w.start(&wish, &repo)), want);
}

#[test]
fn a_run_that_asks_with_a_result_beside_its_question_asks_and_keeps_no_result() {
    let w = world();
    let word = r#"{"state":"asks","question":"Which colour should the heading be?","result":{"pieces":[
     {"type":"did","text":"There is a new page."}]}}"#;
    plays(&w, "told", PAGE, word);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let asked = printed(&w.start(&wish, &repo));
    let question = json!("Which colour should the heading be?");
    assert_eq!((&asked["outcome"], &asked["question"]), (&json!("asks"), &question), "{asked}");
    let run = asked["run"].as_str().unwrap();
    assert!(!w.jm("runs").join(run).join("result.json").exists(), "a run that asks keeps no result");
}
