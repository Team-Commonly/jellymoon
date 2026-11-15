//! The `follow` step (F53): in the repo the `run` step made, a fifth wish whose run finishes with one commit,
//! then `jellymoon wish --after` that wish and a `jellymoon run` of the follow-up. The made-up agent is started
//! in a copy made at the first run's commit, told to fork that run's session under a new id, and does more
//! work: one commit on top of the first. A keep of the follow-up is refused while the first work waits, a toss
//! of the first while the follow-up's waits; tossed in their order, both branches go. The step prints yes or
//! no and counts, and never a wish's words. Its wishes are made on a day of their own.

use std::fs;
use std::path::Path;
use std::time::Duration;

use serde_json::Value;

use super::run::in_wished;
use super::{World, text, write};

/// The step's own day, the one after the chain's: a day holds eight made wishes, and these two would be the
/// chain's eighth and ninth.
const DAY: &str = "2026-03-11";

/// `jellymoon run`, or `run <verb>`, for a wish of the day in the step's repo.
fn line<'a>(verb: &[&'a str], wish: &'a str, repo: &'a str) -> Vec<&'a str> {
    [&["run"], verb, &["--wish", wish, "--date", DAY, "--repo", repo]].concat()
}

/// Whether the made-up agent's command line told it to fork the first run's session under the follow-up's own,
/// and the follow-up's record says its session is a fork.
fn forked(args: &[&str], was: &Value, is: &Value) -> bool {
    let after = |option: &str| args.iter().position(|arg| *arg == option).and_then(|at| args.get(at + 1)).copied();
    args.contains(&"--fork-session")
        && after("--resume").is_some_and(|of| was["session"] == of)
        && after("--session-id").is_some_and(|id| is["session"] == id && was["session"] != id)
        && is["forked"] == true
}

pub fn follow(world: &World) -> Result<String, String> {
    let (repo, rec) = (text(&world.at("wished")), world.at("follow-rec"));
    let play = |script: &str| write(&world.at("agent").join("play"), &format!("{script}\n{}\n0\n", rec.display()));
    write(&rec.join("keep"), "")?;
    let started = |program: &Path, args: &[&str]| world.wait(in_wished(world, program, args)?, Duration::from_secs(20));
    let printed = |program: &Path, args: &[&str]| {
        let ran = started(program, args)?;
        if ran.code == 0 { Ok(ran.stdout) } else { Err(world.printed(&ran)) }
    };
    let json = |printed: &str| serde_json::from_str::<Value>(printed).map_err(|e| format!("not one line of JSON: {e}"));
    let jellymoon = |args: &[&str]| json(&printed(&world.program("jellymoon"), args)?);
    // A command that is refused: whether it exited with 1 and named the wish to go first.
    let refused = |args: &[&str], first: &str| {
        started(&world.program("jellymoon"), args).map(|ran| ran.code == 1 && ran.stderr.contains(first))
    };
    let git = |args: &[&str]| printed(Path::new("git"), args).map(|printed| printed.trim().to_string());
    let record = |run: &str| {
        let kept = fs::read_to_string(world.at("jm").join("runs").join(run).join("run.json")).unwrap_or_default();
        serde_json::from_str::<Value>(&kept).unwrap_or_default()
    };
    let asked = |verb: &[&str], wish: &str| jellymoon(&line(verb, wish, &repo));

    play("edits")?;
    let made = jellymoon(&["wish", "--date", DAY, "--say", "tidy the uploader", "--project", "wished"])?;
    let first = made["wish"]["id"].as_str().ok_or("no wish was made")?;
    let ran = asked(&[], first)?;
    let one = ran["run"].as_str().unwrap_or_default();
    play("follow")?;
    let made = jellymoon(&["wish", "--date", DAY, "--say", "and its tests", "--after", first])?;
    let second = made["wish"]["id"].as_str().ok_or("no follow-up was made")?;
    let names = made["wish"]["after"] == first && made["wish"]["project"] == "wished";
    let more = asked(&[], second)?;
    let two = more["run"].as_str().unwrap_or_default();

    let (was, is) = (record(one), record(two));
    let given = (0..9).find_map(|n| fs::read_to_string(rec.join(format!("line{n}"))).ok()).unwrap_or_default();
    let forked = forked(&given.lines().collect::<Vec<_>>(), &was, &is);
    let on_work = is["start"] == was["end"] && is["follows"] == one;
    let ahead = git(&["rev-list", "--count", &format!("jellymoon/run-{one}..jellymoon/run-{two}")])?;
    let kept_later = refused(&line(&["keep"], second, &repo), &format!("keep wish {first} first"))?;
    let tossed_later = refused(&line(&["toss"], first, &repo), &format!("toss wish {second} first"))?;
    let same = git(&["status", "--porcelain"]).is_ok_and(|changed| changed.is_empty());
    let tossed = asked(&["toss"], second)?["tossed"] == true && asked(&["toss"], first)?["tossed"] == true;
    let main = git(&["rev-parse", "refs/heads/main"])?;
    let gone =
        tossed && git(&["for-each-ref", "--format=%(refname) %(objectname)"])? == format!("refs/heads/main {main}");
    let yes = |so: bool| if so { "yes" } else { "no" };
    let (outcome, again) = (ran["outcome"].as_str().unwrap_or("?"), more["outcome"].as_str().unwrap_or("?"));
    let found = format!(
        "first {outcome}, the follow-up names its wish and project {}, its run {again}, told to fork the first \
         run's session under a new id {}, its copy made at the first run's commit {}, commits on top of the first \
         {ahead}, its keep refused while the first waits {}, the first's toss refused while it waits {}, checkout \
         unchanged {}, both tossed in their order with their branches gone {}",
        yes(names),
        yes(forked),
        yes(on_work),
        yes(kept_later),
        yes(tossed_later),
        yes(same),
        yes(gone)
    );
    let went = outcome == "completed" && again == "completed" && names && forked && on_work && ahead == "1";
    if went && kept_later && tossed_later && same && gone { Ok(found) } else { Err(found) }
}
