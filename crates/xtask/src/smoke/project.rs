//! The `project` step (F62): a repo the chain makes in the system's temporary folder, listed with `jellymoon
//! project add`. Nothing is typed in it: `jellymoon wish --run`, over logs that are not there, finds the repo by
//! the list and starts the run. `project list` names the project and no folder, and `project forget` takes it
//! off. The step removes the repo when it ends.

use std::path::Path;
use std::time::{Duration, Instant};
use std::{env, fs, process, thread};

use serde_json::Value;

use super::copy::in_repo;
use super::{DAY, World, text, write};

pub fn project(world: &World) -> Result<String, String> {
    let above = env::temp_dir().join(format!("jellymoon-smoke-list-{}", process::id()));
    let _ = fs::remove_dir_all(&above);
    let done = in_folder(world, &above.join("listed"));
    let _ = fs::remove_dir_all(&above);
    done
}

fn in_folder(world: &World, repo: &Path) -> Result<String, String> {
    let rec = world.at("project-rec");
    write(&repo.join("a.txt"), "as it was\n")?;
    write(&rec.join("keep"), "as it was\n")?;
    let agent = world.at("agent").join(format!("claude{}", env::consts::EXE_SUFFIX));
    write(&agent.with_file_name("play"), &format!("hello\n{}\n0\n", rec.display()))?;
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
    // What a command printed, and whether it named the repo's folder, or the folder above it.
    let jellymoon = |args: &[&str]| {
        let printed = started(&world.program("jellymoon"), args)?;
        let named = repo.parent().is_some_and(|above| printed.contains(&text(above)));
        let line = serde_json::from_str::<Value>(&printed).map_err(|e| format!("not one line of JSON: {e}"))?;
        Ok::<_, String>((line, named))
    };
    git(&["init", "-q", "-b", "main"])?;
    git(&["add", "."])?;
    git(&["commit", "-q", "-m", "the first commit"])?;
    let (added, mut named) = jellymoon(&["project", "add", "--repo", &text(repo), "--date", DAY])?;
    let listed = |named: &mut bool| {
        let (list, path) = jellymoon(&["project", "list"])?;
        *named |= path;
        Ok::<_, String>(list["projects"].as_array().is_some_and(|all| all.iter().any(|one| one["name"] == "listed")))
    };
    let on_list = listed(&mut named)?;
    let say = "add a file named HELLO.txt that holds the one word hello";
    let none = text(&world.at("no-logs"));
    let (made, _) = jellymoon(&["wish", "--date", DAY, "--say", say, "--project", "listed", "--run", "--logs", &none])?;
    let run = made["run"].as_str().ok_or_else(|| format!("no run was started: {}", made["why"]))?;
    // The run goes on after the command that asked: its command ends by itself, with its work on its branch.
    let lock = world.at("jm").join("runs").join(run).join("run.lock");
    let deadline = Instant::now() + Duration::from_secs(20);
    let branch = format!("main..jellymoon/run-{run}");
    while Instant::now() < deadline && (lock.exists() || git(&["rev-list", "--count", &branch])? != "1") {
        thread::sleep(Duration::from_millis(50));
    }
    let commits = git(&["rev-list", "--count", &branch])?;
    let (forgot, _) = jellymoon(&["project", "forget", "listed"])?;
    let left = listed(&mut named)?;
    let yes = |so: bool| if so { "yes" } else { "no" };
    let found = format!(
        "added as {}, on the list {}, commits on the run's branch {commits}, the run's command ended {}, forgot {}, \
         on the list after {}, a folder named {}",
        added["name"].as_str().unwrap_or("?"),
        yes(on_list),
        yes(!lock.exists()),
        forgot["name"].as_str().unwrap_or("?"),
        yes(left),
        yes(named)
    );
    let fine = added["name"] == "listed" && on_list && commits == "1" && !lock.exists() && forgot["name"] == "listed";
    if fine && !left && !named { Ok(found) } else { Err(found) }
}
