//! The `keep` step (F45): in the repo the `run` step made, `jellymoon run keep` on the wish that step ran. Asked
//! first it says the work fits and changes nothing; then the checkout's branch is moved forward to the run's
//! commit, `HELLO.txt` is in the working tree, the run's branch is gone, and a second keep changes nothing. The
//! step prints yes or no, and nothing a harness wrote.

use std::fs;
use std::path::Path;
use std::time::Duration;

use serde_json::Value;

use super::run::in_wished;
use super::{DAY, World, text};

pub fn keep(world: &World) -> Result<String, String> {
    let repo = text(&world.at("wished"));
    let started = |program: &Path, args: &[&str]| {
        let ran = world.wait(in_wished(world, program, args)?, Duration::from_secs(20))?;
        if ran.code == 0 { Ok(ran.stdout) } else { Err(world.printed(&ran)) }
    };
    let json = |printed: &str| serde_json::from_str::<Value>(printed).map_err(|e| format!("not one line of JSON: {e}"));
    let jellymoon = |args: &[&str]| json(&started(&world.program("jellymoon"), args)?);
    let git = |args: &[&str]| started(Path::new("git"), args).map(|printed| printed.trim().to_string());
    let page = jellymoon(&["page", "--date", DAY, "--logs", &text(&world.at("no-logs"))])?;
    let ran = |on: &&Value| on["project"] == "wished" && on["state"] == "done";
    let wish = page["wishes"].as_array().and_then(|wishes| wishes.iter().find(ran));
    let wish = wish.and_then(|on| on["id"].as_str()).ok_or("the page has no finished wish of the `run` step")?;
    let keep = |more: &[&str]| {
        jellymoon(&[&["run", "keep", "--wish", wish, "--date", DAY, "--repo", &repo][..], more].concat())
    };
    let refs = || git(&["for-each-ref", "--format=%(refname) %(objectname)"]);

    let before = refs()?;
    let run = before.lines().find_map(|line| line.strip_prefix("refs/heads/jellymoon/run-")?.split_once(' '));
    let (_, end) = run.ok_or("the repo has no run's branch to keep")?;
    let asked = keep(&["--ask"])?;
    let asked = asked["fits"] == true && asked["kept"] == end && refs()? == before;
    let kept = keep(&[])?;
    let after = refs()?;
    let moved = kept["kept"] == end && kept["onto"] == "main" && after == format!("refs/heads/main {end}");
    // What the file holds is compared and never printed.
    let held = fs::read_to_string(world.at("wished").join("HELLO.txt"));
    let hello = held.is_ok_and(|held| held.trim().to_lowercase() == "hello");
    let clean = git(&["status", "--porcelain"]).is_ok_and(|changed| changed.is_empty());
    let again = keep(&[])?;
    let once = again["kept"] == end && again["why"] != kept["why"] && refs()? == after;
    let yes = |so: bool| if so { "yes" } else { "no" };
    let found = format!(
        "asked first it fits and changes nothing {}, main moved to the run's commit and its branch gone {}, HELLO.txt \
         in the checkout {}, nothing left unsaved {}, a second keep changes nothing {}",
        yes(asked),
        yes(moved),
        yes(hello),
        yes(clean),
        yes(once)
    );
    if asked && moved && hello && clean && once { Ok(found) } else { Err(found) }
}
