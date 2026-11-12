//! The `found-off-path` step: a harness that is on no `PATH`, found in the home's own place.

use std::process::Command;
use std::time::Duration;
use std::{env, fs};

use super::{World, ZONE, write};

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
    let mut command = Command::new(world.program("jellymoon"));
    command.args(world.day("page")).env_clear().env("TZ", ZONE).env("JELLYMOON_HOME", world.at("jm"));
    command.env("PATH", world.at("empty")).env("JELLYMOON_SYSTEM_BIN", "");
    command.env("HOME", world.at("user")).env("USERPROFILE", world.at("user"));
    if let Some(root) = env::var_os("SystemRoot").filter(|_| cfg!(windows)) {
        command.env("SystemRoot", root); // a Windows program needs it to start
    }
    let ran = world.wait(command, Duration::from_secs(20))?;
    if ran.code != 0 || !place.join("arg0").exists() {
        return Err(world.printed(&ran));
    }
    Ok("no harness on PATH, the one in ~/.local/bin ran".to_string())
}
