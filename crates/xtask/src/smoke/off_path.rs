//! The `found-off-path` step: a harness that is on no `PATH`, found in the home's own place. And the `harness` step:
//! `jellymoon harness` says how it was found, and starts none.

use std::process::Command;
use std::time::Duration;
use std::{env, fs};

use super::{World, ZONE, alone, write};

/// As in an app opened from the Finder: `PATH` holds no harness and none is named, and `jellymoon page` finds the one
/// in the home's own place. No folder of this machine is looked in: the system's are replaced by none.
pub fn found_off_path(world: &World) -> Result<String, String> {
    let place = world.at("user").join(".local").join("bin");
    let harness = place.join(format!("claude{}", env::consts::EXE_SUFFIX));
    for file in ["reply", "exit"] {
        let left = fs::read_to_string(world.harness().with_file_name(file)).map_err(|e| format!("{file}: {e}"))?;
        write(&place.join(file), &left)?;
    }
    fs::copy(world.program("fake-claude"), &harness).map_err(|e| format!("copying fake-claude: {e}"))?;
    // A copy takes the mode the build left, and a harness the group can write to is passed over.
    #[cfg(unix)]
    fs::set_permissions(&harness, std::os::unix::fs::PermissionsExt::from_mode(0o755)).map_err(|e| e.to_string())?;
    write(&world.at("empty").join("keep"), "")?;
    let command = off_path(world, &world.day("page"));
    let ran = world.wait(command, Duration::from_secs(20))?;
    if ran.code != 0 || !place.join("arg0").exists() {
        return Err(world.printed(&ran));
    }
    Ok("no harness on PATH, the one in ~/.local/bin ran".to_string())
}

/// `jellymoon` with these arguments, where `PATH` holds no harness, none is named and no login shell is asked.
fn off_path(world: &World, args: &[String]) -> Command {
    let mut command = Command::new(world.program("jellymoon"));
    alone(command.args(args)).env("TZ", ZONE).env("JELLYMOON_HOME", world.at("jm"));
    command.env("PATH", world.at("empty")).env("JELLYMOON_SYSTEM_BIN", "");
    command.env("HOME", world.at("user")).env("USERPROFILE", world.at("user"));
    if let Some(root) = env::var_os("SystemRoot").filter(|_| cfg!(windows)) {
        command.env("SystemRoot", root); // a Windows program needs it to start
    }
    command
}

/// `jellymoon harness` says how the harness is found, in one line, and starts none: the one `found-off-path` left
/// in the home's own place is found in a fixed place, and one that is named is not looked for.
pub fn harness(world: &World) -> Result<String, String> {
    let ran_at = world.at("user").join(".local").join("bin").join("arg0");
    fs::remove_file(&ran_at).map_err(|e| format!("found-off-path left no harness that ran: {e}"))?;
    for (named, how) in [(false, "fixed_place"), (true, "named")] {
        let mut command = off_path(world, &["harness".to_string()]);
        if named {
            command.env("JELLYMOON_CLAUDE", world.harness());
        }
        let ran = world.wait(command, Duration::from_secs(20))?;
        let said: serde_json::Value = serde_json::from_str(&ran.stdout).map_err(|_| world.printed(&ran))?;
        let folder = world.at("user").to_string_lossy().into_owned();
        if ran.code != 0 || said["found"] != true || said["how"] != how || ran.stdout.contains(&folder) {
            return Err(format!("not found as {how}, or a folder was named: {}", world.printed(&ran)));
        }
    }
    if ran_at.exists() {
        return Err("`jellymoon harness` started the harness".to_string());
    }
    Ok("found in a fixed place, and as named when it is named; no harness started, no folder named".to_string())
}
