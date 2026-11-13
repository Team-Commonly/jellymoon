//! What `jellymoon run show` refuses, each in one plain line and with exit 1: a wish with no run, a run that is
//! going, failed or was cancelled, a record that cannot be read, a repo that is gone, and a branch that is not as
//! the run left it. No refusal holds a word of the agent's, a name of a file or what git said.

use std::fs;

use serde_json::json;

use super::{DAY, MARKER, Made, WORK, ended, finished, printed, ran, refused, wait_for, world};

/// A refusal of the list and of a file alike, which names the run and holds nothing of the work.
fn no(made: &Made, why: &str) {
    for more in [&[][..], &["--file", "1"]] {
        let ran = made.show(more);
        let said = refused(&ran);
        assert!(said.contains(why) && said.contains(&made.run), "{said}");
        assert!(!said.contains(WORK) && !said.contains(MARKER) && !said.contains("fatal"), "{said}");
    }
}

#[test]
fn a_wish_with_no_run_and_a_wish_that_is_not_there_are_refused() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let show = |wish: &str| w.run(&["run", "show", "--wish", wish, "--date", DAY, "--repo", repo.to_str().unwrap()]);
    assert_eq!(refused(&show(&wish)), "jellymoon: wish m1 has had no run, so there is no work to show\n");
    assert!(refused(&show("m2")).contains("has no made wish of that id"));
    let other = w.run(&["run", "show", "--wish", &wish, "--date", "2026-03-11", "--repo", repo.to_str().unwrap()]);
    assert!(refused(&other).contains("has no made wish of that id"));
    assert!(w.branches(&repo).is_empty() && w.copies().is_empty());
}

#[test]
fn a_wish_made_by_hand_to_name_another_wishs_run_is_not_shown_that_runs_work() {
    let made = finished();
    let two = made.w.wish("alpha");
    let file = made.w.jm("pages").join(format!("{DAY}.wishes.json"));
    let mut kept: serde_json::Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
    let theirs = made.w.kept(&made.wish);
    for wish in kept["wishes"].as_array_mut().unwrap().iter_mut().filter(|wish| wish["id"] == two.as_str()) {
        (wish["run"], wish["state"], wish["why"]) = (theirs["run"].clone(), json!("done"), theirs["why"].clone());
    }
    fs::write(&file, format!("{kept}\n")).unwrap();
    let said = made.w.run(&["run", "show", "--wish", &two, "--date", DAY, "--repo", made.repo.to_str().unwrap()]);
    let line =
        format!("jellymoon: wish {two} names run {}, which is another wish's run: its work is not shown\n", made.run);
    assert_eq!(refused(&said), line);
    assert_eq!(printed(&made.show(&[]))["files"].as_array().unwrap().len(), 3, "its own wish is shown it still");
}

#[test]
fn a_run_that_is_going_is_not_shown_until_it_has_finished() {
    let w = world();
    w.play("forever", 0);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let going = w.start_going(&wish, &repo);
    wait_for(&w.at("rec").join("started"));
    let run = w.kept(&wish)["run"].as_str().unwrap().to_string();
    let made = Made { w, repo, wish, run };
    no(&made, "is going: its work is shown once it has finished");
    assert_eq!(made.w.kept(&made.wish)["state"], "working", "a look at the work stops nothing");
    printed(&made.w.run(&["run", "stop", "--wish", &made.wish, "--date", DAY]));
    assert_eq!(printed(&ended(going))["outcome"], "cancelled");
    no(&made, "was cancelled by a stop: there is no work of it to show");
}

#[test]
fn a_run_that_did_not_finish_has_no_work_to_show() {
    let w = world();
    w.play("exit-3", 0);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (run, said) = ran(&w.start(&wish, &repo));
    assert_eq!(said["outcome"], "failed");
    no(&Made { w, repo, wish, run }, "did not finish: there is no work of it to show");
}

#[test]
fn a_record_that_cannot_be_read_or_is_not_cores_is_refused() {
    let made = finished();
    let file = made.w.jm("runs").join(&made.run).join("run.json");
    let kept = made.w.record(&made.run);
    let with = |key: &str, value: serde_json::Value| {
        let mut record = kept.clone();
        record[key] = value;
        fs::write(&file, format!("{record}\n")).unwrap();
    };
    with("why", json!(format!("run {} finished: its work is {MARKER}", made.run)));
    no(&made, "is not one Jellymoon wrote for this wish");
    with("wish", json!("m2"));
    no(&made, "is not one Jellymoon wrote for this wish");
    with("work", json!("burned"));
    no(&made, "cannot be read");
    fs::write(&file, "{ not a record").unwrap();
    no(&made, "cannot be read");
    fs::remove_file(&file).unwrap();
    no(&made, "cannot be read");
    with("denied", json!(0));
    assert_eq!(printed(&made.show(&[]))["files"].as_array().unwrap().len(), 3, "the record as it was is used again");
}

#[test]
fn a_repo_that_is_gone_or_is_another_repo_is_refused() {
    let made = finished();
    let other = made.w.repo("elsewhere/alpha");
    let show = |repo: &std::path::Path| {
        made.w.run(&["run", "show", "--wish", "m1", "--date", DAY, "--repo", repo.to_str().unwrap()])
    };
    // The record names the run's repo (F48), and this is another; by a record from before, which names none, it
    // is a repo that has no branch of the run's.
    let said = show(&other);
    assert!(refused(&said).contains("is not the one its record names"), "{}", said.stderr);
    let kept = made.record();
    made.write(&made.as_before());
    let said = show(&other);
    assert!(refused(&said).contains("is not in the repo"), "{}", said.stderr);
    made.write(&kept);
    // A folder inside the repo is not the repo, and neither is a folder of another name.
    fs::create_dir_all(made.repo.join("sub").join("alpha")).unwrap();
    let said = show(&made.repo.join("sub").join("alpha"));
    assert!(refused(&said).contains("could not be read from the repo"), "{}", said.stderr);
    assert!(refused(&show(&made.w.at("work"))).contains("--repo is not the folder the wish is for"));
    // By another name that same folder is found inside the repo, which git goes up to: it is not the repo's top.
    #[cfg(unix)]
    {
        let linked = made.w.at("work").join("linked");
        fs::create_dir(&linked).unwrap();
        std::os::unix::fs::symlink(made.repo.join("sub").join("alpha"), linked.join("alpha")).unwrap();
        let said = show(&linked.join("alpha"));
        assert!(refused(&said).contains("could not be read from the repo"), "{}", said.stderr);
    }
    let away = made.w.at("work").join("moved");
    fs::rename(&made.repo, &away).unwrap();
    no(&made, "could not be read from the repo");
    fs::create_dir(&made.repo).unwrap();
    no(&made, "could not be read from the repo");
}

#[test]
fn a_branch_that_is_not_as_the_run_left_it_is_refused() {
    let made = finished();
    // By a record from before F48, so that each commit below is refused for its shape: by a record that names
    // the run's end, every one of them is refused as another commit than that (`f48/shown.rs`).
    made.write(&made.as_before());
    let (branch, tip) = (made.branch(), made.w.commit(&made.repo, &made.branch()));
    let git = |args: &[&str]| made.git(args, b"", None);
    let work: [super::Entry; 1] = [("100644", "w.txt", WORK.as_bytes())];
    let message = format!("Jellymoon run {}", made.run);

    // A commit the person, or anyone, put on the branch after the run: its change is not the run's work.
    git(&["update-ref", &branch, &made.commit_of(&work, &[&tip], "tidied by hand")]);
    no(&made, "is not as the run left it");
    // The run's commit has one parent: the commit its copy was made of.
    git(&["update-ref", &branch, &made.commit_of(&work, &["main", &tip], &message)]);
    no(&made, "is not as the run left it");
    git(&["update-ref", &branch, &made.commit_of(&work, &[], &message)]);
    no(&made, "is not as the run left it");
    git(&["update-ref", &branch, &made.commit_of(&work, &["main"], &format!("{message} and more"))]);
    no(&made, "is not as the run left it");
    // The run's message is its one line and nothing below it.
    git(&["update-ref", &branch, &made.commit_of(&work, &["main"], &format!("{message}\n\nand more"))]);
    no(&made, "is not as the run left it");
    // A name for another branch is not the run's branch, wherever it leads.
    git(&["update-ref", "refs/heads/theirs", &tip]);
    git(&["symbolic-ref", &branch, "refs/heads/theirs"]);
    no(&made, "is a name for another branch by now");
    git(&["symbolic-ref", "--delete", &branch]);
    // A branch below the run's name, and a tag of its short name, are not the run's branch either.
    git(&["update-ref", &format!("{branch}/deeper"), &tip]);
    git(&["update-ref", &format!("refs/tags/jellymoon/run-{}", made.run), &tip]);
    no(&made, "is not in the repo");
    git(&["update-ref", "-d", &format!("{branch}/deeper")]);
    // A branch whose file names a tree: git writes no such branch, and a hand can.
    let named = made.repo.join(".git").join("refs").join("heads").join("jellymoon").join(format!("run-{}", made.run));
    fs::create_dir_all(named.parent().unwrap()).unwrap();
    fs::write(&named, format!("{}\n", git(&["rev-parse", "main^{tree}"]))).unwrap();
    no(&made, "is not as the run left it");
    fs::remove_file(&named).unwrap();
    git(&["update-ref", &branch, &tip]);
    assert_eq!(printed(&made.show(&[]))["files"].as_array().unwrap().len(), 3, "the run's own commit is shown again");
}
