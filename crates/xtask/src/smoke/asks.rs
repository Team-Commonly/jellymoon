//! The `asks` step (F51): in the repo the `run` step made, a third wish and a `jellymoon run` whose made-up agent
//! edits a file and then ends with a question. The run says asks, the page shows the wish needing the person with
//! the same question and no star, the copy is kept with nothing committed, and a toss removes it. The step prints
//! yes or no, and never the question: it is a harness's words.

use std::path::Path;
use std::time::Duration;

use serde_json::Value;

use super::run::in_wished;
use super::{DAY, World, text, write};

pub fn asks(world: &World) -> Result<String, String> {
    let (repo, rec) = (text(&world.at("wished")), world.at("asks-rec"));
    write(&rec.join("keep"), "")?;
    write(&world.at("agent").join("play"), &format!("asks\n{}\n1\n", rec.display()))?;
    let started = |program: &Path, args: &[&str]| {
        let ran = world.wait(in_wished(world, program, args)?, Duration::from_secs(20))?;
        if ran.code == 0 { Ok(ran.stdout) } else { Err(world.printed(&ran)) }
    };
    let json = |printed: &str| serde_json::from_str::<Value>(printed).map_err(|e| format!("not one line of JSON: {e}"));
    let jellymoon = |args: &[&str]| json(&started(&world.program("jellymoon"), args)?);
    let git = |args: &[&str]| started(Path::new("git"), args).map(|printed| printed.trim().to_string());
    let made = jellymoon(&["wish", "--date", DAY, "--say", "pick the uploader", "--project", "wished"])?;
    let wish = made["wish"]["id"].as_str().ok_or("no wish was made")?;
    let ran = jellymoon(&["run", "--wish", wish, "--date", DAY, "--repo", &repo])?;
    let run = ran["run"].as_str().unwrap_or_default();

    let page = jellymoon(&["page", "--date", DAY, "--logs", &text(&world.at("no-logs"))])?;
    let on_page = page["wishes"].as_array().and_then(|wishes| wishes.iter().find(|on| on["id"] == wish));
    let asked = ran["question"].as_str().is_some_and(|question| !question.is_empty());
    let shown = asked && on_page.is_some_and(|on| on["question"] == ran["question"] && on["state"] == "needs_you");
    let stars = on_page.and_then(|on| on["stars"].as_array()).map_or(usize::MAX, Vec::len);
    let kept = world.at("jm").join("copies").join(run).join("n.txt").is_file();
    let ahead = git(&["rev-list", "--count", &format!("main..jellymoon/run-{run}")])?;
    let same = git(&["status", "--porcelain"]).is_ok_and(|changed| changed.is_empty());
    let tossed = jellymoon(&["run", "toss", "--wish", wish, "--date", DAY, "--repo", &repo])?;
    let main = git(&["rev-parse", "refs/heads/main"])?;
    let gone = tossed["tossed"] == true
        && !world.at("jm").join("copies").join(run).exists()
        && git(&["for-each-ref", "--format=%(refname) %(objectname)"])? == format!("refs/heads/main {main}");
    let yes = |so: bool| if so { "yes" } else { "no" };
    let outcome = ran["outcome"].as_str().unwrap_or("?");
    let found = format!(
        "outcome {outcome}, its question on the page {}, stars on the page {stars}, copy kept with the agent's new \
         file {}, commits on its branch {ahead}, checkout unchanged {}, tossed with its copy and its branch gone {}",
        yes(shown),
        yes(kept),
        yes(same),
        yes(gone)
    );
    let fine = outcome == "asks" && ran["star"].is_null() && shown && stars == 0 && kept && ahead == "0" && same;
    if fine && gone { Ok(found) } else { Err(found) }
}
