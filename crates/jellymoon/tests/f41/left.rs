//! What a `jellymoon run` that was killed left with no wish to name it (`docs/development/run.md`, "A copy nothing
//! names"), and the agent a killed command left at work when the next run is asked for.

use std::fs::{self, File};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::apart::exit_of;
use super::named::link;
use super::{DAY, World, ended, found_world, grows, made, printed, silent_world, wait_for};
use crate::support::skipped;

impl World {
    /// Starts `jellymoon run` for a wish and holds it inside the claim, with its copy made and its wish not yet
    /// naming the run, until the test leaves the file `claimed` in `rec`. Returns the command and the run.
    fn held_in_the_claim(&self, wish: &str, repo: &Path) -> (std::process::Child, String) {
        let args = ["run", "--wish", wish, "--date", DAY, "--repo", repo.to_str().unwrap()];
        let held = [
            ("JELLYMOON_TEST_CLAIM_WAIT_MS", "60000".into()),
            ("JELLYMOON_TEST_GO", self.at("rec").join("claimed").into()),
        ];
        let going = self.going(&held, &args);
        let from = Instant::now();
        while !self.jm("runs").join("going.lock").exists() || self.copies().is_empty() {
            assert!(from.elapsed() < Duration::from_secs(30), "the run never came to its claim");
            std::thread::sleep(Duration::from_millis(10));
        }
        (going, self.copies().remove(0))
    }

    /// A `jellymoon run` killed inside the claim: its copy, its branch and its folder in `runs` are left, and no
    /// wish names the run.
    fn killed_in_the_claim(&self, wish: &str, repo: &Path) -> String {
        let (mut going, run) = self.held_in_the_claim(wish, repo);
        going.kill().unwrap();
        going.wait().unwrap();
        assert_eq!((self.copies(), self.branches(repo)), (vec![run.clone()], vec![format!("jellymoon/run-{run}")]));
        assert_eq!(self.kept(wish), made(wish, "alpha"), "the wish named the run before the claim");
        run
    }

    fn kept_for(&self, run: &str) -> bool {
        self.jm("runs").join(run).exists()
    }

    /// What a left run leaves, all there or all gone.
    fn left_of(&self, run: &str, repo: &Path) -> (Vec<String>, Vec<String>, bool) {
        (self.copies(), self.branches(repo), self.kept_for(run))
    }
}

/// The system's lock on a file of `runs`, taken as a command takes it.
fn locked(file: &Path) -> File {
    let held = File::options().write(true).create(true).truncate(false).open(file).unwrap();
    held.lock().unwrap();
    held
}

#[test]
fn a_copy_left_by_a_run_killed_before_its_wish_named_it_is_taken_back_by_the_next_look() {
    let w = found_world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let before = w.persons(&repo);
    let run = w.killed_in_the_claim(&wish, &repo);
    assert_eq!(w.on_page(&wish)["state"], "working");
    assert_eq!((w.copies(), w.branches(&repo), w.kept_for(&run)), (vec![], vec![], false));
    assert_eq!((w.kept(&wish), w.persons(&repo)), (made(&wish, "alpha"), before), "more than the copy was changed");
    // The wish never had a run, and gets its one run now.
    assert_eq!(printed(&w.start(&wish, &repo))["outcome"], "completed");
}

#[test]
fn a_copy_whose_command_still_lives_is_not_taken_back() {
    let w = found_world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (going, run) = w.held_in_the_claim(&wish, &repo);
    assert_eq!(w.on_page(&wish)["state"], "working");
    assert_eq!((w.copies(), w.kept_for(&run)), (vec![run.clone()], true), "a look took a living command's copy");
    fs::write(w.at("rec").join("claimed"), "").unwrap();
    let said = printed(&ended(going));
    assert_eq!((&said["outcome"], &said["run"]), (&json!("completed"), &json!(run)));
}

// A command makes its start lock and takes it in two steps, and a look that lists the runs left between the two
// takes the run for a left one. It then waits for the copies' lock, and asks again when it has it. Here the look
// is made to wait for that lock, and the start lock is taken meanwhile. A look slower than the second waited
// here finds the start lock held when it lists, and leaves the run for that.
#[test]
fn a_copy_whose_start_lock_is_taken_while_the_look_waits_for_the_copies_lock_is_not_taken_back() {
    let w = found_world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let run = w.killed_in_the_claim(&wish, &repo);
    let all = (vec![run.clone()], vec![format!("jellymoon/run-{run}")], true);
    let copies = locked(&w.jm("runs").join("clean.lock"));
    let (logs, long) = (w.at("logs"), [("JELLYMOON_TEST_COPY_WAIT_MS", "60000".into())]);
    let look = w.going(&long, &["page", "--date", DAY, "--logs", logs.to_str().unwrap()]);
    thread::sleep(Duration::from_secs(1));
    let start = locked(&w.jm("runs").join(&run).join("start.lock"));
    copies.unlock().unwrap();
    assert_eq!(ended(look).code, 0);
    assert_eq!(w.left_of(&run, &repo), all, "the copy of a command that holds its start lock was taken back");
    drop(start); // closed, not only let go: Windows moves no folder a file of which is open (`gone::remove`)
    w.on_page(&wish);
    assert_eq!(w.left_of(&run, &repo), (vec![], vec![], false));
}

// The command lets go of its start lock just before its run's own: a run whose own lock is held is still ending.
#[test]
fn a_left_run_whose_own_lock_is_held_is_left_alone_until_it_is_let_go() {
    let w = found_world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let run = w.killed_in_the_claim(&wish, &repo);
    let held = locked(&w.jm("runs").join(&run).join("run.lock"));
    w.on_page(&wish);
    let all = (vec![run.clone()], vec![format!("jellymoon/run-{run}")], true);
    assert_eq!(w.left_of(&run, &repo), all, "a run that something holds was taken for one left behind");
    drop(held); // closed, not only let go: Windows moves no folder a file of which is open (`gone::remove`)
    w.on_page(&wish);
    assert_eq!(w.left_of(&run, &repo), (vec![], vec![], false));
}

// A link at the copy's name leads out of Jellymoon's home: it is removed as itself, with everything else the home
// and the repo hold of the run, and what it leads to is not touched.
#[test]
fn a_left_copy_that_is_a_link_is_taken_back_and_what_the_link_leads_to_is_not_touched() {
    let w = found_world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let run = w.killed_in_the_claim(&wish, &repo);
    let (copy, outside) = (w.jm("copies").join(&run), w.at("outside"));
    fs::remove_dir_all(&copy).unwrap();
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("mine.txt"), "the person's own\n").unwrap();
    if !link(&outside, &copy) {
        return skipped("no link to a folder can be made here");
    }
    w.on_page(&wish);
    assert_eq!(fs::read_to_string(outside.join("mine.txt")).unwrap(), "the person's own\n");
    assert!(fs::symlink_metadata(&copy).is_err(), "the link was left in the home");
    assert_eq!(w.left_of(&run, &repo), (vec![], vec![], false));
    assert_eq!(w.git(&repo, &["worktree", "list", "--porcelain"]).matches("worktree ").count(), 1);
}

// A copy's record is a file anyone who can write the home can write. One edited to name another repo, which does
// not list this copy as a worktree of its own, has nothing done in that repo: a branch of the run's name there
// stays. What the home holds of the run goes.
#[test]
fn a_left_copy_whose_record_names_another_repo_removes_no_branch_there() {
    let w = found_world();
    let (repo, beta, wish) = (w.repo("alpha"), w.repo("beta"), w.wish("alpha"));
    let run = w.killed_in_the_claim(&wish, &repo);
    let branch = format!("jellymoon/run-{run}");
    w.git(&beta, &["branch", &branch]);
    let file = w.jm("runs").join(&run).join("copy.json");
    let mut record: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
    record["repo"] = json!(w.git(&beta, &["rev-parse", "--absolute-git-dir"]));
    fs::write(&file, record.to_string()).unwrap();
    let before = w.persons(&beta);
    w.on_page(&wish);
    assert_eq!((w.branches(&beta), w.persons(&beta)), (vec![branch], before), "a repo a record named was changed");
    assert_eq!((w.copies(), w.kept_for(&run)), (vec![], false));
}

// With `worktree.useRelativePaths` (git 2.48) git writes a copy's `gitdir` file as a path from the copy's git
// folder. Jellymoon's git calls switch that off, and the file is written so by hand, as an older copy's is.
#[test]
fn a_left_copy_whose_gitdir_file_is_a_path_from_its_git_folder_is_taken_back_whole() {
    let w = found_world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    w.git(&repo, &["config", "worktree.useRelativePaths", "true"]);
    let run = w.killed_in_the_claim(&wish, &repo);
    let record = fs::read_to_string(w.jm("runs").join(&run).join("copy.json")).unwrap();
    let own = fs::canonicalize(serde_json::from_str::<Value>(&record).unwrap()["own"].as_str().unwrap()).unwrap();
    let copy = fs::canonicalize(w.jm("copies").join(&run)).unwrap();
    let wrote = fs::read_to_string(own.join("gitdir")).unwrap();
    assert!(Path::new(wrote.trim_end()).is_absolute(), "git wrote {wrote:?} for a copy of Jellymoon's");
    let same = own.components().zip(copy.components()).take_while(|(one, other)| one == other).count();
    let up = own.components().skip(same).map(|_| "..".to_string());
    let down = copy.components().skip(same).map(|name| name.as_os_str().to_str().unwrap().to_string());
    fs::write(own.join("gitdir"), up.chain(down).collect::<Vec<_>>().join("/") + "/.git\n").unwrap();
    w.on_page(&wish);
    assert_eq!(w.left_of(&run, &repo), (vec![], vec![], false));
}

#[test]
fn a_branch_that_holds_a_commit_of_its_own_is_kept_when_its_copy_is_taken_back() {
    let w = found_world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let run = w.killed_in_the_claim(&wish, &repo);
    let branch = format!("jellymoon/run-{run}");
    w.git(&w.jm("copies").join(&run), &["commit", "-q", "--allow-empty", "-m", "the person's own"]);
    let mine = w.commit(&repo, &branch);
    w.on_page(&wish);
    assert_eq!((w.copies(), w.kept_for(&run)), (vec![], false));
    assert_eq!((w.branches(&repo), w.commit(&repo, &branch)), (vec![branch.clone()], mine));
    assert_ne!(w.commit(&repo, &branch), w.commit(&repo, "HEAD"));
}

// Killed later, a command has written the run's record: before its wish named the run, or after a run that was
// never started had its wish told so. A record that names no child is of a copy nothing ran in. One that names a
// child is of a copy an agent may have written in: that copy is kept for a look, as a cut-off run's is.
#[test]
fn a_copy_no_wish_names_is_taken_back_on_a_record_with_no_child_and_kept_as_failed_on_one_with_a_child() {
    let record = |child: Value| {
        let mut record = json!({"wish": "m1", "day": DAY, "started": 1, "outcome": null, "denied": 0, "skipped": 0});
        record.as_object_mut().unwrap().extend(child.as_object().unwrap().clone());
        record.to_string()
    };
    let w = found_world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let run = w.killed_in_the_claim(&wish, &repo);
    fs::write(w.jm("runs").join(&run).join("run.json"), record(json!({}))).unwrap();
    w.on_page(&wish);
    assert_eq!((w.copies(), w.branches(&repo), w.kept_for(&run)), (vec![], vec![], false));

    // A pid no process of this system has, so nothing is signalled.
    let run = w.killed_in_the_claim(&wish, &repo);
    fs::write(w.jm("runs").join(&run).join("run.json"), record(json!({"pid": 4_000_000_000_u32, "mark": "none"})))
        .unwrap();
    w.on_page(&wish);
    let why = format!("run {run} was cut off before it ended: its copy is kept and no star was added");
    let kept = w.record(&run);
    assert_eq!((&kept["outcome"], &kept["why"]), (&json!("failed"), &json!(why)));
    assert_eq!((w.copies(), w.branches(&repo)), (vec![run.clone()], vec![format!("jellymoon/run-{run}")]));
    assert_eq!(w.kept(&wish), made(&wish, "alpha"), "a run no wish names changed a wish");
    // It is done with: the next look finds nothing more to do, and the copy stays one of the failed copies.
    w.on_page(&wish);
    assert!(!w.jm("runs").join(&run).join("start.lock").exists() && w.copies() == [run]);
}

// Three commands killed with their agents at work would leave three agents beside the three runs that may go. The
// next `jellymoon run` ends what killed commands left before it counts.
#[test]
fn the_agent_a_killed_command_left_is_ended_before_the_next_run_is_counted_and_started() {
    if !cfg!(unix) {
        return skipped("where there are no signals nothing is stopped, as run.md says");
    }
    let w = silent_world();
    let repo = w.repo("alpha");
    w.typed_in("-work-alpha", &repo);
    let (first, second) = (w.wish("alpha"), w.wish("alpha"));
    w.play("scribble", 0);
    assert_eq!(w.asked(&first).code, 0);
    let run = w.run_of(&first);
    let file: &Path = &w.jm("copies").join(&run).join("scribble.txt");
    wait_for(file);
    fs::write(w.at("rec").join("cut"), "").unwrap(); // `fake-asker` kills the command it started
    assert_ne!(exit_of(&w), "0");
    assert!(grows(file), "the agent the killed command left was to be editing still");

    w.play("edits", 0);
    assert_eq!(printed(&w.start(&second, &repo))["outcome"], "completed");
    assert!(w.freed("held") && !grows(file), "the agent a killed command left is still at work beside the new run");
    // The first wish is told by the look, as before.
    assert_eq!(w.on_page(&first)["state"], "needs_you");
}
