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
    // One in the shape is a run that nothing holds: the wish needs the person.
    let mut named = made.clone();
    named["wishes"][0]["run"] = json!("r0123456789abcdef");
    fs::write(&file, format!("{named}\n")).unwrap();
    assert_eq!(page()["wishes"][0]["state"], "needs_you");
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
