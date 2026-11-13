//! The `show` step (F44): in the repo the `run` step made, `jellymoon run show` on the wish that step ran. The
//! list names `HELLO.txt` as text, `--file` prints its one word, the work waits, and the checkout and the run's
//! branch are as they were. The step prints counts and yes or no, and nothing a harness wrote.

use std::path::Path;
use std::time::Duration;

use serde_json::Value;

use super::run::in_wished;
use super::{DAY, World, text};

pub fn show(world: &World) -> Result<String, String> {
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

    let before = git(&["for-each-ref", "--format=%(refname) %(objectname)"])?;
    let shown = jellymoon(&["run", "show", "--wish", wish, "--date", DAY, "--repo", &repo])?;
    let files = shown["files"].as_array().map_or(&[][..], Vec::as_slice);
    let hello = files.iter().find(|file| file["name"] == "HELLO.txt" && file["kind"] == "text");
    let n = hello.and_then(|file| file["n"].as_u64()).map_or_else(String::new, |n| n.to_string());
    // What the file holds is compared and never printed.
    let file = jellymoon(&["run", "show", "--wish", wish, "--date", DAY, "--repo", &repo, "--file", &n]);
    let word = file.is_ok_and(|file| file["text"].as_str().is_some_and(|held| held.trim().to_lowercase() == "hello"));
    let same = git(&["status", "--porcelain"]).is_ok_and(|changed| changed.is_empty());
    let refs = git(&["for-each-ref", "--format=%(refname) %(objectname)"]).is_ok_and(|after| after == before);
    let yes = |so: bool| if so { "yes" } else { "no" };
    let work = shown["work"].as_str().unwrap_or("?");
    let found = format!(
        "work {work}, files {}, HELLO.txt listed as text {}, its text as wished {}, checkout unchanged {}, refs \
         unchanged {}",
        files.len(),
        yes(hello.is_some()),
        yes(word),
        yes(same),
        yes(refs)
    );
    world.made_up(files.len() == 1, || format!("{} files in the work, not 1", files.len()))?;
    if work == "waits" && hello.is_some() && word && same && refs { Ok(found) } else { Err(found) }
}
