//! What two reviews of F53 left, each as the case it names: a fork that left the run's branch, committed on it
//! or left a file git ignores before it said its session is lost; a follow-up kept by a git that cannot be told
//! a base, where it needs none; and a follow-up that has had no run, which a toss lets go.

use std::fs;

use serde_json::{Value, json};

use super::{DAY, Made, finished, followed, given, printed};

const WORKED: &str = "run {own} did not finish (Claude Code said it keeps no session to fork after it had worked in \
                      the copy): its copy is kept and no star was added";
const LET_GO: &str = "wish {wish} had no run and was let go: nothing of a repo or a copy was touched";
const NO_RUN: &str = "wish {wish} was let go, so no run is started for it";

fn run_of(said: &Value) -> String {
    said["run"].as_str().unwrap().to_string()
}

impl Made {
    /// Lends the made-up agent the real git, by its full path, as a harness with a shell would have it.
    fn lends_git(&self) {
        self.w.fake_git();
        let real = fs::read_to_string(self.w.at("fake").join("real")).unwrap();
        fs::write(self.w.at("rec").join("git"), real).unwrap();
    }

    /// `jellymoon run keep` of a wish under a git that says it is 2.39.5, with `more` arguments.
    fn kept_by_an_old_git(&self, wish: &str, more: &[&str]) -> Value {
        let fake = self.w.fake_git();
        fs::write(self.w.at("fake").join("says"), "git version 2.39.5\n").unwrap();
        let asked = ["run", "keep", "--wish", wish, "--date", DAY, "--repo", self.repo.to_str().unwrap()];
        printed(&self.w.run_with(&fake, &[&asked[..], more].concat()))
    }
}

/// A fork that did something in its copy and then said its session is lost has failed, in the words of one that
/// worked, and its harness was started once: no second start is given what the first left.
fn not_started_again(number: usize) -> (Made, String) {
    let made = finished();
    made.lends_git();
    // Shared by every worktree of the repo: the copy's git ignores the name as the person's own does.
    let exclude = made.repo.join(".git").join("info").join("exclude");
    fs::create_dir_all(exclude.parent().unwrap()).unwrap();
    fs::write(exclude, "*.ignored\n").unwrap();
    let (wish, said) = made.follows("unkept", number);
    let (own, why) = (run_of(&said), json!(WORKED.replace("{own}", &run_of(&said))));
    assert_eq!((&said["outcome"], &said["why"], made.w.starts()), (&json!("failed"), &why, 2), "{number}: {said}");
    let record = made.w.record(&own);
    assert_eq!((&record["forked"], &record["outcome"], record.get("end")), (&json!(true), &json!("failed"), None));
    assert_eq!((&made.w.kept(&wish)["state"], made.w.copies()), (&json!("needs_you"), vec![own.clone()]));
    assert!(given(&made.w, 2).0.iter().any(|arg| arg == "--fork-session"));
    (made, own)
}

#[test]
fn a_fork_that_left_the_run_s_branch_and_committed_before_it_said_lost_is_not_started_again() {
    // It left for no branch, and for a branch it made: the copy's `HEAD` names the run's branch in neither.
    for (number, on) in [(6, None), (7, Some("refs/heads/made-by-the-fork"))] {
        let (made, own) = not_started_again(number);
        // The run's branch is where the copy was made and git finds no changed file: what the fork did is
        // one commit the run's branch does not name, which the copy stands on.
        let (w, copy) = (&made.w, made.w.jm("copies").join(&own));
        assert_eq!(w.commit(&made.repo, &format!("refs/heads/jellymoon/run-{own}")), made.end());
        assert_eq!(w.git(&copy, &["status", "--porcelain", "--ignored", "--untracked-files=all"]), "");
        assert_eq!(w.git(&copy, &["log", "-1", "--format=%s"]), "committed by the fork itself");
        assert_eq!(w.git(&copy, &["rev-parse", "HEAD^"]), made.end());
        let head = String::from_utf8(w.git_ran(&copy, &["symbolic-ref", "-q", "HEAD"]).stdout).unwrap();
        assert_eq!(Some(head.trim()).filter(|head| !head.is_empty()), on, "{number}");
    }
}

#[test]
fn a_fork_that_committed_on_the_run_s_branch_or_left_an_ignored_file_before_it_said_lost_is_not_started_again() {
    // Committed on the run's own branch: git finds no changed file, and the branch is past the copy's start.
    let (made, own) = not_started_again(4);
    let (w, copy) = (&made.w, made.w.jm("copies").join(&own));
    let tip = w.commit(&made.repo, &format!("refs/heads/jellymoon/run-{own}"));
    assert_eq!(w.git(&copy, &["status", "--porcelain", "--ignored", "--untracked-files=all"]), "");
    assert_eq!((w.git(&made.repo, &["rev-parse", &format!("{tip}^")]), tip == made.end()), (made.end(), false));
    // One file of a name an exclude of the repo names: a status that leaves ignored files out finds nothing.
    let (made, own) = not_started_again(5);
    let (w, copy) = (&made.w, made.w.jm("copies").join(&own));
    assert_eq!(w.commit(&made.repo, &format!("refs/heads/jellymoon/run-{own}")), made.end());
    assert_eq!(w.git(&copy, &["status", "--porcelain", "--untracked-files=all"]), "");
    assert_eq!(fs::read_to_string(copy.join("left.ignored")).unwrap(), "left by the fork\n");
}

// The fork did nothing at all, and the copy's `HEAD` is asked of a git that cannot answer: a copy that cannot
// be asked about is not one that is as it was made, so the fork is not started again.
#[test]
fn a_fork_whose_copy_git_cannot_answer_for_is_not_started_again() {
    let made = finished();
    let fake = made.w.fake_git();
    for (file, holds) in [("on", "symbolic-ref"), ("fails", "128")] {
        fs::write(made.w.at("fake").join(file), holds).unwrap();
    }
    let wish = followed(&made.w, &made.wish);
    made.w.play("unkept", 0);
    let asked = ["run", "--wish", &wish, "--date", DAY, "--repo", made.repo.to_str().unwrap()];
    let said = printed(&made.w.run_with(&fake, &asked));
    let why = json!(WORKED.replace("{own}", &run_of(&said)));
    assert_eq!((&said["outcome"], &said["why"], made.w.starts()), (&json!("failed"), &why, 2), "{said}");
}

#[test]
fn a_follow_up_that_needs_no_base_is_kept_by_a_git_that_cannot_be_told_one() {
    // Its followed work was kept by moving the branch forward: the follow-up's own start is in the branch.
    let made = finished();
    let (wish, said) = made.follows("follow", 0);
    let end = made.w.record(&run_of(&said))["end"].clone();
    let first = printed(&made.keep(&[]));
    assert!(first["why"].as_str().unwrap().contains("moved forward") && first["kept"] == json!(made.end()), "{first}");
    assert_eq!(made.kept_by_an_old_git(&wish, &["--ask"])["fits"], true);
    let kept = made.kept_by_an_old_git(&wish, &[]);
    assert_eq!(
        (&kept["kept"], &kept["onto"], json!(made.w.commit(&made.repo, "main"))),
        (&end, &json!("main"), end.clone())
    );
    assert_eq!(fs::read_to_string(made.repo.join("m.txt")).unwrap(), "more from the follow-up\n");
    // It was made after a keep that made a new commit: its copy was made where the checkout stood then.
    let made = finished();
    made.moved_on();
    let first = printed(&made.keep(&[]));
    assert!(first["why"].as_str().unwrap().contains("one new commit"), "{first}");
    let (wish, said) = made.follows("follow", 0);
    let end = made.w.record(&run_of(&said))["end"].clone();
    assert_eq!(made.kept_by_an_old_git(&wish, &["--ask"])["fits"], true);
    let kept = made.kept_by_an_old_git(&wish, &[]);
    assert_eq!(
        (&kept["kept"], &kept["onto"], json!(made.w.commit(&made.repo, "main"))),
        (&end, &json!("main"), end.clone())
    );
}

#[test]
fn a_follow_up_that_has_had_no_run_is_let_go_by_a_toss_whether_the_work_it_follows_waits_or_was_tossed() {
    for tossed in [true, false] {
        let made = finished();
        let (w, wish) = (&made.w, followed(&made.w, "m1"));
        w.play("follow", 0);
        if tossed {
            // The work it follows is gone, so it cannot be run; before this it could not be let go either.
            assert_eq!(printed(&made.toss())["tossed"], true);
            assert_eq!(made.asked(&[], &wish, &[]).code, 1);
        }
        let (was, held) = (w.kept(&wish), (w.persons(&made.repo), made.git(&["for-each-ref"]), made.record()));
        let let_go = printed(&made.asked(&["toss"], &wish, &[]));
        let why = LET_GO.replace("{wish}", &wish);
        assert_eq!(let_go, json!({"run": null, "wish": wish, "tossed": true, "commit": null, "why": why}));
        let mut now = was;
        (now["state"], now["why"]) = (json!("let_go"), json!("the wish had no run, and was let go"));
        assert_eq!((w.kept(&wish), &w.on_page(&wish)["state"]), (now, &json!("let_go")));
        assert!(held == (w.persons(&made.repo), made.git(&["for-each-ref"]), made.record()), "something was touched");
        // No run is started for it, and the work it followed is tossed as work no wish stands on.
        let before = made.all();
        made.unchanged(&made.asked(&[], &wish, &[]), &before, &NO_RUN.replace("{wish}", &wish));
        assert_eq!((w.starts(), w.copies()), (1, vec![]));
        let first = printed(&made.toss());
        assert_eq!((&first["tossed"], w.branches(&made.repo).len()), (&json!(!tossed), 0), "{first}");
    }
}
