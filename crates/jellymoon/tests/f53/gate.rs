//! What F53's gate found, each as the case it was found by: a refusal of the work a follow-up runs on, said as
//! a run's own however its folder was found; a follow-up that changed nothing, which says where its branch is;
//! a fork that worked before it said its session is lost; kept work the checkout does not hold; a keep on top
//! of a joined keep by a git that cannot be told a base; and a follow-up made after the keep it stands on.

use std::fs;
use std::path::Path;

use serde_json::{Value, json};

use super::{DAY, Made, finished, followed, given, printed, support};

const MOVED: &str =
    "the branch of the work this wish follows is not at the commit its record names, so no copy was made of it";
const ELSEWHERE: &str = "the folder this wish would run in is not a checkout of the repo the work it follows is in";
const NOT_HELD: &str = "the checkout's history does not hold the commit the work this wish follows was kept as, so \
                        no copy was made of it";
const NO_BASE: &str = "the work run {own} follows was kept as a new commit, and this git is too old to keep a \
                       follow-up's work on top of one (it is from before 2.40): the work of run {own} was not kept, \
                       and nothing was changed";
const HISTORY: &str = "the branch's history does not hold the commit run {own} started from: the work of run {own} \
                       was not kept, and nothing was changed";

fn run_of(said: &Value) -> String {
    said["run"].as_str().unwrap().to_string()
}

impl Made {
    /// `jellymoon run` for a wish of the day with no folder named, and `more` arguments.
    fn found(&self, wish: &str, more: &[&str]) -> support::Ran {
        self.w.run(&[&["run", "--wish", wish, "--date", DAY][..], more].concat())
    }

    /// A folder of logs, of its own name, in which something was typed in `folder` on the day.
    fn typed_in(&self, name: &str, folder: &Path) -> String {
        let logs = self.w.at(name);
        let typed = support::said(&format!("{DAY}T16:00:00.000Z"), folder.to_str().unwrap(), "something typed there");
        support::write_log(&logs, "-work-alpha/s1.jsonl", &[typed]);
        logs.to_str().unwrap().to_string()
    }

    /// The person commits one more file of their own, and where the branch then is.
    fn one_more(&self, file: &str) -> String {
        fs::write(self.repo.join(file), "one more of the person's own\n").unwrap();
        self.git(&["add", file]);
        self.git(&["commit", "-q", "-m", "one more of the person's own"]);
        self.w.commit(&self.repo, "main")
    }

    fn file(&self, file: &str) -> String {
        fs::read_to_string(self.repo.join(file)).unwrap_or_else(|_| "none".to_string())
    }
}

#[test]
fn a_moved_branch_of_followed_work_is_a_run_s_refusal_however_the_folder_was_found() {
    let made = finished();
    let (w, wish, repo) = (&made.w, followed(&made.w, "m1"), made.repo.to_str().unwrap());
    w.play("follow", 0);
    let logs = made.typed_in("logs", &made.repo);
    // The folder is named, typed in on the day, and found on the list alone: the first run listed it.
    assert_eq!(printed(&w.run(&["project", "list"]))["projects"][0]["name"], "alpha");
    for gone in [false, true] {
        let moved = if gone { vec!["update-ref", "-d"] } else { vec!["update-ref"] };
        let to = if gone { vec![] } else { vec!["main"] };
        made.git(&[&moved[..], &[made.branch().as_str()], &to[..]].concat());
        for more in [&["--repo", repo][..], &["--logs", &logs], &[]] {
            let before = made.all();
            made.unchanged(&made.found(&wish, more), &before, MOVED);
        }
    }
    made.git(&["update-ref", &made.branch(), &made.end()]);
    assert_eq!(printed(&made.found(&wish, &[]))["outcome"], "completed");
}

#[test]
fn another_repo_than_the_followed_work_s_is_a_run_s_refusal_however_the_folder_was_found() {
    let made = finished();
    let (w, wish) = (&made.w, followed(&made.w, "m1"));
    w.play("follow", 0);
    let other = w.repo("elsewhere/alpha");
    let logs = made.typed_in("logs", &other);
    // Named with --repo.
    let before = made.all();
    made.unchanged(&made.found(&wish, &["--repo", other.to_str().unwrap()]), &before, ELSEWHERE);
    // Typed in on the day, with no folder listed under the name.
    printed(&w.run(&["project", "forget", "alpha"]));
    let before = made.all();
    made.unchanged(&made.found(&wish, &["--logs", &logs]), &before, ELSEWHERE);
    // Found on the list alone.
    printed(&w.run(&["project", "add", "--repo", other.to_str().unwrap(), "--date", DAY]));
    let before = made.all();
    made.unchanged(&made.found(&wish, &[]), &before, ELSEWHERE);
    assert!(w.branches(&other).is_empty());
}

#[test]
fn a_follow_up_that_changed_nothing_says_where_its_branch_is() {
    // Of work that waits, its copy was made at that work's commit: the checkout was never there.
    let made = finished();
    let (wish, said) = made.follows("follow", 2);
    let run = run_of(&said);
    let ended = format!(
        "run {run} finished and changed nothing: the branch jellymoon/run-{run} is where the work it follows ended"
    );
    assert_eq!((&said["outcome"], &said["why"]), (&json!("completed"), &json!(ended)), "{said}");
    let tip = made.w.commit(&made.repo, &format!("refs/heads/jellymoon/run-{run}"));
    assert_eq!((tip, &made.w.record(&run)["start"]), (made.end(), &json!(made.end())));
    // The line is one of a record core wrote, as a run's other two are: the run is tossed as one that changed
    // nothing, and not refused as a record that is not Jellymoon's.
    assert_eq!((&made.w.on_page(&wish)["state"], &made.w.kept(&wish)["why"]), (&json!("done"), &json!(ended)));
    let tossed = printed(&made.asked(&["toss"], &wish, &[]));
    let why = format!("run {run}, which changed nothing, was tossed: its branch was deleted, and the wish is let go");
    assert_eq!((&tossed["why"], made.w.commit(&made.repo, &made.branch())), (&json!(why), made.end()), "{tossed}");
    // Of work that was kept, its copy was made where the checkout stood, as any run's is.
    let made = finished();
    assert_eq!(printed(&made.keep(&[]))["kept"], json!(made.end()));
    let stood = made.moved_on();
    let (_, said) = made.follows("follow", 2);
    let run = run_of(&said);
    let ended =
        format!("run {run} finished and changed nothing: the branch jellymoon/run-{run} is where the checkout was");
    assert_eq!((&said["why"], &made.w.record(&run)["start"]), (&json!(ended), &json!(stood)), "{said}");
}

#[test]
fn a_fork_that_worked_before_it_said_its_session_is_lost_is_not_started_again() {
    let made = finished();
    let (wish, said) = made.follows("unkept", 3);
    let (w, run) = (&made.w, run_of(&said));
    let why = format!(
        "run {run} did not finish (Claude Code said it keeps no session to fork after it had worked in the copy): \
         its copy is kept and no star was added"
    );
    assert_eq!((&said["outcome"], &said["why"], w.starts()), (&json!("failed"), &json!(why), 2), "{said}");
    // Its copy is kept as a failed run's is, with what that start left in it, and nothing was committed.
    let record = w.record(&run);
    assert_eq!((&record["forked"], &record["outcome"], record.get("end")), (&json!(true), &json!("failed"), None));
    let copy = w.jm("copies").join(&run);
    assert_eq!(fs::read_to_string(copy.join("m.txt")).unwrap(), "more from the follow-up\n");
    let tip = w.commit(&made.repo, &format!("refs/heads/jellymoon/run-{run}"));
    assert_eq!((tip, &w.kept(&wish)["state"], w.copies()), (made.end(), &json!("needs_you"), vec![run]));
    assert!(given(w, 2).0.iter().any(|arg| arg == "--fork-session"));
}

#[test]
fn a_follow_up_of_kept_work_is_refused_in_a_checkout_whose_history_does_not_hold_that_work() {
    let made = finished();
    let (w, start) = (&made.w, made.record()["start"].as_str().unwrap().to_string());
    assert_eq!(printed(&made.keep(&[]))["kept"], json!(made.end()));
    let wish = followed(w, "m1");
    w.play("follow", 0);
    // A second worktree of the repo, on a branch that never had the work.
    let second = w.at("second").join("alpha");
    fs::create_dir_all(second.parent().unwrap()).unwrap();
    made.git(&["worktree", "add", "-q", "-b", "side", second.to_str().unwrap(), &start]);
    let before = (made.all(), w.persons(&made.repo), support::tree(&second));
    let ran = made.found(&wish, &["--repo", second.to_str().unwrap()]);
    made.unchanged(&ran, &before.0, NOT_HELD);
    assert!(before.2 == support::tree(&second), "the second worktree was changed");
    // The person's own branch, put back behind the commit the work was kept as.
    made.git(&["reset", "-q", "--hard", &start]);
    let before = made.all();
    made.unchanged(&made.found(&wish, &["--repo", made.repo.to_str().unwrap()]), &before, NOT_HELD);
    // The branch holds the work again: the follow-up runs, in a copy made where the checkout stands.
    made.git(&["reset", "-q", "--hard", &made.end()]);
    let said = printed(&made.found(&wish, &["--repo", made.repo.to_str().unwrap()]));
    assert_eq!((&said["outcome"], &w.record(&run_of(&said))["start"]), (&json!("completed"), &json!(made.end())));
}

// Go on from an answer's gate: a question asked on kept work saw that work, so a follow-up of the question needs
// the checkout to hold it, as a follow-up of the work itself does.
#[test]
fn a_follow_up_of_a_question_asked_on_kept_work_needs_a_checkout_that_holds_that_work() {
    let made = finished();
    let (w, start) = (&made.w, made.record()["start"].as_str().unwrap().to_string());
    assert_eq!(printed(&made.keep(&[]))["kept"], json!(made.end()));
    // A follow-up of the kept work that changed nothing: a question, answered in a copy that held the work.
    let (question, said) = made.follows("follow", 2);
    assert_eq!((&said["outcome"], &w.record(&run_of(&said))["start"]), (&json!("completed"), &json!(made.end())));
    let wish = followed(w, &question);
    w.play("follow", 2);
    // The person's own branch, put back behind the commit the work was kept as.
    made.git(&["reset", "-q", "--hard", &start]);
    let before = made.all();
    made.unchanged(&made.found(&wish, &["--repo", made.repo.to_str().unwrap()]), &before, NOT_HELD);
    // The branch holds the work again: the follow-up of the question runs.
    made.git(&["reset", "-q", "--hard", &made.end()]);
    let said = printed(&made.found(&wish, &["--repo", made.repo.to_str().unwrap()]));
    assert_eq!((&said["outcome"], &w.record(&run_of(&said))["start"]), (&json!("completed"), &json!(made.end())));
}

/// Followed work kept as one new commit on the person's own, a follow-up of it that waits, and then a commit of
/// the person's that takes part of the followed work back: `n.txt` is removed again and `b.txt` is back.
fn taken_back() -> (Made, String, String) {
    let made = finished();
    let (wish, said) = made.follows("follow", 0);
    made.moved_on();
    let first = printed(&made.keep(&[]));
    assert!(first["why"].as_str().unwrap().contains("one new commit"), "{first}");
    fs::remove_file(made.repo.join("n.txt")).unwrap();
    fs::write(made.repo.join("b.txt"), "b.txt as it was\n").unwrap();
    made.git(&["add", "-A", "--", "n.txt", "b.txt"]);
    made.git(&["commit", "-q", "-m", "part of the run's work, taken back"]);
    (made, wish, run_of(&said))
}

#[test]
fn a_git_that_cannot_be_told_a_base_keeps_no_follow_up_on_top_of_a_joined_keep() {
    let (made, wish, own) = taken_back();
    let (fake, why) = (made.w.fake_git(), NO_BASE.replace("{own}", &own));
    let keep = |more: &[&str]| {
        let asked = ["run", "keep", "--wish", &wish, "--date", DAY, "--repo", made.repo.to_str().unwrap()];
        made.w.run_with(&fake, &[&asked[..], more].concat())
    };
    for says in ["git version 2.39.5\n", "git version 2.38.0\n", "git version 2.39.3 (Apple Git-146)\n"] {
        fs::write(made.w.at("fake").join("says"), says).unwrap();
        let before = made.all();
        let asked = printed(&keep(&["--ask"]));
        let unfit = json!({"run": own, "wish": wish, "kept": null, "onto": null, "why": why, "fits": false});
        assert_eq!(asked, unfit, "{says}");
        let ran = keep(&[]);
        assert_eq!(
            (ran.code, ran.stdout.as_str(), ran.stderr.as_str()),
            (1, "", format!("jellymoon: {why}\n").as_str())
        );
        assert!(before == made.all(), "a refused keep changed something: {says}");
    }
    // The same git, saying it is one that can be told a base, keeps the follow-up's own change and no more.
    fs::write(made.w.at("fake").join("says"), "git version 2.40.0\n").unwrap();
    assert_eq!(printed(&keep(&[]))["onto"], "main");
    assert_eq!((made.file("b.txt").as_str(), made.file("n.txt").as_str()), ("b.txt as it was\n", "none"));
}

#[test]
fn a_follow_up_kept_after_a_joined_keep_does_not_bring_back_followed_work_the_person_took_back() {
    let (made, wish, _) = taken_back();
    let says = made.git(&["version"]);
    let mut numbers = says.trim_start_matches("git version ").split(['.', ' ']).map(str::parse::<u32>);
    if !matches!((numbers.next(), numbers.next()), (Some(Ok(major)), Some(Ok(minor))) if (major, minor) >= (2, 40)) {
        eprintln!("skipped: this machine's git ({says}) is from before 2.40, which Jellymoon refuses this keep with");
        return;
    }
    let shown = printed(&made.asked(&["show"], &wish, &[]));
    let names = |shown: &Value| -> Vec<String> {
        shown["files"].as_array().unwrap().iter().map(|file| file["name"].as_str().unwrap().to_string()).collect()
    };
    assert_eq!(names(&shown), ["a.txt", "m.txt"]);
    assert_eq!(printed(&made.asked(&["keep"], &wish, &["--ask"]))["fits"], true);
    assert_eq!(printed(&made.asked(&["keep"], &wish, &[]))["onto"], "main");
    assert_eq!(made.git(&["diff", "--name-status", "HEAD^", "HEAD"]), "M\ta.txt\nA\tm.txt");
    assert_eq!((made.file("b.txt").as_str(), made.file("n.txt").as_str()), ("b.txt as it was\n", "none"));
    assert_eq!(made.file("m.txt"), "more from the follow-up\n");
}

#[test]
fn a_follow_up_made_after_a_joined_keep_stands_on_its_own_start_and_not_on_the_kept_commit() {
    let made = finished();
    made.moved_on();
    let first = printed(&made.keep(&[]));
    let joined = made.w.commit(&made.repo, "main");
    assert!(first["why"].as_str().unwrap().contains("one new commit") && first["kept"] == json!(joined), "{first}");
    // The person commits once more, and only then is the follow-up made and run: at their commit.
    let later = made.one_more("later.txt");
    let (wish, said) = made.follows("follow", 0);
    let own = run_of(&said);
    assert_eq!(made.w.record(&own)["start"], json!(later));
    // The branch is put back to the kept commit: it no longer holds the commit the follow-up started from.
    made.git(&["reset", "-q", "--hard", &joined]);
    let before = made.all();
    made.unchanged(&made.asked(&["keep"], &wish, &[]), &before, &HISTORY.replace("{own}", &own));
    assert_eq!(printed(&made.asked(&["keep"], &wish, &["--ask"]))["fits"], false);
    // Back at their commit, it is kept as any run's work.
    made.git(&["reset", "-q", "--hard", &later]);
    assert_eq!(printed(&made.asked(&["keep"], &wish, &[]))["kept"], made.w.record(&own)["end"]);
}
