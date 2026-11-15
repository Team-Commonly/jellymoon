//! The order followed work and its follow-up are kept and tossed in (ADR 0018, decision 9): a follow-up is not
//! kept while the work it follows waits, and work is not tossed while a follow-up's run stands on it. Each
//! refusal names the wish to keep or to toss first, and changes nothing.

use std::fs;

use serde_json::{Value, json};

use super::{AS_THE_RUN_LEFT_IT, Made, ended, finished, followed, given, printed, wait_for};

const KEEP_FIRST: &str = "the work of run {own} follows the work of wish {after}, which still waits: keep wish \
                          {after} first, and nothing was changed";
const TOSS_FIRST: &str = "wish {wish} follows the work of run {run}, and its run {own} is not tossed: toss wish \
                          {wish} first, and nothing was changed";

fn keep_first(own: &str, after: &str) -> String {
    KEEP_FIRST.replace("{own}", own).replace("{after}", after)
}

fn toss_first(wish: &str, own: &str) -> String {
    TOSS_FIRST.replace("{wish}", wish).replace("{own}", own)
}

fn run_of(said: &Value) -> String {
    said["run"].as_str().unwrap().to_string()
}

impl Made {
    fn read(&self, file: &str) -> String {
        fs::read_to_string(self.repo.join(file)).unwrap_or_else(|_| "none".to_string())
    }
}

#[test]
fn a_follow_up_is_not_kept_while_the_work_it_follows_waits_and_is_kept_after_it() {
    let made = finished();
    let (wish, said) = made.follows("follow", 0);
    let (own, why, before) = (run_of(&said), keep_first(&run_of(&said), "m1"), made.all());
    made.unchanged(&made.asked(&["keep"], &wish, &[]), &before, &why);
    made.unchanged(&made.asked(&["keep"], &wish, &["--with-settings"]), &before, &why);
    // Asked, a keep says the same and that it would not be done.
    let asked = printed(&made.asked(&["keep"], &wish, &["--ask"]));
    let unfit = json!({"run": own, "wish": wish, "kept": null, "onto": null, "why": why, "fits": false});
    assert!(asked == unfit && before == made.all(), "{asked}");
    // The followed work first: the branch is moved forward to it, and then to the follow-up's commit.
    let first = printed(&made.keep(&[]));
    assert_eq!((&first["kept"], &first["onto"]), (&json!(made.end()), &json!("main")));
    assert_eq!((made.read("a.txt").as_str(), made.read("m.txt").as_str()), ("edited by the run\n", "none"));
    let end = made.w.record(&own)["end"].clone();
    let second = printed(&made.asked(&["keep"], &wish, &[]));
    let forward = format!("run {own} was kept: the branch was moved forward to the run's commit");
    assert_eq!((&second["kept"], &second["onto"], &second["why"]), (&end, &json!("main"), &json!(forward)));
    assert_eq!(made.read("a.txt"), "edited by the run, and again by the follow-up\n");
    assert_eq!((made.read("m.txt").as_str(), made.read("b.txt").as_str()), ("more from the follow-up\n", "none"));
    assert_eq!((made.w.branches(&made.repo), json!(made.w.commit(&made.repo, "main"))), (vec![], end));
}

#[test]
fn a_follow_up_is_kept_after_followed_work_that_was_kept_as_a_new_commit_on_the_person_s_own() {
    let made = finished();
    let (wish, said) = made.follows("follow", 0);
    let own = run_of(&said);
    // The person has committed since the first run started: its work is kept as one new commit, which is not
    // the commit the follow-up's copy was made at.
    let theirs = made.moved_on();
    let first = printed(&made.keep(&[]));
    let joined = made.w.commit(&made.repo, "main");
    assert!(first["why"].as_str().unwrap().contains("one new commit") && first["kept"] == json!(joined), "{first}");
    assert_eq!(made.git(&["rev-parse", "main^"]), theirs);
    assert_ne!(made.w.record(&own)["start"], json!(joined));
    // The follow-up's work is its change from the work it follows, landed on the branch that holds that work.
    let asked = printed(&made.asked(&["keep"], &wish, &["--ask"]));
    assert_eq!((&asked["fits"], &asked["onto"]), (&json!(true), &json!("main")), "{asked}");
    let second = printed(&made.asked(&["keep"], &wish, &[]));
    let why = format!("run {own} was kept: one new commit on top of the branch's own holds its work and the run's");
    assert_eq!((&second["why"], &second["onto"]), (&json!(why), &json!("main")), "{second}");
    assert_eq!(
        (made.git(&["rev-parse", "main^"]), &second["kept"]),
        (joined, &json!(made.w.commit(&made.repo, "main")))
    );
    assert_eq!(made.read("a.txt"), "edited by the run, and again by the follow-up\n");
    assert_eq!(
        (made.read("m.txt").as_str(), made.read("n.txt").as_str()),
        ("more from the follow-up\n", "new from the run\n")
    );
    assert_eq!(
        (made.read("c.txt").as_str(), made.read("mine.txt").as_str()),
        ("c.txt as the person left it\n", "the person's own\n")
    );
    assert_eq!(
        (made.read("b.txt").as_str(), made.w.branches(&made.repo), made.status().as_str()),
        ("none", vec![], "")
    );
    // What the follow-up's commit changes on the branch is the follow-up's own two files.
    assert_eq!(made.git(&["diff", "--name-only", "main^", "main"]), "a.txt\nm.txt");
}

#[test]
fn work_is_not_tossed_while_a_follow_up_s_run_waits_or_asks_on_it() {
    for (script, number, outcome) in [("follow", 0, "completed"), ("asks", 0, "asks")] {
        let made = finished();
        let (wish, said) = made.follows(script, number);
        assert_eq!(said["outcome"], outcome, "{said}");
        let (own, before) = (run_of(&said), made.all());
        made.unchanged(&made.toss(), &before, &toss_first(&wish, &own));
        // The follow-up is tossed first; then the work it followed can be, and both wishes are let go.
        assert_eq!(printed(&made.asked(&["toss"], &wish, &[]))["tossed"], true, "{script} {number}");
        let tossed = printed(&made.toss());
        assert_eq!((&tossed["tossed"], &tossed["commit"]), (&json!(true), &json!(made.end())));
        assert_eq!((made.w.branches(&made.repo), made.w.copies()), (Vec::<String>::new(), vec![]));
        assert_eq!((&made.w.kept("m1")["state"], &made.w.kept(&wish)["state"]), (&json!("let_go"), &json!("let_go")));
    }
}

#[test]
fn a_follow_up_that_failed_or_changed_nothing_holds_nothing_of_the_work_it_followed() {
    for (script, number, outcome) in
        [("follow", 1, "failed"), ("follow", 2, "completed"), ("old-option", 4, "not_started")]
    {
        let made = finished();
        let (wish, said) = made.follows(script, number);
        assert_eq!(said["outcome"], outcome, "{said}");
        let tossed = printed(&made.toss());
        assert_eq!((&tossed["tossed"], &tossed["commit"]), (&json!(true), &json!(made.end())), "{script} {number}");
        assert_eq!((&made.w.kept("m1")["state"], made.w.kept(&wish)["after"].as_str()), (&json!("let_go"), Some("m1")));
        assert!(made.w.branches(&made.repo).iter().all(|branch| *branch != made.branch()));
    }
}

#[test]
fn work_is_not_tossed_while_a_follow_up_s_run_goes_on_it() {
    let made = finished();
    let wish = followed(&made.w, "m1");
    made.w.play("forever", 0);
    let going =
        made.w.going(&[], &["run", "--wish", &wish, "--date", super::DAY, "--repo", made.repo.to_str().unwrap()]);
    wait_for(&made.w.at("rec").join("started"));
    let own = made.w.kept(&wish)["run"].as_str().unwrap().to_string();
    let ran = made.toss();
    assert_eq!(super::refused(&ran), super::said(&toss_first(&wish, &own).replace("{run}", &made.run)));
    assert_eq!((made.w.branches(&made.repo).len(), made.record().get("work")), (2, None));
    made.w.let_end("stop");
    assert_eq!(printed(&ended(going))["outcome"], "failed");
}

// A follow-up's run holds the lock a keep and a toss hold from before it reads the followed work until its wish
// names its run. Held with its copy made and its wish naming no run, it lets no toss of that work by: the toss
// would find no run standing on the work, and the copy would be left on a branch that is gone.
#[test]
fn the_work_is_not_tossed_between_a_follow_up_s_look_at_it_and_the_wish_naming_its_run() {
    let made = finished();
    let (w, wish) = (&made.w, followed(&made.w, "m1"));
    w.play("follow", 0);
    let (go, there) = (w.at("rec").join("claimed"), w.at("rec").join("claimed.at"));
    let vars = [("JELLYMOON_TEST_MADE_WAIT_MS", "60000".into()), ("JELLYMOON_TEST_GO", go.clone().into())];
    let going = w.going(&vars, &["run", "--wish", &wish, "--date", super::DAY, "--repo", made.repo.to_str().unwrap()]);
    wait_for(&there);
    assert_eq!((w.kept(&wish)["run"].as_str(), w.copies().len()), (None, 1));
    let busy = format!(
        "a keep, a toss or the start of a follow-up's run is at work in this home: nothing of run {} was tossed",
        made.run
    );
    let ran = made.toss_in(&made.repo, &[("JELLYMOON_TEST_KEEP_WAIT_MS", "200".into())]);
    assert_eq!(super::refused(&ran), super::said(&busy));
    fs::write(&go, "").unwrap();
    let own = run_of(&printed(&ended(going)));
    assert_eq!(w.record(&own)["start"], made.record()["end"]);
    assert_eq!(super::refused(&made.toss()), super::said(&toss_first(&wish, &own).replace("{run}", &made.run)));
}

#[test]
fn a_follow_up_of_a_follow_up_stands_on_it_and_the_three_are_kept_in_their_order() {
    let made = finished();
    let (second, said) = made.follows("follow", 0);
    let two = run_of(&said);
    // The third wish follows the second: its copy holds what the first two made, and it adds a line.
    let third = followed(&made.w, &second);
    let said = printed(&made.asked(&[], &third, &[]));
    assert_eq!(said["outcome"], "completed", "{said}");
    let three = run_of(&said);
    let record = made.w.record(&three);
    assert_eq!((&record["start"], &record["follows"]), (&made.w.record(&two)["end"], &json!(two)));
    let held = AS_THE_RUN_LEFT_IT
        .replace("edited by the run", "edited by the run, and again by the follow-up")
        .replace("m.txt: none", "m.txt: more from the follow-up");
    assert_eq!(given(&made.w, 3).1, held);
    assert_eq!(given(&made.w, 3).0[14], made.w.record(&two)["session"], "a fork of the second run's session");
    // Each waits for the one before it, and each holds the one before it.
    let before = made.all();
    made.unchanged(&made.asked(&["keep"], &second, &[]), &before, &keep_first(&two, "m1"));
    made.unchanged(&made.toss(), &before, &toss_first(&second, &two));
    made.unchanged(&made.asked(&["toss"], &second, &[]), &before, &toss_first(&third, &three).replace("{run}", &two));
    made.unchanged(&made.asked(&["keep"], &third, &[]), &before, &keep_first(&three, &second));
    for wish in ["m1", &second, &third] {
        assert_eq!(printed(&made.asked(&["keep"], wish, &[]))["onto"], "main", "{wish}");
    }
    assert_eq!(made.read("m.txt"), "more from the follow-up\nmore from the follow-up\n");
    assert_eq!((made.w.branches(&made.repo), made.status().as_str()), (vec![], ""));
}
