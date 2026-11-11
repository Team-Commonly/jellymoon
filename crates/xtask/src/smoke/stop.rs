//! The `stop` step (F40): in the repo the `run` step made, a second wish and a `jellymoon run` whose made-up agent
//! does its work and then does not end. `jellymoon run stop` ends it: both commands say cancelled, the page shows
//! the wish needing the person with no star, the copy is kept with nothing committed, and the checkout is as it was.

use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};
use std::{fs, thread};

use serde_json::Value;

use super::run::in_wished;
use super::{DAY, World, text, write};

pub fn stop(world: &World) -> Result<String, String> {
    let (repo, rec) = (world.at("wished"), world.at("stop-rec"));
    write(&rec.join("keep"), "")?;
    write(&world.at("agent").join("play"), &format!("lingers\n{}\n0\n", rec.display()))?;
    let started = |program: &Path, args: &[&str]| {
        let ran = world.wait(in_wished(world, program, args)?, Duration::from_secs(20))?;
        if ran.code == 0 { Ok(ran.stdout) } else { Err(world.printed(&ran)) }
    };
    let json = |printed: &str| serde_json::from_str::<Value>(printed).map_err(|e| format!("not one line of JSON: {e}"));
    let jellymoon = |args: &[&str]| json(&started(&world.program("jellymoon"), args)?);
    let made = jellymoon(&["wish", "--date", DAY, "--say", "tidy the notes", "--project", "wished"])?;
    let wish = made["wish"]["id"].as_str().ok_or("no wish was made")?;

    // The run goes on beside this step, printing to a file of its own, until the stop ends it.
    let said = world.at("stop-run-stdout");
    let mut going =
        in_wished(world, &world.program("jellymoon"), &["run", "--wish", wish, "--date", DAY, "--repo", &text(&repo)])?;
    going.stdout(fs::File::create(&said).map_err(|e| format!("{}: {e}", said.display()))?);
    let mut going = going.stderr(Stdio::null()).spawn().map_err(|e| format!("starting the run: {e}"))?;
    let deadline = Instant::now() + Duration::from_secs(20);
    let wait = |until: &mut dyn FnMut() -> bool| {
        while !until() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        until()
    };
    let stopped = match wait(&mut || rec.join("started").exists()) {
        true => jellymoon(&["run", "stop", "--wish", wish, "--date", DAY]),
        false => Err("the agent had not started after 20 seconds".to_string()),
    };
    if !wait(&mut || !matches!(going.try_wait(), Ok(None))) {
        let _ = going.kill();
        return Err(format!("the run was still going after 20 seconds; the stop: {stopped:?}"));
    }
    let (stopped, ran) = (stopped?, json(&fs::read_to_string(&said).unwrap_or_default())?);

    let page = jellymoon(&["page", "--date", DAY, "--logs", &text(&world.at("no-logs"))])?;
    let on_page = page["wishes"].as_array().and_then(|wishes| wishes.iter().find(|on| on["id"] == wish));
    let stars = on_page.and_then(|on| on["stars"].as_array()).map_or(usize::MAX, Vec::len);
    let run = stopped["run"].as_str().unwrap_or_default();
    let kept = world.at("jm").join("copies").join(run).join("a.txt").is_file();
    let git = |args: &[&str]| started(Path::new("git"), args).map(|printed| printed.trim().to_string());
    let ahead = git(&["rev-list", "--count", &format!("main..jellymoon/run-{run}")])?;
    let same = git(&["status", "--porcelain"]).is_ok_and(|changed| changed.is_empty());
    let yes = |so: bool| if so { "yes" } else { "no" };
    let outcome = stopped["outcome"].as_str().unwrap_or("?");
    let found = format!(
        "outcome {outcome}, stars on the page {stars}, copy kept {}, commits on its branch {ahead}, checkout unchanged {}",
        yes(kept),
        yes(same)
    );
    let needs = on_page.is_some_and(|on| on["state"] == "needs_you");
    let cancelled = outcome == "cancelled" && stopped == ran && stopped["star"].is_null();
    if cancelled && stars == 0 && needs && kept && ahead == "0" && same { Ok(found) } else { Err(found) }
}
