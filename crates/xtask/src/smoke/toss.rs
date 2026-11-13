//! The `toss` step (F46): in the repo the `run` step made, `jellymoon run toss` on the wish whose run the `stop`
//! step cancelled. The kept copy and the run's branch go, the wish is let go on the page, `main` and the checkout
//! are as they were, and a second toss changes nothing. The step prints yes or no, and nothing a harness wrote.

use std::path::Path;
use std::time::Duration;

use serde_json::Value;

use super::run::in_wished;
use super::{DAY, World, text};

pub fn toss(world: &World) -> Result<String, String> {
    let repo = text(&world.at("wished"));
    let started = |program: &Path, args: &[&str]| {
        let ran = world.wait(in_wished(world, program, args)?, Duration::from_secs(20))?;
        if ran.code == 0 { Ok(ran.stdout) } else { Err(world.printed(&ran)) }
    };
    let json = |printed: &str| serde_json::from_str::<Value>(printed).map_err(|e| format!("not one line of JSON: {e}"));
    let jellymoon = |args: &[&str]| json(&started(&world.program("jellymoon"), args)?);
    let git = |args: &[&str]| started(Path::new("git"), args).map(|printed| printed.trim().to_string());
    let page = || jellymoon(&["page", "--date", DAY, "--logs", &text(&world.at("no-logs"))]);
    let stopped = |on: &&Value| on["project"] == "wished" && on["state"] == "needs_you";
    let before = page()?;
    let wish = before["wishes"].as_array().and_then(|wishes| wishes.iter().find(stopped));
    let wish = wish.and_then(|on| on["id"].as_str()).ok_or("the page has no wish of the `stop` step")?;
    let toss = || jellymoon(&["run", "toss", "--wish", wish, "--date", DAY, "--repo", &repo]);
    let refs = || git(&["for-each-ref", "--format=%(refname) %(objectname)"]);

    let main = git(&["rev-parse", "refs/heads/main"])?;
    let had = refs()?.lines().filter(|line| line.starts_with("refs/heads/jellymoon/run-")).count();
    let tossed = toss()?;
    let run = tossed["run"].as_str().unwrap_or_default();
    let did = tossed["tossed"] == true && tossed["commit"] == main.as_str();
    let gone = !world.at("jm").join("copies").join(run).exists() && refs()? == format!("refs/heads/main {main}");
    let let_go = page()?["wishes"]
        .as_array()
        .is_some_and(|wishes| wishes.iter().any(|on| on["id"] == wish && on["state"] == "let_go"));
    let same = git(&["status", "--porcelain"]).is_ok_and(|changed| changed.is_empty());
    let again = toss()?;
    let once =
        again["tossed"] == false && again["why"] != tossed["why"] && refs()? == format!("refs/heads/main {main}");
    let yes = |so: bool| if so { "yes" } else { "no" };
    let found = format!(
        "run branches before {had}, tossed at the commit the run started from {}, its copy and its branch gone and \
         main where it was {}, the wish let go on the page {}, checkout unchanged {}, a second toss changes nothing {}",
        yes(did),
        yes(gone),
        yes(let_go),
        yes(same),
        yes(once)
    );
    if had == 1 && did && gone && let_go && same && once { Ok(found) } else { Err(found) }
}
