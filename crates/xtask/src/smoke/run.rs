//! The `run` step (F39): a repo the chain makes, a wish made for it, `jellymoon run` on that wish, and the wish's star
//! on the page. By default the harness is the made-up `fake-agent`, named with `JELLYMOON_CLAUDE`, so no harness on
//! the caller's `PATH` is ever looked for. With `--real` it is the person's own Claude Code, on one harmless fixed
//! wish; the step then prints the outcome, whether there is a star and how many things were denied, and no more.

use std::path::Path;
use std::time::Duration;
use std::{env, fs};

use serde_json::Value;

use super::copy::in_repo;
use super::{DAY, World, text, write};

/// The wish: harmless in any repo, and this repo is the chain's own.
const WISH: &str = "add a file named HELLO.txt that holds the one word hello";
/// With `--real`: the run is stopped after four minutes, and the step after five.
const REAL_LIMIT_MS: &str = "240000";

pub fn run(world: &World) -> Result<String, String> {
    let (repo, agent) = (world.at("wished"), world.at("agent").join(format!("claude{}", env::consts::EXE_SUFFIX)));
    for file in ["a.txt", "b.txt"] {
        write(&repo.join(file), "as it was\n")?;
    }
    write(&world.at("no-logs").join("keep"), "")?;
    if !world.real {
        let rec = world.at("agent-rec");
        write(&rec.join("keep"), "")?;
        write(&agent.with_file_name("play"), &format!("edits\n{}\n0\n", rec.display()))?;
        fs::copy(world.program("fake-agent"), &agent).map_err(|e| format!("copying fake-agent: {e}"))?;
    }
    let started = |program: &Path, args: &[&str], seconds: u64| {
        let mut command = in_repo(world, &repo, program, args, !world.real)?;
        if world.real {
            command.env("JELLYMOON_TEST_RUN_LIMIT_MS", REAL_LIMIT_MS);
        } else {
            command.env("JELLYMOON_CLAUDE", &agent).env("JELLYMOON_SYSTEM_BIN", "");
            command.env("CLAUDE_CONFIG_DIR", world.at("harness"));
        }
        let ran = world.wait(command, Duration::from_secs(seconds))?;
        if ran.code == 0 { Ok(ran.stdout) } else { Err(world.printed(&ran)) }
    };
    let git = |args: &[&str]| {
        let who = ["-c", "user.name=Smoke Chain", "-c", "user.email=smoke@example.test"];
        started(Path::new("git"), &[&who[..], args].concat(), 20).map(|printed| printed.trim().to_string())
    };
    let jellymoon = |args: &[&str], seconds: u64| {
        let printed = started(&world.program("jellymoon"), args, seconds)?;
        serde_json::from_str::<Value>(&printed).map_err(|e| format!("not one line of JSON: {e}"))
    };
    git(&["init", "-q", "-b", "main"])?;
    git(&["add", "."])?;
    git(&["commit", "-q", "-m", "the first commit"])?;

    let made = jellymoon(&["wish", "--date", DAY, "--say", WISH, "--project", "wished"], 20)?;
    let wish = made["wish"]["id"].as_str().ok_or("no wish was made")?;
    let ran =
        jellymoon(&["run", "--wish", wish, "--date", DAY, "--repo", &text(&repo)], if world.real { 300 } else { 20 })?;
    let page = jellymoon(&["page", "--date", DAY, "--logs", &text(&world.at("no-logs"))], 20)?;
    let on_page = page["wishes"].as_array().and_then(|wishes| wishes.iter().find(|on| on["id"] == wish));
    let stars = on_page.and_then(|on| on["stars"].as_array()).map_or(0, Vec::len);
    let (outcome, denied) = (ran["outcome"].as_str().unwrap_or("?"), ran["denied"].as_u64().unwrap_or(0));
    let found = format!("outcome {outcome}, stars on the page {stars}, denied {denied}");
    let done = on_page.is_some_and(|on| on["state"] == "done");
    if outcome != "completed" || stars != 1 || !done || ran["star"].is_null() {
        return Err(found);
    }
    let branch = format!("jellymoon/run-{}", ran["run"].as_str().unwrap_or_default());
    let commits = git(&["rev-list", "--count", &format!("main..{branch}")])?;
    world.made_up(commits == "1", || format!("{commits} commits on the run's branch, not 1"))?;
    Ok(found)
}
