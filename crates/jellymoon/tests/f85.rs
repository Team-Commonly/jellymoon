//! F85: what a finished run says of its own work is kept, checked against the work at every look and printed
//! beside it (ADR 0023; `docs/development/show.md`, "What the run says of its work"). `jellymoon run`, `run show`
//! and `run waiting` are run as black boxes in F39's test folder (`f39/world.rs`). The made-up `fake-agent` leaves
//! the files a test typed and ends with the last word a test typed (its script `told`), so every result here is
//! one a model could write, and none is a model's. `f85/checks.rs` holds each rule of a check, asked of a result
//! written into the run's folder by hand, which is checked as the run's own is; `f85/kept.rs` holds what is kept
//! of a last word, and what is no result.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "f85/checks.rs"]
mod checks;
#[path = "f85/kept.rs"]
mod kept;
mod support;
#[path = "f39/world.rs"]
mod world;

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use support::{Ran, tree};
pub use world::*;

/// One file of a test's typing: its path, with `/` between folders, and what it holds.
pub type Typed<'a> = (&'a str, &'a [u8]);

fn put(folder: &Path, files: &[Typed]) {
    for (path, holds) in files {
        let file = path.split('/').fold(folder.to_path_buf(), |at, part| at.join(part));
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, holds).unwrap();
    }
}

/// A repo that has `had` on `main` beside its three files, and one run in it: the made-up agent leaves `left`,
/// takes `gone` away and ends with `word` as its last word. With it, what the run printed.
pub fn told(had: &[Typed], left: &[Typed], gone: &[&str], word: &str) -> (Made, Value) {
    told_exit(0, had, left, gone, word)
}

/// The same, with the made-up agent exiting with `exit` after its last word.
pub fn told_exit(exit: usize, had: &[Typed], left: &[Typed], gone: &[&str], word: &str) -> (Made, Value) {
    let w = world();
    let repo = w.repo("alpha");
    if !had.is_empty() {
        put(&repo, had);
        w.git(&repo, &["add", "."]);
        w.git(&repo, &["commit", "-q", "-m", "the files a test works on"]);
    }
    let rec = w.at("rec");
    fs::create_dir_all(rec.join("work")).unwrap();
    put(&rec.join("work"), left);
    fs::write(rec.join("gone"), gone.join("\n")).unwrap();
    fs::write(rec.join("word"), word).unwrap();
    w.play("told", exit);
    let wish = w.wish("alpha");
    let (run, said) = ran(&w.start(&wish, &repo));
    (Made { w, repo, wish, run }, said)
}

/// The last word of a run that is done and says `pieces` of its work.
pub fn done(pieces: &Value) -> String {
    json!({"state": "done", "result": {"pieces": pieces}}).to_string()
}

/// `jellymoon run show` for the run's wish, in its repo.
pub fn show(made: &Made, more: &[&str]) -> Ran {
    let asked = ["run", "show", "--wish", &made.wish, "--date", DAY, "--repo", made.repo.to_str().unwrap()];
    made.w.run(&[&asked[..], more].concat())
}

/// What `run show` prints as the run's `result`: null when it prints none.
pub fn result(made: &Made) -> Value {
    printed(&show(made, &[]))["result"].clone()
}

/// The one wish `run waiting` lists.
pub fn waits(made: &Made) -> Value {
    let ran = made.w.run(&["run", "waiting", "--date", DAY, "--repo", made.repo.to_str().unwrap()]);
    let said = printed(&ran);
    let [one] = &said["waits"].as_array().unwrap()[..] else { panic!("{said}") };
    one.clone()
}

/// Where a run's result is kept.
pub fn kept_file(made: &Made) -> PathBuf {
    made.w.jm("runs").join(&made.run).join("result.json")
}

/// Writes a result into the run's folder by hand, as anyone who can write Jellymoon's home can.
pub fn write_kept(made: &Made, text: &str) {
    let file = kept_file(made);
    fs::write(&file, text).unwrap();
    // Core takes no file its group or others can write to, which a new file is under a umask of 002.
    #[cfg(unix)]
    fs::set_permissions(&file, std::os::unix::fs::PermissionsExt::from_mode(0o600)).unwrap();
}

/// The two files of the first worked example, as they were and as the run left them.
const SHIPPING: [&str; 2] = [
    "pub fn shipping(total: u32) -> u32 {\n    if total >= 100 { 0 } else { 8 }\n}\n",
    "pub fn shipping(total: u32) -> u32 {\n    if total >= 50 { 0 } else { 8 }\n}\n",
];
const TESTED: [&str; 2] = [
    "#[test]\nfn free_from_100() {\n    assert_eq!(shipping(100), 0);\n}\n",
    "#[test]\nfn free_from_50() {\n    assert_eq!(shipping(49), 8);\n    assert_eq!(shipping(50), 0);\n}\n",
];
/// The last word of the first worked example, as ADR 0023 types it.
const FIRST: &str = r#"{"state":"done","result":{"pieces":[{"type":"did","text":"Shipping is now free from 50, not 100. The test tries 49 and 50."},
 {"type":"change","file":"src/shipping.rs","before":["    if total >= 100 { 0 } else { 8 }"],"after":["    if total >= 50 { 0 } else { 8 }"]},
 {"type":"decided","text":"An order of exactly 50 ships free."}, {"type":"unsure","text":"I could not run the test."},
 {"type":"change","file":"README.md","before":[],"after":["Free shipping from 50."]}]}}"#;

/// The run of the first worked example: one function and its test.
fn shipping() -> (Made, Value) {
    let had = [("src/shipping.rs", SHIPPING[0].as_bytes()), ("tests/shipping.rs", TESTED[0].as_bytes())];
    let left = [("src/shipping.rs", SHIPPING[1].as_bytes()), ("tests/shipping.rs", TESTED[1].as_bytes())];
    told(&had, &left, &[], FIRST)
}

/// The `result` of the first worked example, as it is printed.
fn first_result(dropped: usize, change: bool) -> String {
    let quoted = r#"{"type":"change","n":1,"name":"src/shipping.rs","made":false,"before":["if total >= 100 { 0 } else { 8 }"],"after":["if total >= 50 { 0 } else { 8 }"]},"#;
    format!(
        r#"{{"pieces":[{{"type":"did","text":"Shipping is now free from 50, not 100. The test tries 49 and 50."}},{}{{"type":"decided","text":"An order of exactly 50 ships free."}},{{"type":"unsure","text":"I could not run the test."}}],"dropped":{dropped}}}"#,
        if change { quoted } else { "" }
    )
}

/// The same as a value: a value's own writing puts its parts in another order than core prints them in.
fn first(dropped: usize, change: bool) -> Value {
    serde_json::from_str(&first_result(dropped, change)).unwrap()
}

// The first worked example of ADR 0023, word for word: the pieces in the catalogue's order, the change under the
// file's number and core's own name for it with the quoted lines' ends left off, and the piece about a file the
// work does not hold dropped and counted. The list of files is what it was before there was a result.
#[test]
fn a_run_that_ends_with_a_result_has_it_printed_beside_its_files_by_a_look_and_by_run_waiting() {
    let (made, said) = shipping();
    assert_eq!((&said["outcome"], &said["star"]), (&json!("completed"), &json!(STAR)), "{said}");
    let run = &made.run;
    let files = format!(
        r#"[{{"n":1,"name":"src/shipping.rs","change":"changed","size":{},"kind":"text","obeyed":false}},{{"n":2,"name":"tests/shipping.rs","change":"changed","size":{},"kind":"text","obeyed":false}}]"#,
        SHIPPING[1].len(),
        TESTED[1].len()
    );
    let want = format!(
        r#"{{"run":"{run}","wish":"m1","work":"waits","files":{files},"more":0,"most":200,"result":{}}}"#,
        first_result(1, true)
    );
    let shown = show(&made, &[]);
    assert_eq!((shown.stdout.as_str(), shown.stderr.as_str(), shown.code), (format!("{want}\n").as_str(), "", 0));
    // `run waiting` says the same of it, beside what it said of the work before.
    let waits = waits(&made);
    assert_eq!(waits["result"], printed(&shown)["result"]);
    assert_eq!((&waits["names"], &waits["changed"]), (&json!(["src/shipping.rs", "tests/shipping.rs"]), &json!(2)));
    assert_eq!(waits.as_object().unwrap().len(), 13, "{waits}");
    // One file's text is printed as it was before F85, without its last line break: the result is no part of it.
    let file = printed(&show(&made, &["--file", "1"]));
    assert_eq!((&file["text"], &file["result"]), (&json!(SHIPPING[1].trim_end()), &Value::Null));
    made.w.never_says_the_agent(&shown);
}

// The second worked example: a file the run made takes no `before`, and is said to be made.
#[test]
fn a_change_to_a_file_the_run_made_is_printed_as_made_with_the_lines_it_holds() {
    let page = "<!doctype html>\n<h1>We bake on Saturdays</h1>\n<p>Come by from 8 in the morning.</p>\n";
    let word = r#"{"state":"done","result":{"pieces":[{"type":"did","text":"There is a new page, index.html, with a heading and one line about Saturdays."},
 {"type":"change","file":"index.html","before":[],"after":["<h1>We bake on Saturdays</h1>","<p>Come by from 8 in the morning.</p>"]},
 {"type":"decided","text":"I made up the opening time, 8 in the morning."}, {"type":"decided","text":"The page has no colours or pictures yet."}]}}"#;
    let (made, _) = told(&[], &[("index.html", page.as_bytes())], &[], word);
    let want = format!(
        r#"{{"run":"{}","wish":"m1","work":"waits","files":[{{"n":1,"name":"index.html","change":"made","size":{},"kind":"text","obeyed":false}}],"more":0,"most":200,"result":{{"pieces":[{{"type":"did","text":"There is a new page, index.html, with a heading and one line about Saturdays."}},{{"type":"change","n":1,"name":"index.html","made":true,"before":[],"after":["<h1>We bake on Saturdays</h1>","<p>Come by from 8 in the morning.</p>"]}},{{"type":"decided","text":"I made up the opening time, 8 in the morning."}},{{"type":"decided","text":"The page has no colours or pictures yet."}}],"dropped":0}}}}"#,
        made.run,
        page.len()
    );
    assert_eq!(show(&made, &[]).stdout, format!("{want}\n"));
    // The lines are the made file's, and are printed under no other name: a made file has no side to be read.
    for file in ["other.html", "a.txt", "index.htm", "Index.html"] {
        let piece = json!({"type": "change", "file": file, "after": ["<h1>We bake on Saturdays</h1>"]});
        write_kept(&made, &json!({"pieces": [piece]}).to_string());
        assert_eq!(result(&made), json!({"pieces": [], "dropped": 1}), "{file}");
    }
}

// A change is held to the file it names, and to no other: lines the work put in its first file are not printed
// for a name that is no file of the work, whether nothing has that name or a file the work left alone has it.
#[test]
fn a_change_that_names_no_file_of_the_work_is_dropped_though_its_lines_are_in_another_file_of_the_work() {
    let (made, _) = shipping();
    let line = "if total >= 50 { 0 } else { 8 }";
    for file in ["src/other.rs", "a.txt", "shipping.rs", "src"] {
        let piece = json!({"type": "change", "file": file, "after": [line]});
        write_kept(&made, &json!({"pieces": [piece]}).to_string());
        assert_eq!(result(&made), json!({"pieces": [], "dropped": 1}), "{file}");
    }
    let piece = json!({"type": "change", "file": "src/shipping.rs", "after": [line]});
    write_kept(&made, &json!({"pieces": [piece]}).to_string());
    assert_eq!(result(&made)["pieces"][0]["n"], 1);
}

/// Puts other work where the run's own was, as someone who can write the repo can: a commit over `main` with the
/// message core gives a run's, which the run's branch and its record are made to name.
fn rework(made: &Made, files: &[Typed]) {
    let git = |args: &[&str]| made.w.git(&made.repo, args);
    git(&["checkout", "-q", "-B", "made-up", "main"]);
    put(&made.repo, files);
    git(&["add", "."]);
    git(&["commit", "-q", "-m", &format!("Jellymoon run {}", made.run)]);
    git(&["checkout", "-q", "main"]);
    let end = made.w.commit(&made.repo, "made-up");
    git(&["update-ref", &made.branch(), &end]);
    let mut record = made.record();
    record["end"] = json!(end);
    made.write(&record);
}

// The check is of the work as it is at the look, not as it was when the result was kept: the same kept result,
// over work that no longer holds the quoted line, loses that piece. And the look writes nothing.
#[test]
fn a_result_is_checked_against_the_work_at_every_look_and_never_when_it_is_kept() {
    let (made, _) = shipping();
    let kept = fs::read(kept_file(&made)).unwrap();
    assert_eq!(result(&made), first(1, true));
    let other = SHIPPING[1].replace("50", "60");
    rework(&made, &[("src/shipping.rs", other.as_bytes()), ("tests/shipping.rs", TESTED[1].as_bytes())]);
    let before = tree(&made.w.jm("runs").join(&made.run));
    assert_eq!(result(&made), first(2, false), "the line is in the work no more");
    assert_eq!(waits(&made)["result"], first(2, false));
    assert_eq!((tree(&made.w.jm("runs").join(&made.run)), fs::read(kept_file(&made)).unwrap()), (before, kept));
    // Work that holds the line again has the piece again.
    rework(&made, &[("src/shipping.rs", SHIPPING[1].as_bytes())]);
    assert_eq!(result(&made), first(1, true));
}

// What a run says is kept in one file of its own folder and is in no other file of Jellymoon's home, whatever
// was asked meanwhile: no log, no record, no page and no made wish holds a word of it.
#[test]
fn a_result_is_kept_in_the_runs_own_file_and_in_no_other_file_of_the_home() {
    let (made, _) = shipping();
    let (_, _, _) = (show(&made, &[]), waits(&made), made.w.on_page(&made.wish));
    let words = ["Shipping is now free", "total >= 50", "ships free", "could not run the test", "Free shipping from"];
    let mut held = Vec::new();
    for (path, bytes) in tree(&made.w.at("jm")) {
        let text = String::from_utf8_lossy(&bytes);
        if words.iter().any(|word| text.contains(word)) {
            held.push(path);
        }
    }
    assert_eq!(held, [kept_file(&made)]);
    assert!(!made.w.jm("app.log").exists());
    // The work that waits is kept and tossed as before, and a look at work that waits no more prints no result.
    let asked = ["--wish", &made.wish, "--date", DAY, "--repo", made.repo.to_str().unwrap()];
    let kept = printed(&made.w.run(&[&["run", "keep"][..], &asked].concat()));
    assert!(kept["kept"].is_string(), "{kept}");
    let shown = printed(&show(&made, &[]));
    assert_eq!((&shown["work"], &shown["result"], shown.as_object().unwrap().len()), (&json!("kept"), &Value::Null, 6));
}

// git is told an object's id and nothing else of the work: the made-up git, first on `PATH`, leaves a file when
// one of its arguments is the word it was given. It leaves none for the file's name or for a quoted line, and
// leaves one for the id of the file as it was, which is the one new question.
#[test]
fn no_name_and_no_line_of_the_work_is_ever_an_argument_to_git_and_the_file_as_it_was_is_asked_for_by_its_id() {
    let (made, _) = shipping();
    let (vars, fake) = (made.w.fake_git(), made.w.at("fake"));
    fs::write(fake.join("returns"), "").unwrap();
    let asked = ["run", "show", "--wish", &made.wish, "--date", DAY, "--repo", made.repo.to_str().unwrap()];
    let seen = |word: &str| {
        let _ = fs::remove_file(fake.join("held"));
        fs::write(fake.join("on"), word).unwrap();
        let shown = printed(&made.w.run_with(&vars, &asked));
        assert_eq!(shown["result"], first(1, true), "{word}");
        fake.join("held").exists()
    };
    let was = made.w.git(&made.repo, &["rev-parse", "main:src/shipping.rs"]);
    let is = made.w.git(&made.repo, &["rev-parse", &format!("{}:src/shipping.rs", made.branch())]);
    assert!(seen(&was), "the file as it was is asked for by the id git printed for it");
    assert!(seen("blob"), "with `cat-file blob`");
    assert!(!seen(&is), "the file as it is was read with the others, by an id on git's input");
    for word in
        ["src/shipping.rs", "README.md", "    if total >= 100 { 0 } else { 8 }", "if total >= 50 { 0 } else { 8 }"]
    {
        assert!(!seen(word), "{word:?} was an argument to git");
    }
    // One file is printed without the result, so nothing is asked of the file as it was.
    fs::write(fake.join("on"), &was).unwrap();
    let file = printed(&made.w.run_with(&vars, &[&asked[..], &["--file", "1"]].concat()));
    assert!(file["text"].is_string() && !fake.join("held").exists(), "{file}");
}
