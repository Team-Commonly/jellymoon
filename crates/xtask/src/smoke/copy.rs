//! The `copy` step (F38): a repo the chain makes itself with the real `git`, `jellymoon copy make` on it, an edit in
//! the copy, and `jellymoon copy clean` committing the edit onto the run's branch. The programs of this step alone see
//! the caller's `PATH`, since that is where git is. They read no git config of this machine: the global one is an empty
//! file in the temporary home and the system's is switched off.

use std::path::Path;
use std::process::Command;
use std::time::Duration;
use std::{env, fs};

use serde_json::Value;

use super::{World, text, write};

pub fn copy(world: &World) -> Result<String, String> {
    let (repo, config) = (world.at("repo"), world.at("gitconfig"));
    write(&repo.join("a.txt"), "as it was\n")?;
    write(&config, "")?;
    let started = |program: &Path, args: &[&str]| {
        let mut command = Command::new(program);
        command.args(args).current_dir(&repo).env_clear().env("PATH", env::var_os("PATH").unwrap_or_default());
        command
            .env("HOME", world.at("user"))
            .env("USERPROFILE", world.at("user"))
            .env("JELLYMOON_HOME", world.at("jm"));
        command.env("GIT_CONFIG_GLOBAL", &config).env("GIT_CONFIG_NOSYSTEM", "1");
        // git looks for a repo no higher than the one made here: the temporary home is inside this repo's checkout.
        command.envs(repo.parent().map(|above| ("GIT_CEILING_DIRECTORIES", above)));
        if let Some(root) = env::var_os("SystemRoot").filter(|_| cfg!(windows)) {
            command.env("SystemRoot", root); // a Windows program needs it to start
        }
        let ran = world.wait(command, Duration::from_secs(20))?;
        if ran.code == 0 { Ok(ran.stdout) } else { Err(world.printed(&ran)) }
    };
    let git = |args: &[&str]| {
        let who = ["-c", "user.name=Smoke Chain", "-c", "user.email=smoke@example.test"];
        started(Path::new("git"), &[&who[..], args].concat()).map(|printed| printed.trim().to_string())
    };
    let jellymoon = |args: &[&str]| {
        let printed = started(&world.program("jellymoon"), args)?;
        serde_json::from_str::<Value>(&printed).map_err(|e| format!("not one line of JSON ({e}): {printed}"))
    };
    git(&["init", "-q", "-b", "main"])?;
    git(&["add", "a.txt"])?;
    git(&["commit", "-q", "-m", "the first commit"])?;

    let made = jellymoon(&["copy", "make", "--repo", &text(&repo)])?;
    let (run, folder) = (made["run"].as_str().unwrap_or_default(), made["folder"].as_str().unwrap_or_default());
    let folder = Path::new(folder);
    if run.is_empty() || !folder.join("a.txt").is_file() || made["kept"] != true {
        return Err(format!("no copy was made: {made}"));
    }
    write(&folder.join("a.txt"), "edited by the run\n")?;
    let cleaned = jellymoon(&["copy", "clean", "--run", run, "--outcome", "completed"])?;
    let branch = format!("jellymoon/run-{run}");
    let commits = git(&["rev-list", "--count", &format!("main..{branch}")])?;
    let committed = git(&["show", &format!("{branch}:a.txt")])?;
    let checkout = fs::read_to_string(repo.join("a.txt")).unwrap_or_default();
    if cleaned["kept"] != false || folder.exists() || commits != "1" || committed != "edited by the run" {
        return Err(format!("the copy was not committed and removed ({commits} commits): {cleaned}"));
    }
    if checkout != "as it was\n" {
        return Err("the checkout the copy was made from was changed".to_string());
    }
    Ok("a copy made, edited and cleaned: 1 commit on the run's branch, the checkout as it was".to_string())
}
