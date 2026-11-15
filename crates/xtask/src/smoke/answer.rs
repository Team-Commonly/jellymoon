//! The `answer` step (F52): in the repo the `run` step made, a fourth wish and a `jellymoon run` whose made-up
//! agent ends with a question, then `jellymoon run answer` with the answer on its input. The made-up agent is
//! started again, told to go on with the session, is given the answer as its whole input, and does the work:
//! the same run finishes with one commit, the page shows the wish done with one star and no question, and a
//! toss removes the branch. The step prints yes or no, and never the question or the answer.

use std::fs::{self, File};
use std::path::Path;
use std::time::Duration;

use serde_json::Value;

use super::run::in_wished;
use super::{DAY, World, text, write};

/// The answer: words of the chain's own.
const ANSWER: &str = "the old one";

pub fn answer(world: &World) -> Result<String, String> {
    let (repo, rec, given) = (text(&world.at("wished")), world.at("answer-rec"), world.at("answer"));
    let play = |script: &str| write(&world.at("agent").join("play"), &format!("{script}\n{}\n0\n", rec.display()));
    write(&rec.join("keep"), "")?;
    write(&given, &format!("{ANSWER}\n"))?;
    play("asks")?;
    let started = |program: &Path, args: &[&str], fed: bool| {
        let mut command = in_wished(world, program, args)?;
        if fed {
            command.stdin(File::open(&given).map_err(|e| format!("{}: {e}", given.display()))?);
        }
        let ran = world.wait(command, Duration::from_secs(20))?;
        if ran.code == 0 { Ok(ran.stdout) } else { Err(world.printed(&ran)) }
    };
    let json = |printed: &str| serde_json::from_str::<Value>(printed).map_err(|e| format!("not one line of JSON: {e}"));
    let jellymoon = |args: &[&str], fed: bool| json(&started(&world.program("jellymoon"), args, fed)?);
    let git = |args: &[&str]| started(Path::new("git"), args, false).map(|printed| printed.trim().to_string());
    let made = jellymoon(&["wish", "--date", DAY, "--say", "pick the uploader again", "--project", "wished"], false)?;
    let wish = made["wish"]["id"].as_str().ok_or("no wish was made")?;
    let asked = jellymoon(&["run", "--wish", wish, "--date", DAY, "--repo", &repo], false)?;
    let run = asked["run"].as_str().unwrap_or_default();

    play("edits")?;
    let ran = jellymoon(&["run", "answer", "--wish", wish, "--date", DAY], true)?;
    let read = |name: &str| fs::read_to_string(rec.join(name)).unwrap_or_default();
    let args: Vec<String> = (0..).map_while(|n| fs::read_to_string(rec.join(format!("arg{n}"))).ok()).collect();
    let resumed = args.iter().any(|arg| arg == "--resume") && !args.iter().any(|arg| arg.contains(ANSWER));
    let fed = read("stdin") == format!("{ANSWER}\n");
    let page = jellymoon(&["page", "--date", DAY, "--logs", &text(&world.at("no-logs"))], false)?;
    let on_page = page["wishes"].as_array().and_then(|wishes| wishes.iter().find(|on| on["id"] == wish));
    let done = on_page.is_some_and(|on| on["state"] == "done" && on.get("question").is_none());
    let stars = on_page.and_then(|on| on["stars"].as_array()).map_or(usize::MAX, Vec::len);
    let ahead = git(&["rev-list", "--count", &format!("main..jellymoon/run-{run}")])?;
    let same = git(&["status", "--porcelain"]).is_ok_and(|changed| changed.is_empty());
    let tossed = jellymoon(&["run", "toss", "--wish", wish, "--date", DAY, "--repo", &repo], false)?;
    let main = git(&["rev-parse", "refs/heads/main"])?;
    let gone = tossed["tossed"] == true
        && !world.at("jm").join("copies").join(run).exists()
        && git(&["for-each-ref", "--format=%(refname) %(objectname)"])? == format!("refs/heads/main {main}");
    let yes = |so: bool| if so { "yes" } else { "no" };
    let (first, outcome) = (asked["outcome"].as_str().unwrap_or("?"), ran["outcome"].as_str().unwrap_or("?"));
    let found = format!(
        "first {first}, answered {outcome} as the same run {}, told to resume with the answer on no command line {}, \
         the answer its whole input {}, done on the page with no question {}, stars on the page {stars}, commits on \
         its branch {ahead}, checkout unchanged {}, tossed with its branch gone {}",
        yes(ran["run"] == asked["run"]),
        yes(resumed),
        yes(fed),
        yes(done),
        yes(same),
        yes(gone)
    );
    let went = first == "asks" && outcome == "completed" && ran["run"] == asked["run"] && resumed && fed;
    if went && done && stars == 1 && ahead == "1" && same && gone { Ok(found) } else { Err(found) }
}
