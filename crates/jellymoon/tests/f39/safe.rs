//! What a run never lets out or leaves behind: a folder read from a session log, a relative entry of the
//! environment, a run id that is no run id, a program the agent started, a finished run written down as cut off,
//! and a page waiting on a dead run's agent.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use std::{env, fs, thread};

use serde_json::{Value, json};

use super::limits::{grows, wait_for};
use super::{DAY, SAID, STAR, printed, ran, refused, world, world_in};
use crate::support::{pay_key, shows, skipped, tree};

/// In the name of every folder a session log names here. No output and no file of Jellymoon's may hold it.
const FOLDER: &str = "walrus-nickel-in-a-folder-name";

#[test]
fn a_folder_read_from_a_session_log_is_never_printed_or_kept_when_it_is_refused() {
    // In the system's own temporary folder, as the other test of folders read from the logs is.
    let w = world_in(tempfile::tempdir().unwrap());
    let under = w.at("work").join(FOLDER);
    let top = w.repo(&format!("{FOLDER}/top"));
    fs::create_dir(top.join("sub")).unwrap();
    fs::write(under.join("file"), "a file, not a folder\n").unwrap();
    for folder in ["plain", "bare"] {
        fs::create_dir(under.join(folder)).unwrap();
    }
    w.git(&under.join("bare"), &["init", "-q", "--bare"]);
    let no_repo =
        "it is not a git repo git could open: what git said of a folder read from a session log is not repeated";
    let cases = [
        ("gone", under.join("gone"), no_repo),
        ("file", under.join("file"), no_repo),
        ("plain", under.join("plain"), no_repo),
        ("sub", top.join("sub"), no_repo),
        ("bare", under.join("bare"), "it is a bare repo or a repo's own git folder: a run needs a checkout to copy"),
    ];
    for (name, folder, why) in cases {
        w.typed_in(&format!("-session-{name}"), &folder);
        let wish = w.wish(name);
        let said = w.start_found(&wish);
        let want = format!("jellymoon: the folder of that name typed in on {DAY} is not one a run can copy: {why}\n");
        assert_eq!(refused(&said), want, "{name}");
        for (path, bytes) in tree(&w.at("jm")) {
            let held = bytes.windows(FOLDER.len()).any(|window| window == FOLDER.as_bytes());
            assert!(!held && !path.to_string_lossy().contains(FOLDER), "{name}: {} holds the folder", path.display());
        }
        assert_eq!((w.kept(&wish).get("run"), w.copies(), w.starts()), (None, vec![], 0), "{name}");
    }
    // The same folders, named by the person, may be named back in git's own words.
    let typed = w.start(&w.wish("gone"), &under.join("gone"));
    assert!(refused(&typed).contains("--repo is not a git repo git could open: fatal:"), "{}", typed.stderr);
}

/// `PATH` as the world has it, with entries that are not full paths before, between and after its own.
fn mixed(path: &OsString) -> OsString {
    let own: Vec<PathBuf> = env::split_paths(path).collect();
    env::join_paths([Path::new("bin"), &own[0], Path::new("."), &own[1], Path::new("")]).unwrap()
}

#[test]
fn only_the_full_paths_of_the_environment_reach_the_agent() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let path = &w.vars.iter().find(|(name, _)| *name == "PATH").unwrap().1;
    // Each of these, read as it is by a program started in the copy, would name a folder the agent writes.
    let mut more = vec![("PATH", mixed(path))];
    more.extend(["TMPDIR", "CLAUDE_CONFIG_DIR", "HOME", "USERPROFILE"].map(|name| (name, OsString::from("harness"))));
    let args = ["run", "--wish", &wish, "--date", DAY, "--repo", repo.to_str().unwrap()];
    assert_eq!(ran(&w.run_with(&more, &args)).1["outcome"], "completed");
    assert_eq!(w.rec("var-PATH").map(OsString::from).as_ref(), Some(path), "only the two full paths, in their order");
    for name in ["TMPDIR", "CLAUDE_CONFIG_DIR", "HOME", "USERPROFILE"] {
        assert_eq!(w.rec(&format!("var-{name}")).as_deref(), Some(""), "{name}");
        assert!(!w.rec("env").unwrap().lines().any(|seen| seen == name), "{name} was passed on");
    }

    // No full path in PATH at all: nothing is started and nothing is made, even for a harness that was named.
    let wish = w.wish("alpha");
    let before = tree(w.dir.path());
    let harness = w.at("bin").join(format!("claude{}", env::consts::EXE_SUFFIX));
    let none = [("PATH", OsString::from("bin")), ("JELLYMOON_CLAUDE", harness.into())];
    let args = ["run", "--wish", &wish, "--date", DAY, "--repo", repo.to_str().unwrap()];
    let said = w.run_with(&none, &args);
    let want = "jellymoon: nothing was started: PATH names no folder by its full path, and a folder named any other \
                way is not passed on to Claude Code\n";
    assert_eq!(refused(&said), want);
    assert!(tree(w.dir.path()) == before && w.starts() == 1, "nothing was written, and no agent was started");
}

// A run's id is used in a path. One kept with a wish that is not in core's shape makes the day's made wishes
// unusable: nothing is read or written under that name.
#[test]
fn a_wish_whose_run_is_not_in_the_shape_of_a_run_id_is_not_used() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let file = w.jm("pages").join(format!("{DAY}.wishes.json"));
    let made: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
    let logs = w.at("logs");
    let page = || printed(&w.run(&["page", "--date", DAY, "--logs", logs.to_str().unwrap()]));
    for run in ["../../user", "r0123456789abcdeg", "r0123456789ABCDEF", "r0123456789abcdef0", "0123456789abcdef", ""] {
        let mut named = made.clone();
        named["wishes"][0]["run"] = json!(run);
        fs::write(&file, format!("{named}\n")).unwrap();
        let before = tree(w.dir.path());
        let said = w.start(&wish, &repo);
        assert!(refused(&said).contains("they are not the made wishes of that day"), "{run:?}: {}", said.stderr);
        assert!(tree(w.dir.path()) == before, "{run:?}: something was written");
        assert_eq!(page()["wishes"], json!([]), "{run:?}");
    }
    // One in the shape, of which nothing is kept, is a run that was never started: the wish is as it was.
    let mut named = made.clone();
    named["wishes"][0]["run"] = json!("r0123456789abcdef");
    fs::write(&file, format!("{named}\n")).unwrap();
    assert_eq!((&page()["wishes"][0]["state"], w.kept(&wish)), (&json!("working"), made["wishes"][0].clone()));
}

#[test]
fn nothing_the_agent_started_is_left_going_after_a_run_ends_however_it_ended() {
    if !cfg!(unix) {
        return skipped("this system has no process groups, and what the agent started is not stopped");
    }
    for (code, outcome) in [(0, "completed"), (3, "failed")] {
        let w = world();
        let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
        w.play("litter", code);
        assert_eq!(ran(&w.start(&wish, &repo)).1["outcome"], outcome);
        // What it left held this lock and wrote this file. Both have stopped.
        assert!(w.freed("litter-held"), "{outcome}: what the agent started is still going");
        assert!(!grows(&w.at("rec").join("litter.txt")), "{outcome}: it is still writing");
    }
}

// The harness finishes while the day's lock is busy for longer than a run waits. The work is committed and the
// copy removed, so "cut off, its copy is kept" would be false twice.
#[test]
fn a_run_that_finished_while_the_day_was_locked_is_drawn_finished_with_its_star() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    w.play("slow", 0);
    let lock = w.jm("pages").join(format!("{DAY}.ruled.lock"));
    let short = [("JELLYMOON_TEST_END_WAIT_MS", OsString::from("300"))];
    let args = ["run", "--wish", &wish, "--date", DAY, "--repo", repo.to_str().unwrap()];
    let ended = thread::scope(|scope| {
        let going = scope.spawn(|| w.run_with(&short, &args));
        wait_for(&w.at("rec").join("started"));
        fs::write(&lock, "").unwrap(); // another change to the day, which does not finish in time
        fs::write(w.at("rec").join("go"), "").unwrap();
        going.join().unwrap()
    });
    let run = w.kept(&wish)["run"].as_str().unwrap().to_string();
    let busy = format!(
        "jellymoon: run {run} has ended and its record says how; the page of {DAY} was busy, and shows it when it is \
         next drawn\n"
    );
    assert_eq!(refused(&ended), busy);
    // The run's own record has its end, the work is committed, and the wish has not been touched.
    assert_eq!((&w.record(&run)["outcome"], &w.kept(&wish)["state"]), (&json!("completed"), &json!("working")));
    let branch = format!("jellymoon/run-{run}");
    assert_eq!(
        (w.copies(), w.git(&repo, &["show", &format!("{branch}:a.txt")]).as_str()),
        (vec![], "edited by the run")
    );

    fs::remove_file(&lock).unwrap();
    for _ in 0..2 {
        let on_page = w.on_page(&wish);
        assert_eq!((&on_page["state"], &on_page["stars"]), (&json!("done"), &json!([STAR])), "one star, drawn twice");
    }
    let why = format!("run {run} finished: its work is one commit on the branch {branch}");
    let kept =
        json!({"id": wish, "said": SAID, "project": "alpha", "state": "done", "stars": [STAR], "run": run, "why": why});
    assert_eq!((w.kept(&wish), &w.record(&run)["outcome"]), (kept, &json!("completed")));
    assert!(refused(&w.start(&wish, &repo)).contains("has had its run"));
}

// The made wishes are changed by hand while the agent works, to hold a wish with no words, which is refused. The
// day is not locked, so the run does not say it was busy.
#[test]
fn a_run_that_finished_over_made_wishes_that_cannot_be_used_says_so_and_not_that_the_day_was_busy() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    w.play("slow", 0);
    let file = w.jm("pages").join(format!("{DAY}.wishes.json"));
    let ended = thread::scope(|scope| {
        let going = scope.spawn(|| w.start(&wish, &repo));
        wait_for(&w.at("rec").join("started"));
        let mut made: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
        let mut empty = made["wishes"][0].clone();
        (empty["id"], empty["said"]) = (json!("m2"), json!(""));
        made["wishes"].as_array_mut().unwrap().push(empty);
        fs::write(&file, format!("{made}\n")).unwrap();
        fs::write(w.at("rec").join("go"), "").unwrap();
        going.join().unwrap()
    });
    let run = w.kept(&wish)["run"].as_str().unwrap().to_string();
    let unused = format!(
        "jellymoon: run {run} has ended and its record says how; the made wishes of {DAY} could not be used, so the \
         wish does not show it\n"
    );
    assert_eq!(refused(&ended), unused);
    assert_eq!((&w.record(&run)["outcome"], &w.kept(&wish)["state"]), (&json!("completed"), &json!("working")));
    assert_eq!(w.copies(), Vec::<String>::new());
}

// A record says its run was never started, and the copy cannot be taken back now: the lock a copy is taken back
// behind is held. The wish goes on naming the run, so the copy, its branch and the run's folder are not left with
// nothing naming them, and the next look takes them back. It is drawn as it was, never as a run going.
#[test]
fn a_never_started_run_whose_copy_cannot_be_taken_back_is_still_named_by_its_wish_for_the_next_look() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let before = w.kept(&wish);
    w.play("slow", 0);
    let mut command = w.start_going(&wish, &repo);
    wait_for(&w.at("rec").join("started"));
    let going = w.kept(&wish);
    let run = going["run"].as_str().unwrap().to_string();
    command.kill().unwrap();
    command.wait().unwrap();
    let mut record = w.record(&run);
    record["outcome"] = json!("not_started");
    fs::write(w.jm("runs").join(&run).join("run.json"), format!("{record}\n")).unwrap();

    let held = fs::File::options().write(true).open(w.jm("runs").join("clean.lock")).unwrap();
    held.lock().unwrap();
    let (logs, short) = (w.at("logs"), [("JELLYMOON_TEST_COPY_WAIT_MS", OsString::from("100"))]);
    let page = printed(&w.run_with(&short, &["page", "--date", DAY, "--logs", logs.to_str().unwrap()]));
    assert_eq!((&page["wishes"][0]["state"], &page["wishes"][0]["run"]), (&json!("working"), &json!(null)));
    assert_eq!(w.kept(&wish), going, "the wish still names the run");
    assert_eq!((w.copies(), w.branches(&repo).len()), (vec![run.clone()], 1), "and its copy is still there");
    held.unlock().unwrap();

    w.let_end("go"); // where nothing stops the agent the run left, it ends here: a folder it stands in cannot go
    assert_eq!((&w.on_page(&wish)["run"], w.kept(&wish)), (&json!(null), before), "the next look took it back");
    assert_eq!((w.copies(), w.branches(&repo), w.jm("runs").join(&run).exists()), (vec![], vec![], false));
}

// The same record, over a repo that was moved away since: git can never take that copy back. The look removes what
// Jellymoon's home holds of the run and makes the wish as it was, so it is not refused for a run it is not drawn
// with: it runs where the repo is now.
#[test]
fn a_never_started_run_whose_repo_was_moved_away_is_cleared_by_a_look_and_its_wish_can_be_run_again() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let before = w.kept(&wish);
    w.play("slow", 0);
    let mut command = w.start_going(&wish, &repo);
    wait_for(&w.at("rec").join("started"));
    let run = w.kept(&wish)["run"].as_str().unwrap().to_string();
    command.kill().unwrap();
    command.wait().unwrap();
    let mut record = w.record(&run);
    record["outcome"] = json!("not_started");
    fs::write(w.jm("runs").join(&run).join("run.json"), format!("{record}\n")).unwrap();
    w.let_end("go"); // the agent the run left stands in the copy, and is gone before the repo is moved
    let moved = w.at("work").join("moved").join("alpha");
    fs::create_dir(moved.parent().unwrap()).unwrap();
    if let (Err(why), true) = (fs::rename(&repo, &moved), cfg!(windows)) {
        return skipped(&format!("the repo could not be moved: {why}"));
    }

    assert_eq!((&w.on_page(&wish)["run"], w.kept(&wish)), (&json!(null), before), "the wish is as it was");
    let runs = fs::read_dir(w.jm("runs")).unwrap().map(|entry| entry.unwrap().file_name().into_string().unwrap());
    let runs: Vec<String> = runs.filter(|name| name != "no-hooks" && !name.contains("lock")).collect();
    assert_eq!((w.copies(), runs), (vec![], Vec::<String>::new()), "nothing of the run is left in the home");
    w.play("edits", 0);
    assert_eq!(ran(&w.start(&wish, &moved)).1["star"], json!(STAR), "and it runs where the repo is now");
}

// A dead run's agent has nobody to report to: it is not asked and waited for, it is killed.
#[test]
fn a_page_does_not_wait_for_the_agent_a_dead_run_left() {
    if !cfg!(unix) {
        return skipped("this system has no signal to stop another command's child with, and nothing is stopped");
    }
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    w.play("deaf", 0);
    let mut command = w.start_going(&wish, &repo);
    let rec = w.at("rec");
    while !rec.join("started").exists() && !rec.join("cannot").exists() {
        thread::sleep(Duration::from_millis(10));
    }
    command.kill().unwrap();
    command.wait().unwrap();
    if let Some(why) = w.rec("cannot") {
        return skipped(&why);
    }
    assert!(!w.free("held"), "the deaf agent was to be left going");
    let from = Instant::now();
    assert_eq!(w.on_page(&wish)["state"], "needs_you");
    assert!(from.elapsed() < Duration::from_secs(3), "the page waited {:?}", from.elapsed());
    assert!(w.freed("held"), "the deaf agent is still going after the page");
}

// A run writes the day's wishes back, at its start and at its end, through the door they are read through: a wish
// a build before that door kept, cleaned once, is written as it stands still, and is not dropped.
#[test]
fn a_run_s_start_and_end_keep_a_wish_of_an_earlier_build_and_write_it_as_it_stands_still() {
    let w = world();
    let (repo, first) = (w.repo("alpha"), w.wish("alpha"));
    let file = w.jm("pages").join(format!("{DAY}.wishes.json"));
    let made =
        |id: &str, said: &str| json!({"id": id, "said": said, "project": "alpha", "state": "working", "stars": []});
    let once = format!("use [redacted]{} here", pay_key());
    let left = json!({"day": DAY, "wishes": [made(&first, &once), made("m2", SAID)]});
    fs::write(&file, left.to_string()).unwrap();
    w.play("edits", 0);
    let (_, said) = ran(&w.start("m2", &repo));
    assert_eq!((&said["outcome"], &said["star"]), (&json!("completed"), &json!(STAR)));
    let (old, new) = (w.on_page(&first), w.on_page("m2"));
    let still = "use [redacted][redacted] here";
    assert_eq!((&old["said"], &old["state"], &old["stars"]), (&json!(still), &json!("working"), &json!([])));
    assert_eq!((&new["said"], &new["state"], &new["stars"]), (&json!(SAID), &json!("done"), &json!([STAR])));
    let kept = fs::read_to_string(&file).unwrap();
    assert!(!shows(&kept, &pay_key()), "the key an earlier build kept is in the file no more");
    assert_eq!((w.kept(&first), &w.kept("m2")["state"]), (made(&first, still), &json!("done")));
    let wishes: Value = serde_json::from_str(&kept).unwrap();
    assert_eq!(wishes["wishes"].as_array().unwrap().len(), 2, "no wish was dropped");
}
