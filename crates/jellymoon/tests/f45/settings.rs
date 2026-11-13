//! What a keep never lands, what it lands only when asked to, and what it never starts: another repo's entry and
//! `.gitmodules`, the files the person's tools obey, and every hook, filter and merge driver the repo names
//! (`docs/development/keep.md`, "What is never kept" and "What a keep starts").

use std::fs;

use serde_json::json;

use super::{Entry, FORWARD, JOINED, Made, UNCHANGED, World, finished, kept, planted, printed, ran, started, world};

const REPOS: &str = "the work of run {run} adds, changes or removes another repo's entry or `.gitmodules`, and \
                     Jellymoon never keeps such work: nothing was changed";
const SETTINGS: &str = "the work of run {run} touches files the person's tools obey, which `run keep --ask` names: \
                        it is kept only with --with-settings, and nothing was changed";
const FILTER: &str = "a file of the work would be passed through a filter this repo sets for it, and Jellymoon \
                      starts no filter";

/// A run in a repo whose first commit holds `files` too.
pub fn finished_over(files: &[(&str, &str)]) -> Made {
    let w = world();
    let repo = w.repo("alpha");
    for (name, holds) in files {
        fs::create_dir_all(repo.join(name).parent().unwrap()).unwrap();
        fs::write(repo.join(name), holds).unwrap();
    }
    w.git(&repo, &["add", "."]);
    w.git(&repo, &["commit", "-q", "-m", "more files"]);
    started_in(w, repo)
}

fn started_in(w: World, repo: std::path::PathBuf) -> Made {
    let wish = w.wish("alpha");
    let (run, said) = ran(&w.start(&wish, &repo));
    assert_eq!(said["outcome"], "completed", "{said}");
    Made { w, repo, wish, run }
}

/// Fails unless the keep is refused in these words, asked or not, with settings or not, with nothing changed.
fn never(made: &Made, why: &str) {
    let (before, why) = (made.held(), why.replace("{run}", &made.run));
    for with in [&[][..], &["--with-settings"]] {
        let asked = printed(&made.keep(&[with, &["--ask"]].concat()));
        assert_eq!((&asked["fits"], &asked["kept"], &asked["why"]), (&json!(false), &json!(null), &json!(why)));
        made.refuses(&made.keep(with), &before, &why);
    }
}

#[test]
fn work_that_adds_or_changes_another_repos_entry_or_gitmodules_is_never_kept() {
    let works: [&[Entry]; 5] = [
        &[("160000", "vendored", b"")],
        &[("100644", ".gitmodules", b"[submodule \"x\"]\n\tpath = x\n\turl = ../x\n")],
        &[("100644", "deep/.gitmodules", b"anything\n")],
        &[("100644", ".GitModules", b"anything\n")],
        &[("160000", "a.txt", b"")],
    ];
    for work in works {
        let made = finished();
        made.rework(work);
        never(&made, REPOS);
        // And with commits of the person's own since.
        made.moved_on();
        never(&made, REPOS);
    }
}

#[test]
fn work_that_removes_another_repos_entry_or_gitmodules_is_never_kept() {
    let made = finished_over(&[(".gitmodules", "[submodule \"x\"]\n\tpath = x\n\turl = ../x\n")]);
    made.rework(&[("0", ".gitmodules", b""), ("100644", "n.txt", b"new\n")]);
    never(&made, REPOS);
}

#[test]
fn work_that_touches_files_the_persons_tools_obey_is_kept_only_when_that_is_asked_for() {
    let made = finished();
    let entries: [Entry; 4] = [
        ("100644", ".claude/settings.json", b"{}\n"),
        ("100644", "CLAUDE.md", b"rules\n"),
        ("100644", ".github/workflows/ci.yml", b"on: push\n"),
        ("100644", "src/plain.rs", b"fn main() {}\n"),
    ];
    let end = made.rework(&entries);
    let names = json!([".claude/settings.json", ".github/workflows/ci.yml", "CLAUDE.md"]);
    let before = made.held();
    // Asked, a keep names them and says it would not keep them.
    let asked = printed(&made.keep(&["--ask"]));
    let why = SETTINGS.replace("{run}", &made.run);
    assert_eq!(
        asked,
        json!({"run": made.run, "wish": made.wish, "kept": null, "onto": null, "fits": false, "why": why, "settings": names})
    );
    made.refuses(&made.keep(&[]), &before, SETTINGS);
    // Asked with the word, it names them and says it would.
    let asked = printed(&made.keep(&["--ask", "--with-settings"]));
    assert_eq!((&asked["fits"], &asked["settings"], &asked["kept"]), (&json!(true), &names, &json!(end)));
    assert!(before == made.held());
    // And with the word they are kept, and named in what is printed.
    let said = printed(&made.keep(&["--with-settings"]));
    let why = FORWARD.replace("{run}", &made.run);
    assert_eq!(
        said,
        json!({"run": made.run, "wish": made.wish, "kept": end, "onto": "main", "why": why, "settings": names})
    );
    assert_eq!(fs::read_to_string(made.repo.join("CLAUDE.md")).unwrap(), "rules\n");
}

#[test]
fn work_that_removes_a_file_the_persons_tools_obey_is_kept_only_when_that_is_asked_for() {
    let made = finished_over(&[("AGENTS.md", "rules\n")]);
    made.rework(&[("0", "AGENTS.md", b""), ("100644", "n.txt", b"new\n")]);
    let before = made.held();
    made.refuses(&made.keep(&[]), &before, SETTINGS);
    assert_eq!(printed(&made.keep(&["--with-settings"]))["settings"], json!(["AGENTS.md"]));
    assert!(!made.repo.join("AGENTS.md").exists());
}

#[test]
fn work_the_persons_own_rename_carries_into_a_file_their_tools_obey_is_judged_where_it_would_land() {
    // The work changes a plain file; the person has since made that file one their tools obey.
    let made = finished_over(&[("notes/rules.md", "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\n")]);
    made.rework(&[("100644", "notes/rules.md", b"one\ntwo\nthree\nfour\nfive\nsix\nseven\nand a line of the run's\n")]);
    made.who();
    made.git(&["mv", "notes/rules.md", "CLAUDE.md"]);
    made.git(&["commit", "-q", "-m", "the notes are the rules now"]);
    let before = made.held();
    let asked = printed(&made.keep(&["--ask"]));
    assert_eq!((&asked["fits"], &asked["settings"]), (&json!(false), &json!(["CLAUDE.md"])));
    made.refuses(&made.keep(&[]), &before, SETTINGS);
    let said = printed(&made.keep(&["--with-settings"]));
    assert_eq!((&said["settings"], &said["why"]), (&json!(["CLAUDE.md"]), &json!(JOINED.replace("{run}", &made.run))));
    assert!(fs::read_to_string(made.repo.join("CLAUDE.md")).unwrap().ends_with("and a line of the run's\n"));
}

#[test]
fn no_hook_of_the_repo_is_started_by_a_keep() {
    let hooks = [
        "post-merge",
        "post-checkout",
        "post-commit",
        "pre-commit",
        "pre-merge-commit",
        "prepare-commit-msg",
        "commit-msg",
        "post-index-change",
        "reference-transaction",
        "post-rewrite",
    ];
    for joined in [false, true] {
        let made = finished();
        if joined {
            made.moved_on();
        }
        let folder = made.repo.join(".git").join("hooks");
        fs::create_dir_all(&folder).unwrap();
        for hook in hooks {
            planted(&folder, hook);
        }
        assert_eq!(printed(&made.keep(&["--ask"]))["fits"], json!(true));
        let commit = if joined { None } else { Some(made.end()) };
        let ran = made.keep(&[]);
        let commit = commit.unwrap_or_else(|| made.w.commit(&made.repo, "main"));
        kept(&made, &ran, &commit, "main", if joined { JOINED } else { FORWARD });
        assert_eq!(started(&folder), Vec::<String>::new());
        // They are hooks git starts: the person's own next commit starts them.
        made.who();
        fs::write(made.repo.join("c.txt"), "one more edit\n").unwrap();
        made.git(&["commit", "-q", "-am", "the person's next commit"]);
        assert!(started(&folder).contains(&"ran-post-commit".to_string()), "{:?}", started(&folder));
    }
}

#[test]
fn a_file_the_repo_sets_a_filter_for_is_not_kept_and_no_filter_is_started() {
    // The filter is named by the work's own `.gitattributes`, by the person's committed one, by the repo's
    // `info/attributes`, and by a `.gitattributes` the person has not committed.
    for from in ["work", "tree", "info", "disk"] {
        let made =
            if from == "tree" { finished_over(&[(".gitattributes", "*.dat filter=planted\n")]) } else { finished() };
        let folder = made.w.at("tmp");
        let filter = planted(&folder, "filter");
        for step in ["clean", "smudge", "process"] {
            made.git(&["config", &format!("filter.planted.{step}"), filter.to_str().unwrap()]);
        }
        made.git(&["config", "filter.planted.required", "true"]);
        let mut entries: Vec<Entry> = vec![("100644", "big.dat", b"what the store holds\n")];
        match from {
            "work" => entries.push(("100644", ".gitattributes", b"*.dat filter=planted\n")),
            "info" => {
                fs::write(made.repo.join(".git").join("info").join("attributes"), "*.dat filter=planted\n").unwrap()
            }
            "disk" => fs::write(made.repo.join(".gitattributes"), "*.dat filter=planted\n").unwrap(),
            _ => {}
        }
        let end = made.rework(&entries);
        let (before, why) = (made.held(), format!("{FILTER}: {UNCHANGED}"));
        let asked = printed(&made.keep(&["--ask", "--with-settings"]));
        assert_eq!((&asked["fits"], &asked["why"]), (&json!(false), &json!(why.replace("{run}", &made.run))), "{from}");
        made.refuses(&made.keep(&["--with-settings"]), &before, &why);
        assert_eq!(started(&folder), Vec::<String>::new(), "{from}");
        // The control: it is a filter git starts for this file, when the person lands the commit themselves.
        made.w.git_ran(&made.repo, &["merge", "-q", "--ff-only", &end]);
        assert_eq!(started(&folder), ["ran-filter"], "{from}: git starts this filter for this file");
    }
    // A filter that touches no file of the work is no reason to stop, and is not started either.
    let made = finished();
    let folder = made.w.at("tmp");
    made.git(&["config", "filter.planted.smudge", planted(&folder, "filter").to_str().unwrap()]);
    fs::write(made.repo.join(".git").join("info").join("attributes"), "*.dat filter=planted\n").unwrap();
    kept(&made, &made.keep(&[]), &made.end(), "main", FORWARD);
    assert_eq!(started(&folder), Vec::<String>::new());
}

#[test]
fn no_merge_driver_of_the_repo_is_started_and_what_the_config_adds_to_a_merge_is_not_done() {
    // Both sides changed a file the repo names a merge driver for: git is not let start it, and without it the
    // file is one git does not merge here.
    let made = finished_over(&[("long.txt", "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\n")]);
    let end = made.rework(&[("100644", "long.txt", b"one\ntwo\nthree\nfour\nfive\nsix\nseven\neight by the run\n")]);
    made.who();
    let folder = made.w.at("tmp");
    made.git(&["config", "merge.planted.driver", planted(&folder, "driver").to_str().unwrap()]);
    fs::write(made.repo.join(".git").join("info").join("attributes"), "long.txt merge=planted\n").unwrap();
    fs::write(made.repo.join("long.txt"), "one by the person\ntwo\nthree\nfour\nfive\nsix\nseven\neight\n").unwrap();
    made.git(&["commit", "-q", "-am", "the person's edit of another line"]);
    let before = made.held();
    assert_eq!(made.keep(&[]).code, 1);
    assert!(before == made.held());
    assert_eq!(started(&folder), Vec::<String>::new());
    // The control: it is a driver git starts for this merge, when the person merges the two in the store.
    made.w.git_ran(&made.repo, &["merge-tree", "--write-tree", "main", &end]);
    assert_eq!(started(&folder), ["ran-driver"], "git starts this driver for this merge");
    fs::remove_file(folder.join("ran-driver")).unwrap();
    // A squash, a stash, no fast-forward and a signature check, set for the branch and for every merge: none is done.
    let made = finished();
    made.w.work_in_progress(&made.repo);
    for (key, value) in [
        ("branch.main.mergeOptions", "--squash --autostash --no-ff --verify-signatures"),
        ("merge.autoStash", "true"),
        ("merge.ff", "false"),
        ("merge.verifySignatures", "true"),
    ] {
        made.git(&["config", key, value]);
    }
    // A stash puts the person's unsaved files away and writes them again: their times would be the keep's.
    let times = || ["c.txt", "staged.txt"].map(|file| fs::metadata(made.repo.join(file)).unwrap().modified().unwrap());
    super::touch(&made.repo.join("c.txt"));
    super::touch(&made.repo.join("staged.txt"));
    let (end, stash, written) = (made.end(), made.git(&["rev-parse", "refs/stash"]), times());
    kept(&made, &made.keep(&[]), &end, "main", FORWARD);
    assert_eq!((made.w.commit(&made.repo, "main"), made.git(&["rev-parse", "refs/stash"])), (end, stash));
    assert!(!made.repo.join(".git").join("SQUASH_MSG").exists());
    assert_eq!(started(&folder), Vec::<String>::new());
    // Nothing was put away and taken out again: what was staged is staged, and what was not is not.
    assert_eq!(made.status(), " M c.txt\nA  staged.txt\n?? note.txt");
    assert_eq!(times(), written, "a file of the person's beside the work was written again");
}

// The one new commit a keep makes is not signed, whatever the config says of every commit.
#[test]
fn no_signer_of_the_persons_is_started_for_the_one_new_commit() {
    let made = finished();
    let (theirs, folder) = (made.moved_on(), made.w.at("tmp"));
    made.git(&["config", "commit.gpgSign", "true"]);
    made.git(&["config", "gpg.program", planted(&folder, "signer").to_str().unwrap()]);
    let ran = made.keep(&[]);
    let tip = made.w.commit(&made.repo, "main");
    kept(&made, &ran, &tip, "main", JOINED);
    assert_eq!(made.git(&["rev-parse", &format!("{tip}^")]), theirs);
    assert_eq!(started(&folder), Vec::<String>::new());
    // The control: it is a signer git starts, for the person's own next commit.
    fs::write(made.repo.join("mine.txt"), "the person's own, again\n").unwrap();
    made.w.git_ran(&made.repo, &["commit", "-q", "-am", "the person's next commit"]);
    assert_eq!(started(&folder), ["ran-signer"], "git starts this signer for a commit");
}

// A name that stands in for the run's commit (`git replace`) is not followed: the files that land are those of
// the commit the record names, which is the commit the branch is moved to.
#[test]
fn a_commit_another_stands_in_for_is_landed_as_itself() {
    let made = finished();
    let (end, start) = (made.end(), made.w.commit(&made.repo, "main"));
    // The stand-in holds the files of the run's start: followed, it would land nothing of the work.
    let other = made.git(&["commit-tree", "-p", &start, "-m", "a stand-in", &format!("{start}^{{tree}}")]);
    made.git(&["update-ref", &format!("refs/replace/{end}"), &other]);
    let said = printed(&made.keep(&[]));
    assert_eq!((&said["kept"], &said["why"]), (&json!(end), &json!(FORWARD.replace("{run}", &made.run))));
    assert_eq!(made.files(), super::worked());
    assert_eq!(made.git(&["--no-replace-objects", "rev-parse", "main^{tree}"]), made.git(&["write-tree"]));
}
