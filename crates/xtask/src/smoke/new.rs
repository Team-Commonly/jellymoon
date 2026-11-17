//! The `new` step (F65): `jellymoon project new` makes a project in the chain's temporary home, a repo with one
//! empty first commit. Nothing is typed in it and no folder is named: `jellymoon wish --run`, over logs that are
//! not there, finds it by the list and starts the run. `project forget` takes it off the list, and its folder
//! stays, in the home the chain removes.

use std::path::Path;
use std::time::{Duration, Instant};
use std::{env, thread};

use serde_json::Value;

use super::copy::in_repo;
use super::{World, text, write};

/// A day of its own: the chain's day has every made wish a page keeps by now.
const DAY: &str = "2026-03-12";

pub fn new(world: &World) -> Result<String, String> {
    let (home, rec) = (world.at("jm"), world.at("new-rec"));
    let made = home.join("projects").join("made");
    write(&rec.join("keep"), "as it was\n")?;
    let agent = world.at("agent").join(format!("claude{}", env::consts::EXE_SUFFIX));
    write(&agent.with_file_name("play"), &format!("hello\n{}\n0\n", rec.display()))?;
    let started = |folder: &Path, program: &Path, args: &[&str]| {
        let mut command = in_repo(world, folder, program, args, true)?;
        command.env("JELLYMOON_CLAUDE", &agent).env("JELLYMOON_SYSTEM_BIN", "");
        command.env("CLAUDE_CONFIG_DIR", world.at("harness"));
        let ran = world.wait(command, Duration::from_secs(20))?;
        if ran.code == 0 { Ok(ran.stdout) } else { Err(world.printed(&ran)) }
    };
    let count = |range: &str| {
        started(&made, Path::new("git"), &["rev-list", "--count", range]).map(|printed| printed.trim().to_string())
    };
    // What a command printed, and whether it named the home, and so the made folder in it.
    let jellymoon = |args: &[&str]| {
        let printed = started(&home, &world.program("jellymoon"), args)?;
        let line = serde_json::from_str::<Value>(&printed).map_err(|e| format!("not one line of JSON: {e}"))?;
        Ok::<_, String>((line, printed.contains(&text(&home))))
    };
    let (new, mut named) = jellymoon(&["project", "new", "made", "--date", DAY])?;
    let first = count("main")?;
    let say = "add a file named HELLO.txt that holds the one word hello";
    let none = text(&world.at("no-logs"));
    let (wish, path) =
        jellymoon(&["wish", "--date", DAY, "--say", say, "--project", "made", "--run", "--logs", &none])?;
    named |= path;
    let run = wish["run"].as_str().ok_or_else(|| format!("no run was started: {}", wish["why"]))?;
    // The run goes on after the command that asked: its command ends by itself, with its work on its branch.
    let lock = home.join("runs").join(run).join("run.lock");
    let deadline = Instant::now() + Duration::from_secs(20);
    let branch = format!("main..jellymoon/run-{run}");
    while Instant::now() < deadline && (lock.exists() || count(&branch)? != "1") {
        thread::sleep(Duration::from_millis(50));
    }
    let commits = count(&branch)?;
    let (forgot, _) = jellymoon(&["project", "forget", "made"])?;
    let yes = |so: bool| if so { "yes" } else { "no" };
    let found = format!(
        "made as {}, commits in the new repo {first}, commits on the run's branch {commits}, the run's command \
         ended {}, forgot {}, the folder is there after {}, a folder named {}",
        new["name"].as_str().unwrap_or("?"),
        yes(!lock.exists()),
        forgot["name"].as_str().unwrap_or("?"),
        yes(made.join(".git").is_dir()),
        yes(named)
    );
    let fine = new["name"] == "made" && first == "1" && commits == "1" && !lock.exists() && forgot["name"] == "made";
    if fine && made.join(".git").is_dir() && !named { Ok(found) } else { Err(found) }
}
