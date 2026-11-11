//! What `jellymoon run` does not start: no harness, a harness too old, a wish with no one folder, a wish that has
//! had its run. Where nothing is started, nothing is left: no copy, no branch, no record, and the wish as it was.

use std::ffi::OsString;
use std::path::Path;
use std::time::{Duration, Instant};
use std::{fs, thread};

use serde_json::json;

use super::limits::wait_for;
use super::{DAY, STAR, World, printed, ran, refused, world, world_in};
use crate::support::{said, skipped, tree, write_log};

impl World {
    /// `jellymoon run` for a wish of the day, with no repo named: the folder is found in the day's session logs.
    pub fn start_found(&self, wish: &str) -> crate::Ran {
        self.run(&["run", "--wish", wish, "--date", DAY])
    }

    /// A session log, in the harness's own folder, of one thing typed in `folder` on the day.
    pub fn typed_in(&self, session: &str, folder: &Path) {
        let line = said("2026-03-10T16:00:00.000Z", folder.to_str().unwrap(), "look at the uploader");
        write_log(&self.at("harness").join("projects"), &format!("{session}/s.jsonl"), &[line]);
    }

    /// Nothing of a run is left in Jellymoon's home or in the repo, and the agent was started `starts` times.
    fn nothing_is_left(&self, repo: &Path, starts: usize) {
        let runs = fs::read_dir(self.jm("runs")).into_iter().flatten().map(|entry| entry.unwrap().file_name());
        let runs: Vec<_> = runs.filter(|name| name.to_string_lossy().starts_with('r')).collect();
        assert_eq!((self.copies(), runs, self.branches(repo)), (vec![], vec![], Vec::<String>::new()));
        assert_eq!(self.git(repo, &["worktree", "list", "--porcelain"]).matches("worktree ").count(), 1);
        assert_eq!(self.starts(), starts);
    }
}

#[test]
fn with_no_harness_nothing_is_made_and_the_wish_stays_as_it_was() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    fs::create_dir(w.at("empty")).unwrap();
    let before = tree(w.dir.path());
    let nowhere = [("PATH", OsString::from(w.at("empty")))];
    let args = ["run", "--wish", &wish, "--date", DAY, "--repo", repo.to_str().unwrap()];
    let said = printed(&w.run_with(&nowhere, &args));
    let why = "finding Claude Code: no `claude` program in PATH, ~/.local/bin, ~/.claude/local; install Claude \
               Code, or name the program with JELLYMOON_CLAUDE";
    let want = json!({"run": null, "wish": wish, "outcome": "not_started", "star": null, "denied": 0, "why": why});
    assert_eq!(said, want);
    assert!(tree(w.dir.path()) == before, "nothing was written");
}

#[test]
fn a_harness_too_old_to_know_restricted_starts_no_run_and_the_wish_can_be_run_again() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (kept, before) = (w.kept(&wish), w.persons(&repo));
    w.play("old", 0);
    let old = w.start(&wish, &repo);
    let said = printed(&old);
    let why = "nothing was started: this Claude Code is too old for runs, as it does not know --restricted; \
               update Claude Code and run the wish again";
    let want = json!({"run": null, "wish": wish, "outcome": "not_started", "star": null, "denied": 0, "why": why});
    assert_eq!(said, want);
    // It was asked once, with the flag, and never again without it.
    assert_eq!(w.rec("arg1").as_deref(), Some("--restricted"));
    w.nothing_is_left(&repo, 1);
    assert_eq!((w.kept(&wish), w.persons(&repo)), (kept, before));

    w.play("edits", 0);
    let (_, said) = ran(&w.start(&wish, &repo));
    assert_eq!((&said["outcome"], &said["star"]), (&json!("completed"), &json!(STAR)));
}

#[test]
fn a_wish_runs_in_the_one_folder_of_its_name_that_was_typed_in_that_day() {
    // In the system's own temporary folder: Cargo's may be inside a harness worktree of this repo, and a folder
    // there is named after the repo the worktree is of, which is the rule the fourth case here is about.
    let w = world_in(tempfile::tempdir().unwrap());
    let (repo, other) = (w.repo("one/alpha"), w.repo("beta"));
    let (alpha, gamma) = (w.wish("alpha"), w.wish("gamma"));
    // No session logs at all, then logs that name no such folder.
    let before = tree(w.dir.path());
    assert!(refused(&w.start_found(&alpha)).contains("no Claude Code session logs were found"));
    assert!(tree(w.dir.path()) == before);
    w.typed_in("-work-one-alpha", &repo);
    w.typed_in("-work-beta", &other);
    let before = tree(w.dir.path());
    let none =
        "jellymoon: no folder of the name the wish gives was typed in on 2026-03-10; name the folder with --repo\n";
    assert_eq!(refused(&w.start_found(&gamma)), none);
    assert!(tree(w.dir.path()) == before);

    // One folder of the name: the run is started there, and in no other repo.
    let (_, said) = ran(&w.start_found(&alpha));
    assert_eq!(said["outcome"], "completed");
    assert_eq!((w.branches(&repo).len(), w.branches(&other).len()), (1, 0));
    // A session in a harness worktree of the repo is the repo's own folder, not a second one.
    w.typed_in("-work-one-alpha-wt", &repo.join(".claude").join("worktrees").join("fix").join("web"));
    assert_eq!(ran(&w.start_found(&w.wish("alpha"))).1["outcome"], "completed");

    // A second folder of the same name: refused, and nothing is written.
    let twin = w.repo("two/alpha");
    w.typed_in("-work-two-alpha", &twin);
    let (wish, before) = (w.wish("alpha"), w.persons(&twin));
    let all = tree(w.dir.path());
    let two = "jellymoon: 2 folders of the name the wish gives were typed in on 2026-03-10; name the one with --repo\n";
    assert_eq!(refused(&w.start_found(&wish)), two);
    assert!(tree(w.dir.path()) == all && w.persons(&twin) == before);
    // Named, it runs there.
    assert_eq!(ran(&w.start(&wish, &twin)).1["outcome"], "completed");
    assert_eq!((w.branches(&repo).len(), w.branches(&twin).len()), (2, 1));

    // A folder inside a repo, typed in that day: refused with the reason, and the repo above it is not used.
    let inside = other.join("web");
    fs::create_dir(&inside).unwrap();
    w.typed_in("-work-beta-web", &inside);
    let (wish, before) = (w.wish("web"), w.persons(&other));
    let all = tree(w.dir.path());
    // The reason is `copy make`'s own: git is not let look above the folder, so to git it is no repo.
    let below = "jellymoon: the folder of that name typed in on 2026-03-10 is not one a run can copy: it is not a \
                 git repo git could open: ";
    let said = w.start_found(&wish);
    assert!(refused(&said).starts_with(below) && !said.stderr.contains("--repo"), "{}", said.stderr);
    assert!(tree(w.dir.path()) == all && w.persons(&other) == before && w.branches(&other).is_empty());
}

#[test]
fn a_repo_of_another_name_no_repo_a_second_run_and_a_wish_that_is_not_there_are_refused() {
    let w = world();
    let (repo, other) = (w.repo("alpha"), w.repo("beta"));
    let wish = w.wish("alpha");
    fs::create_dir(w.at("work").join("plain")).unwrap();
    let plain = w.wish("plain");
    let before = tree(w.dir.path());
    let name = "jellymoon: --repo is not the folder the wish is for: its last name is not the wish's project\n";
    assert_eq!(refused(&w.start(&wish, &other)), name);
    assert_eq!(refused(&w.start(&wish, &repo.join("src"))), name);
    let relative = w.run(&["run", "--wish", &wish, "--date", DAY, "--repo", "alpha"]);
    assert!(refused(&relative).contains("--repo must be a full path"));
    let no_wish = "jellymoon: the page of 2026-03-10 has no made wish of that id; a made wish's id is m1 to m8\n";
    for id in ["m3", "m9", "w1", "", "M1", "../m1", " m1"] {
        assert_eq!(refused(&w.start(id, &repo)), no_wish, "{id:?}");
    }
    let day = w.run(&["run", "--wish", &wish, "--date", "2026-03-11", "--repo", repo.to_str().unwrap()]);
    assert!(refused(&day).contains("the page of 2026-03-11 has no made wish of that id"));
    let day = w.run(&["run", "--wish", &wish, "--date", "today", "--repo", repo.to_str().unwrap()]);
    assert_eq!(refused(&day), "jellymoon: --date must be a day as YYYY-MM-DD\n");
    assert!(tree(w.dir.path()) == before, "nothing was written");
    for args in [&["run"][..], &["run", "--date", DAY], &["run", "--wish", "m1", "--stop"], &["run", "m1"]] {
        let ran = w.run(args);
        assert!(ran.code == 2 && ran.stdout.is_empty() && ran.stderr.contains("usage:"), "{args:?}: {}", ran.stderr);
    }

    // A folder that is no git repo is F38's own refusal, and the wish is as it was.
    let kept = w.kept(&plain);
    assert!(refused(&w.start(&plain, &w.at("work").join("plain"))).contains("is not a git repo"));
    assert_eq!(w.kept(&plain), kept);
    w.nothing_is_left(&repo, 0);

    // One run for a wish: a second is refused whatever the first came to, and nothing is written.
    let (run, _) = ran(&w.start(&wish, &repo));
    let before = tree(w.dir.path());
    let second = format!("jellymoon: wish {wish} has had its run, {run}: a made wish gets one run\n");
    assert_eq!(refused(&w.start(&wish, &repo)), second);
    assert!(tree(w.dir.path()) == before && w.starts() == 1);
}

// The harness turns out too old while another change holds the day. Held for a while, the day is waited for; held
// past the wait, the copy is taken back all the same, and the next look finds a wish whose run left nothing.
#[test]
fn a_harness_too_old_while_the_day_is_locked_leaves_the_wish_as_it_was_and_no_copy() {
    for (wait, held) in [(None, 4), (Some("300"), 5)] {
        let w = world();
        let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
        let before = w.kept(&wish);
        w.play("old-late", 0);
        let lock = w.jm("pages").join(format!("{DAY}.ruled.lock"));
        let short: Vec<_> = wait.map(|ms| ("JELLYMOON_TEST_END_WAIT_MS", OsString::from(ms))).into_iter().collect();
        let args = ["run", "--wish", &wish, "--date", DAY, "--repo", repo.to_str().unwrap()];
        let ended = thread::scope(|scope| {
            let going = scope.spawn(|| w.run_with(&short, &args));
            wait_for(&w.at("rec").join("started"));
            fs::write(&lock, "").unwrap();
            fs::write(w.at("rec").join("go"), "").unwrap();
            thread::sleep(Duration::from_secs(held));
            fs::remove_file(&lock).unwrap();
            going.join().unwrap()
        });
        if wait.is_none() {
            assert_eq!(printed(&ended)["outcome"], "not_started");
        } else {
            assert_eq!((ended.code, ended.stdout.as_str()), (1, ""), "{}", ended.stderr);
            assert_eq!((&w.on_page(&wish)["state"], &w.on_page(&wish)["stars"]), (&json!("working"), &json!([])));
        }
        let runs = fs::read_dir(w.jm("runs")).unwrap().map(|entry| entry.unwrap().file_name().into_string().unwrap());
        let runs: Vec<String> = runs.filter(|name| name != "no-hooks" && !name.contains("lock")).collect();
        assert_eq!((w.copies(), w.branches(&repo), runs), (vec![], vec![], Vec::<String>::new()), "{wait:?}");
        assert_eq!(w.kept(&wish), before, "{wait:?}: the wish is as it was");
        w.play("edits", 0);
        assert_eq!(ran(&w.start(&wish, &repo)).1["star"], json!(STAR), "{wait:?}: and can be run again");
    }
}

// The repo is moved away while a harness that turns out too old runs, so git cannot be asked to take the copy back.
// What is in Jellymoon's own home goes all the same; the branch and git's note of the copy are in the repo, wherever
// it is now. The wish is as it was, and runs where the repo went.
#[test]
fn a_repo_moved_away_under_a_harness_too_old_leaves_nothing_of_the_run_in_jellymoon_s_home() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let before = w.kept(&wish);
    w.play("old-late", 0);
    let moved = w.at("work").join("moved").join("alpha");
    fs::create_dir(moved.parent().unwrap()).unwrap();
    let (ended, renamed) = thread::scope(|scope| {
        let going = scope.spawn(|| w.start(&wish, &repo));
        wait_for(&w.at("rec").join("started"));
        let renamed = fs::rename(&repo, &moved);
        fs::write(w.at("rec").join("go"), "").unwrap();
        (going.join().unwrap(), renamed)
    });
    if let (Err(why), true) = (&renamed, cfg!(windows)) {
        return skipped(&format!("the repo could not be moved while a run had its copy: {why}"));
    }
    renamed.unwrap();
    // The one line is git's reason: the run was not started, and its branch could not be taken back.
    let why = refused(&ended);
    let runs = fs::read_dir(w.jm("runs")).unwrap().map(|entry| entry.unwrap().file_name().into_string().unwrap());
    let runs: Vec<String> = runs.filter(|name| name.starts_with('r')).collect();
    assert_eq!((w.copies(), runs), (vec![], Vec::<String>::new()), "no copy and no folder of the run: {why}");
    assert_eq!((w.kept(&wish), w.starts()), (before, 1), "the wish is as it was");
    w.play("edits", 0);
    assert_eq!(ran(&w.start(&wish, &moved)).1["star"], json!(STAR), "and can be run where the repo is now");
}

// A record is a file anyone who can write the home can write. An end in it is believed only when it is one core
// writes, for this wish, and, for a finished run, when the copy is gone as a finished run's is.
#[test]
fn an_end_written_into_a_cut_off_run_s_record_by_hand_gives_no_star() {
    for edit in 0..3 {
        let w = world();
        let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
        w.play("slow", 0);
        let mut command = w.start_going(&wish, &repo);
        wait_for(&w.at("rec").join("started"));
        command.kill().unwrap();
        command.wait().unwrap();
        let run = w.kept(&wish)["run"].as_str().unwrap().to_string();
        let mut record = w.record(&run);
        let fine = format!("run {run} finished: its work is one commit on the branch jellymoon/run-{run}");
        record["outcome"] = json!("completed");
        record["why"] = json!(if edit == 0 { "words nobody but the editor wrote" } else { &fine });
        // 0: words that are not core's. 1: the copy still stands. 2: the record is another wish's.
        record["wish"] = json!(if edit == 2 { "m2" } else { &wish });
        if cfg!(windows) {
            w.let_end("go"); // there the agent is not stopped, and the folder a command stands in cannot be removed
        }
        if edit != 1 {
            fs::remove_dir_all(w.jm("copies").join(&run)).unwrap();
            fs::remove_file(w.jm("runs").join(&run).join("copy.json")).unwrap();
        }
        fs::write(w.jm("runs").join(&run).join("run.json"), record.to_string()).unwrap();
        let on_page = w.on_page(&wish);
        assert_eq!((&on_page["state"], &on_page["stars"]), (&json!("needs_you"), &json!([])), "edit {edit}");
        let cut = format!("run {run} was cut off before it ended: its copy is kept and no star was added");
        assert_eq!(w.kept(&wish)["why"], json!(cut), "edit {edit}");
        fs::write(w.at("rec").join("go"), "").unwrap();
    }
}

/// Whether nothing holds a run's lock: it is free, or gone with everything else of the run.
fn let_go(lock: &Path) -> bool {
    fs::File::open(lock).map_or(true, |lock| lock.try_lock().is_ok())
}

// The day stays locked for the whole of the wait, and a page is drawn the moment the run's lock is let go. There
// is no moment at which that run looks cut off: it holds its lock until nothing of it is kept.
#[test]
fn a_look_the_moment_a_run_that_never_started_lets_go_finds_the_wish_as_it_was() {
    let round = |_| {
        let w = world();
        let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
        let before = w.kept(&wish);
        w.play("old-late", 0);
        let day = w.jm("pages").join(format!("{DAY}.ruled.lock"));
        let short = [("JELLYMOON_TEST_END_WAIT_MS", OsString::from("500"))];
        let args = ["run", "--wish", &wish, "--date", DAY, "--repo", repo.to_str().unwrap()];
        let (ended, seen) = thread::scope(|scope| {
            let going = scope.spawn(|| w.run_with(&short, &args));
            wait_for(&w.at("rec").join("started"));
            let lock = w.jm("runs").join(w.kept(&wish)["run"].as_str().unwrap()).join("run.lock");
            fs::write(&day, "").unwrap();
            fs::write(w.at("rec").join("go"), "").unwrap();
            let deadline = Instant::now() + Duration::from_secs(30);
            while !let_go(&lock) && Instant::now() < deadline {
                thread::yield_now();
            }
            fs::remove_file(&day).unwrap();
            let seen = w.on_page(&wish);
            (going.join().unwrap(), seen)
        });
        assert_eq!((ended.code, ended.stdout.as_str()), (1, ""), "{}", ended.stderr);
        assert_eq!((&seen["state"], &seen["stars"]), (&json!("working"), &json!([])));
        assert_eq!((w.kept(&wish), w.copies(), w.branches(&repo)), (before, vec![], vec![]));
        w.play("edits", 0);
        assert_eq!(ran(&w.start(&wish, &repo)).1["star"], json!(STAR), "and can be run again");
    };
    thread::scope(|scope| {
        let lanes: Vec<_> = (0..4).map(|_| scope.spawn(|| (0..5).for_each(round))).collect();
        lanes.into_iter().for_each(|lane| lane.join().unwrap());
    });
}

// A wish that names a run of which nothing is kept is run by the `jellymoon run` that finds it so. A run whose
// record says it was never started is taken back by the next look, and its wish runs again too.
#[test]
fn the_run_that_finds_a_wish_naming_a_run_never_started_starts_one() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let file = w.jm("pages").join(format!("{DAY}.wishes.json"));
    let mut named: serde_json::Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
    named["wishes"][0]["run"] = json!("r0123456789abcdef");
    fs::write(&file, format!("{named}\n")).unwrap();
    w.play("edits", 0);
    let (run, said) = ran(&w.start(&wish, &repo));
    assert_eq!((&said["star"], &w.kept(&wish)["run"]), (&json!(STAR), &json!(run)));

    let (w, wish) = (world(), "m1");
    let repo = w.repo("alpha");
    w.wish("alpha");
    w.play("slow", 0);
    let mut command = w.start_going(wish, &repo);
    wait_for(&w.at("rec").join("started"));
    command.kill().unwrap();
    command.wait().unwrap();
    let cut = w.kept(wish)["run"].as_str().unwrap().to_string();
    let mut record = w.record(&cut);
    record["outcome"] = json!("not_started");
    fs::write(w.jm("runs").join(&cut).join("run.json"), record.to_string()).unwrap();
    if cfg!(windows) {
        w.let_end("go"); // there the agent is not stopped, and a copy a command stands in cannot be taken back
    }
    assert_eq!((&w.on_page(wish)["state"], &w.kept(wish)["run"]), (&json!("working"), &json!(null)));
    assert_eq!((w.copies(), w.branches(&repo)), (vec![], vec![]), "the copy of a run never started is taken back");
    fs::write(w.at("rec").join("go"), "").unwrap();
}
