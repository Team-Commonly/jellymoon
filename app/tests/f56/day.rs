//! The fourth step: with `--date` the day's made wishes are read, by core, and give `working`, `asks` and `waits`
//! with no hearing at all; and when a line of a hearing is given too, core's order says which is drawn. The made
//! wishes and a run's record are typed here as core keeps them, in a home of the test's own; no run is started.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tempfile::TempDir;

use crate::jelly::{
    ASKS, Drawn, NO_LINE, SCREEN, app, asks, cannot, heard, listening, placed, quiet, said, waits, working,
};
use crate::support::worked::tree;
use crate::{QUESTION, SAID};

const DAY: &str = "2026-03-10";
/// The run of core's own example of a record, which finished with one commit that waits.
const FINISHED: &str = "r0123456789abcdef";

/// A Jellymoon home with one day's made wishes.
struct Home(TempDir);

fn own(file: &Path, text: &str) {
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(file, text).unwrap();
    // Core reads a kept file only when its owner alone can write to it, whatever this process's umask is.
    #[cfg(unix)]
    fs::set_permissions(file, std::os::unix::fs::PermissionsExt::from_mode(0o644)).unwrap();
}

impl Home {
    /// The day with these made wishes, in their order, and the record of the one run that finished.
    fn of(wishes: &[Value]) -> Home {
        let home = Home(tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap());
        if !wishes.is_empty() {
            let made = json!({"day": DAY, "wishes": wishes});
            own(&home.at().join("pages").join(format!("{DAY}.wishes.json")), &format!("{made}\n"));
        }
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("crates").join("jellymoon-core");
        let record = fs::read_to_string(fixtures.join("tests").join("fixtures").join("run-record.json")).unwrap();
        own(&home.at().join("runs").join(FINISHED).join("run.json"), &record);
        home
    }

    fn at(&self) -> PathBuf {
        self.0.path().join("jm")
    }

    /// What is drawn for the day, with `input` as the hearing's line: nothing, for no hearing at all.
    fn drawn(&self, input: &str) -> Drawn {
        let before = tree(&self.at());
        let shown = placed(&app(input, SCREEN, &["--date", DAY], &[("JELLYMOON_HOME", &self.at())]));
        assert_eq!(tree(&self.at()), before, "drawing the jellyfish wrote to Jellymoon's home");
        shown
    }

    fn state(&self, input: &str) -> String {
        self.drawn(input).jellyfish().0.to_string()
    }
}

fn made(at: usize, state: &str, more: Value) -> Value {
    let mut wish = json!({"id": format!("m{at}"), "said": SAID, "project": "alpha", "state": state, "stars": []});
    for (name, value) in more.as_object().unwrap() {
        wish[name] = value.clone();
    }
    wish
}

/// A made wish whose run is going.
fn going(at: usize) -> Value {
    made(at, "working", json!({"run": format!("r{}", at.to_string().repeat(16))}))
}

/// A made wish whose run asks.
fn asking(at: usize, question: &str) -> Value {
    made(at, "needs_you", json!({"question": question, "run": format!("r{}", at.to_string().repeat(16))}))
}

/// A made wish whose run finished: with [`FINISHED`], its work waits; any other run has no record here.
fn done(at: usize, run: &str) -> Value {
    made(at, "done", json!({"run": run, "stars": ["finished"]}))
}

#[test]
fn the_day_s_made_wishes_give_working_asks_and_waits_with_no_hearing_at_all() {
    let plain = made(1, "working", json!({}));
    assert_eq!(Home::of(&[]).state(""), "quiet");
    assert_eq!(Home::of(&[plain]).state(""), "quiet", "a made wish with no run is nothing to show");
    // One run going: the jellyfish swims. Two: it carries their count.
    let one = Home::of(&[going(1)]).drawn("");
    assert_eq!((one.jellyfish(), one.kinds().len()), (("working", "swims"), 2));
    let two = Home::of(&[going(1), going(2)]).drawn("\n");
    assert_eq!((two.jellyfish().0, two.of("count")[0].said()), ("working", "2".to_string()));
    // Work that waits: it glows, with no bubble. Work with no record that says so does not wait.
    let waiting = Home::of(&[done(1, FINISHED)]).drawn("");
    assert_eq!((waiting.jellyfish(), waiting.kinds().len()), (("waits", "glows"), 2));
    assert_eq!(Home::of(&[done(1, "r9999999999999999")]).state(""), "quiet");
    // A run that asks: its question, as the run's own words, under Jellymoon's.
    let asked = Home::of(&[asking(1, QUESTION)]).drawn("");
    assert_eq!(asked.jellyfish(), ("asks", "waves"));
    assert_eq!((asked.text("body"), asked.text("quoted")), (ASKS.to_string(), QUESTION.to_string()));
    // Made wishes that cannot be used give nothing to show, and are not an error.
    let broken = Home::of(&[]);
    own(&broken.at().join("pages").join(format!("{DAY}.wishes.json")), "not the made wishes\n");
    assert_eq!(broken.state(""), "quiet");
}

#[test]
fn when_more_than_one_state_holds_core_s_order_decides() {
    // Of what a day gives: a run that asks, then runs going, then work that waits.
    let all = Home::of(&[done(1, FINISHED), going(2), asking(3, QUESTION), going(4)]);
    assert_eq!(all.state(""), "asks");
    let two = Home::of(&[done(1, FINISHED), going(2)]);
    assert_eq!(two.state(""), "working");
    // What a hearing alone knows comes before all of it: the key held, a heard wish, a refusal.
    for (line, state) in
        [(listening("make"), "listening"), (heard(SAID, "alpha", 3000), "heard"), (cannot("no"), "cannot")]
    {
        let shown = all.drawn(&line.to_string());
        assert_eq!(shown.jellyfish().0, state);
        assert_eq!(shown.text("quoted"), "", "{state}: the run's question waits its turn");
    }
    // What a hearing's line says of the day's runs is as old as its last look: the day, read now, is drawn.
    for stale in [quiet(), working(3), waits(), asks("an earlier question")] {
        let shown = all.drawn(&stale.to_string());
        assert_eq!((shown.jellyfish().0, shown.text("quoted")), ("asks", QUESTION.to_string()), "{stale}");
        assert_eq!(two.state(&stale.to_string()), "working", "{stale}");
        assert_eq!(Home::of(&[]).state(&stale.to_string()), "quiet", "{stale}");
    }
    // The first question of the day is the one drawn.
    let questions = Home::of(&[asking(1, "The first question?"), asking(2, "The second question?")]);
    assert_eq!(questions.drawn("").text("quoted"), "The first question?");
}

#[test]
fn a_day_is_read_only_when_it_is_named_and_a_line_is_still_asked_for_without_one() {
    let asks_today = Home::of(&[asking(1, QUESTION)]);
    let home = asks_today.at();
    let vars: [(&str, &Path); 1] = [("JELLYMOON_HOME", &home)];
    // With no `--date` the line is all that is drawn, and nothing at all is no line.
    assert_eq!(placed(&app(&quiet().to_string(), SCREEN, &[], &vars)).jellyfish().0, "quiet");
    assert_eq!(said(&app("", SCREEN, &[], &vars)), (Some(1), String::new(), NO_LINE.to_string()));
    // With one, a line that is no line is still refused, and the day is not drawn in its place.
    assert_eq!(said(&app("{}", SCREEN, &["--date", DAY], &vars)), (Some(1), String::new(), NO_LINE.to_string()));
    // A day that is none, and a day with no home to read it in, are a wrong command line.
    for (date, vars, why) in
        [("2026-3-10", &vars[..], "--date must be a day as YYYY-MM-DD"), (DAY, &vars[..0], "JELLYMOON_HOME")]
    {
        let (code, out, err) = said(&app("", SCREEN, &["--date", date], vars));
        assert_eq!((code, out.as_str()), (Some(2), ""), "{err}");
        assert!(err.starts_with("jellymoon-app: ") && err.contains(why) && err.contains("usage:"), "{err}");
    }
    let relative: [(&str, &Path); 1] = [("JELLYMOON_HOME", Path::new("jm"))];
    assert_eq!(said(&app("", SCREEN, &["--date", DAY], &relative)).0, Some(2));
}
