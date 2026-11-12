//! The `wish-run` step (F41): a repo the chain makes, a made-up session log of something typed in it, and
//! `jellymoon wish --run`, which finds the repo by its name in that log, starts the run and exits while it goes.
//! The page then shows the wish done with its star, and the run's command ends by itself. The repo is in the
//! system's temporary folder, not the chain's home: that home may be inside a harness worktree of this repo, and
//! a folder typed in there is read as this repo's own. The step removes the repo when it ends.

use std::path::Path;
use std::time::{Duration, Instant};
use std::{env, fs, process, thread};

use serde_json::{Value, json};

use super::copy::in_repo;
use super::{DAY, World, text, write};

pub fn wish_run(world: &World) -> Result<String, String> {
    let above = env::temp_dir().join(format!("jellymoon-smoke-{}", process::id()));
    let _ = fs::remove_dir_all(&above);
    let done = in_folder(world, &above.join("apart"));
    let _ = fs::remove_dir_all(&above);
    done
}

fn in_folder(world: &World, repo: &Path) -> Result<String, String> {
    let (rec, logs) = (world.at("apart-rec"), world.at("apart-logs"));
    for keep in [repo.join("a.txt"), rec.join("keep"), world.at("user").join("keep")] {
        write(&keep, "as it was\n")?;
    }
    let agent = world.at("agent").join(format!("claude{}", env::consts::EXE_SUFFIX));
    write(&agent.with_file_name("play"), &format!("hello\n{}\n0\n", rec.display()))?;
    let typed = json!({"type": "user", "timestamp": format!("{DAY}T16:00:00.000Z"), "cwd": text(repo), "sessionId": "s",
        "origin": {"kind": "human"}, "message": {"role": "user", "content": "look at the notes"}});
    write(&logs.join("-work-apart").join("s.jsonl"), &format!("{typed}\n"))?;
    let started = |program: &Path, args: &[&str]| {
        let mut command = in_repo(world, repo, program, args, true)?;
        command.env("JELLYMOON_CLAUDE", &agent).env("JELLYMOON_SYSTEM_BIN", "");
        command.env("CLAUDE_CONFIG_DIR", world.at("harness"));
        let ran = world.wait(command, Duration::from_secs(20))?;
        if ran.code == 0 { Ok(ran.stdout) } else { Err(world.printed(&ran)) }
    };
    let git = |args: &[&str]| {
        let who = ["-c", "user.name=Smoke Chain", "-c", "user.email=smoke@example.test"];
        started(Path::new("git"), &[&who[..], args].concat()).map(|printed| printed.trim().to_string())
    };
    let jellymoon = |args: &[&str]| {
        let printed = started(&world.program("jellymoon"), args)?;
        serde_json::from_str::<Value>(&printed).map_err(|e| format!("not one line of JSON: {e}"))
    };
    git(&["init", "-q", "-b", "main"])?;
    git(&["add", "."])?;
    git(&["commit", "-q", "-m", "the first commit"])?;

    let say = "add a file named HELLO.txt that holds the one word hello";
    let made =
        jellymoon(&["wish", "--date", DAY, "--say", say, "--project", "apart", "--run", "--logs", &text(&logs)])?;
    let (wish, run) = (made["wish"]["id"].clone(), made["run"].as_str().unwrap_or_default().to_string());
    if run.is_empty() {
        return Err(format!("no run was started: {}", made["why"]));
    }
    // The command that asked has exited. The run goes on: its star comes onto the page, and its command ends.
    let lock = world.at("jm").join("runs").join(&run).join("run.lock");
    let deadline = Instant::now() + Duration::from_secs(20);
    let (mut state, mut stars) = (Value::Null, usize::MAX);
    while Instant::now() < deadline && (state != "done" || lock.exists()) {
        thread::sleep(Duration::from_millis(50));
        let page = jellymoon(&["page", "--date", DAY, "--logs", &text(&world.at("no-logs"))])?;
        let on_page = page["wishes"].as_array().and_then(|wishes| wishes.iter().find(|on| on["id"] == wish));
        state = on_page.map_or(Value::Null, |on| on["state"].clone());
        stars = on_page.and_then(|on| on["stars"].as_array()).map_or(usize::MAX, Vec::len);
    }
    let commits = git(&["rev-list", "--count", &format!("main..jellymoon/run-{run}")])?;
    let same = git(&["status", "--porcelain"]).is_ok_and(|changed| changed.is_empty());
    let (gone, yes) = (!lock.exists(), |so: bool| if so { "yes" } else { "no" });
    let found = format!(
        "the wish's state {}, stars on the page {stars}, commits on the run's branch {commits}, the run's command ended {}, \
         checkout unchanged {}",
        state.as_str().unwrap_or("?"),
        yes(gone),
        yes(same)
    );
    if state == "done" && stars == 1 && commits == "1" && gone && same { Ok(found) } else { Err(found) }
}
