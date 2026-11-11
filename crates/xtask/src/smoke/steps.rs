//! The smoke chain. A feature that adds a command adds its step here: one function, and one line in `STEPS` where it
//! belongs in the order. A step returns counts and states for its line, and its checks of what the made-up day holds
//! go through `World::made_up`, so `--real` runs the same step without them and never prints the person's words.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use std::{env, fs};

use serde_json::{Value, json};

use super::{DAY, Only, Step, World, ZONE, text, write};

pub const STEPS: &[Step] = &[
    Step { name: "logs", only: Only::Synthetic, run: logs },
    Step { name: "day", only: Only::Any, run: day },
    Step { name: "page", only: Only::Any, run: page },
    Step { name: "found-off-path", only: Only::Synthetic, run: found_off_path },
    Step { name: "layout-sticker", only: Only::Any, run: |world| layout(world, "sticker") },
    Step { name: "layout-plain", only: Only::Any, run: |world| layout(world, "plain") },
    // Not with --real: these keep a page of the day, and a second reading call would spend the person's plan.
    Step { name: "today", only: Only::Synthetic, run: today },
    Step { name: "today-again", only: Only::Synthetic, run: today_again },
    Step { name: "broken-page", only: Only::Any, run: broken_page },
    Step { name: "empty-day", only: Only::Synthetic, run: empty_day },
    Step { name: "no-logs", only: Only::Synthetic, run: no_logs },
    // Delivery, never with --real: these would write the person's own harness settings and tell their sessions.
    Step { name: "hook-install", only: Only::Synthetic, run: hook_install },
    Step { name: "rule", only: Only::Synthetic, run: rule },
    Step { name: "hook-told", only: Only::Synthetic, run: hook_told },
    Step { name: "hook-again", only: Only::Synthetic, run: hook_again },
    // The join (F35): a tap on a clash is a ruling, the session hears it, and the page shows the clash settled.
    Step { name: "tap", only: Only::Synthetic, run: tap },
    Step { name: "tap-told", only: Only::Synthetic, run: tap_told },
    Step { name: "today-settled", only: Only::Synthetic, run: today_settled },
    Step { name: "uninstall", only: Only::Synthetic, run: uninstall },
    Step { name: "window", only: Only::Window, run: window },
];

/// A made-up day in two projects: a wish and the change of mind that clashes with it, and a wish that finished. Then
/// the made-up harness, with the reading a model could have returned for that day.
fn logs(world: &World) -> Result<String, String> {
    let said = |at: &str, project: &str, text: &str| {
        let (at, cwd) = (format!("{DAY}T{at}:00.000Z"), format!("/work/{project}"));
        let message = json!({"role": "user", "content": text});
        json!({"type": "user", "timestamp": at, "cwd": cwd, "sessionId": "s", "origin": {"kind": "human"}, "message": message})
            .to_string()
    };
    let alpha = [
        said("16:00", "alpha", "add a retry to the uploader"),
        said("17:00", "alpha", "actually never retry uploads, fail fast"),
    ];
    let beta = said("18:00", "beta", "make the login page load faster");
    write(&world.at("logs").join("-work-alpha").join("s1.jsonl"), &(alpha.join("\n") + "\n"))?;
    write(&world.at("logs").join("-work-beta").join("s2.jsonl"), &(beta + "\n"))?;
    let wish = |id: &str, said: &str, project: &str, state: &str, stars: &[&str]| json!({"id": id, "label": format!("Wish {id}"), "said": said, "project": project, "state": state, "stars": stars});
    let reading = json!({
        "wishes": [
            wish("a", "add a retry to the uploader", "alpha", "working", &[]),
            wish("b", "never retry uploads, fail fast", "alpha", "working", &[]),
            wish("c", "make the login page load faster", "beta", "done", &["loads in one second"]),
        ],
        "clashes": [{"a": "a", "b": "b", "quote_a": "add a retry", "quote_b": "never retry uploads", "settled": false}],
    });
    let harness = world.harness();
    write(
        &harness.with_file_name("reply"),
        &json!({"type": "result", "is_error": false, "result": reading.to_string()}).to_string(),
    )?;
    write(&harness.with_file_name("exit"), "0")?;
    fs::copy(world.program("fake-claude"), &harness).map_err(|e| format!("copying fake-claude: {e}"))?;
    Ok("2 projects, 3 things typed, a made-up harness".to_string())
}

/// `jellymoon day` reads the logs and counts the turns.
fn day(world: &World) -> Result<String, String> {
    let day = world.json("jellymoon", &world.day("day"))?;
    let turns = day["turns"].as_array().ok_or("the day has no turns list")?;
    let projects: BTreeSet<&str> = turns.iter().filter_map(|turn| turn["project"].as_str()).collect();
    let found = format!("{} turns, {} projects", turns.len(), projects.len());
    world.made_up((turns.len(), projects.len()) == (3, 2), || format!("{found}, not 3 turns in 2 projects"))?;
    Ok(found)
}

/// `jellymoon page` makes one reading call and prints page data, which the later steps draw from `page.json`.
fn page(world: &World) -> Result<String, String> {
    let page = world.json("jellymoon", &world.day("page"))?;
    write(&world.at("page.json"), &page.to_string())?;
    let (wishes, clashes) = held(&page)?;
    let mut states: BTreeMap<&str, usize> = BTreeMap::new();
    for wish in wishes {
        *states.entry(wish["state"].as_str().unwrap_or("?")).or_default() += 1;
    }
    let states: Vec<String> = states.iter().map(|(state, count)| format!("{state} {count}")).collect();
    let found = format!("wishes {} ({}), clashes {}", wishes.len(), states.join(", "), clashes.len());
    let wanted = "wishes 3 (done 1, working 2), clashes 1";
    world.made_up(found == wanted, || format!("{found}, not {wanted}"))?;
    Ok(found)
}

/// As in an app opened from the Finder: `PATH` holds no harness and none is named, and `jellymoon page` finds the one
/// in the home's own place. No folder of this machine is looked in: the system's are replaced by none.
fn found_off_path(world: &World) -> Result<String, String> {
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

/// The wishes and the clashes of page data.
fn held(page: &Value) -> Result<(&Vec<Value>, &Vec<Value>), String> {
    let list = |name: &str| page[name].as_array().ok_or_else(|| format!("the page has no {name} list"));
    Ok((list("wishes")?, list("clashes")?))
}

/// `jellymoon-app layout` places every wish of the page as a character and every clash as a choice note.
fn layout(world: &World, theme: &str) -> Result<String, String> {
    placed(world, "page.json", theme)
}

fn placed(world: &World, file: &str, theme: &str) -> Result<String, String> {
    let args = ["layout", "--page", &text(&world.at(file)), "--theme", theme].map(str::to_string);
    Ok(drawn(world, &args, &world.at(file))?.0)
}

/// Runs `jellymoon-app` and checks what it placed against the page data in `file`, which the run may have written.
/// Returns the step's words and the pieces.
fn drawn(world: &World, args: &[String], file: &Path) -> Result<(String, Vec<Value>), String> {
    let pieces = world.lines("jellymoon-app", args)?;
    let page = fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))?;
    let page: Value = serde_json::from_str(&page).map_err(|e| format!("{}: {e}", file.display()))?;
    let (wishes, clashes) = held(&page)?;
    let ids = |kind: &str, start: &str| -> BTreeSet<&str> {
        let of_kind = pieces.iter().filter(|piece| piece["kind"] == kind).filter_map(|piece| piece["id"].as_str());
        of_kind.filter(|id| id.starts_with(start)).collect()
    };
    let (characters, notes) = (ids("character", ""), ids("note", "clash-"));
    let found = format!("{} pieces, characters {}, clash notes {}", pieces.len(), characters.len(), notes.len());
    let wishes: BTreeSet<&str> = wishes.iter().filter_map(|wish| wish["id"].as_str()).collect();
    if characters != wishes || notes.len() != clashes.len() {
        return Err(format!("{found}, for a page of {} wishes and {} clashes", wishes.len(), clashes.len()));
    }
    Ok((found, pieces))
}

/// Where the app keeps the day's page, and what the made-up harness leaves when it is called.
fn kept_and_seen(world: &World) -> (PathBuf, PathBuf) {
    (world.at("jm").join("pages").join(format!("{DAY}.json")), world.harness().with_file_name("stdin"))
}

/// `jellymoon-app layout` with no `--page` reads the day itself, through the harness, draws it and keeps the page.
fn today(world: &World) -> Result<String, String> {
    let (kept, seen) = kept_and_seen(world);
    let _ = fs::remove_file(&seen);
    let (found, pieces) = drawn(world, &world.day("layout"), &kept)?;
    if !seen.exists() {
        return Err("a page came out with no reading call".to_string());
    }
    write(&world.at("today.json"), &Value::Array(pieces).to_string())?;
    Ok(format!("read with one call and kept; {found}"))
}

/// Opened again, the app draws the kept page: the same pieces, and the harness is not called.
fn today_again(world: &World) -> Result<String, String> {
    let (kept, seen) = kept_and_seen(world);
    fs::remove_file(&seen).map_err(|e| format!("what the harness saw of the today step: {e}"))?;
    let (_, pieces) = drawn(world, &world.day("layout"), &kept)?;
    let first = fs::read_to_string(world.at("today.json")).map_err(|e| format!("today.json: {e}"))?;
    if serde_json::from_str::<Value>(&first).ok() != Some(Value::Array(pieces)) || seen.exists() {
        return Err(format!("not the kept page with no call (a call was made: {})", seen.exists()));
    }
    Ok("the same pieces from the kept page, and no call".to_string())
}

/// A file that is not page data is refused in one line, with exit 1 and nothing on stdout.
fn broken_page(world: &World) -> Result<String, String> {
    let broken = world.at("broken.json");
    write(&broken, "[\"not page data\"]")?;
    let ran = world.run("jellymoon-app", &["layout", "--page", &text(&broken)].map(str::to_string))?;
    let refusal = ran.stderr.trim_end();
    let one_line = refusal.lines().count() == 1 && refusal.starts_with("jellymoon-app: the file is not page data");
    if ran.code != 1 || !ran.stdout.is_empty() || !one_line {
        return Err(world.printed(&ran));
    }
    Ok("refused in one line, exit 1".to_string())
}

/// A person who has not used Claude Code yet: no logs folder is one plain line, never the system's own error.
fn no_logs(world: &World) -> Result<String, String> {
    let args = ["day", "--date", DAY, "--logs", &text(&world.at("no-logs-here"))].map(str::to_string);
    let ran = world.run("jellymoon", &args)?;
    let plain = ran.stderr.starts_with("jellymoon: no Claude Code session logs were found: there is no folder ");
    if ran.code != 1 || !ran.stdout.is_empty() || !plain || ran.stderr.lines().count() != 1 {
        return Err(world.printed(&ran));
    }
    Ok("no session logs said in one line, exit 1".to_string())
}

/// A day nobody typed in is an empty page, made with no reading call, and the app draws it.
fn empty_day(world: &World) -> Result<String, String> {
    let seen = world.harness().with_file_name("stdin");
    let _ = fs::remove_file(&seen); // the last call was the today step's, and the step after it made none
    let args = ["page", "--date", "2026-03-11", "--logs", &text(&world.at("logs"))].map(str::to_string);
    let page = world.json("jellymoon", &args)?;
    let (wishes, clashes) = held(&page)?;
    if (wishes.len(), clashes.len(), &page["read"]["turns"]) != (0, 0, &json!(0)) || seen.exists() {
        return Err(format!("not an empty page with no call (a call was made: {}): {page}", seen.exists()));
    }
    write(&world.at("empty.json"), &page.to_string())?;
    Ok(format!("an empty page and no call; {}", placed(world, "empty.json", "sticker")?))
}

/// The made-up harness settings, and what they hold before the hook goes in. They are in the temporary home, where
/// `CLAUDE_CONFIG_DIR` points every made-up run.
const SETTINGS: &str = "{\n  // the person's own\n  \"model\": \"opus\"\n}\n";
const STANDS: &str = "never retry uploads, fail fast";

fn settings(world: &World) -> std::path::PathBuf {
    world.at("harness").join("settings.json")
}

/// `jellymoon hook install` puts its entry into the settings, which stay a JSON object around it.
fn hook_install(world: &World) -> Result<String, String> {
    write(&settings(world), SETTINGS)?;
    let args = ["hook", "install", "--settings", &text(&settings(world))].map(str::to_string);
    let state = world.json("jellymoon", &args)?;
    let now = fs::read_to_string(settings(world)).map_err(|e| format!("the settings: {e}"))?;
    let entry =
        now.contains("\"PostToolUse\":[{\"hooks\":[{\"type\":\"command\"") && now.contains("\"args\":[\"hook\"]");
    if state != json!({"installed": true, "changed": true}) || !entry || !now.ends_with(&SETTINGS[1..]) {
        return Err(format!("{state}, and the settings are now: {now}"));
    }
    Ok("the entry is in the settings".to_string())
}

/// `jellymoon rule` keeps a ruling, which waits in Jellymoon's home.
fn rule(world: &World) -> Result<String, String> {
    let args = ["rule", "--stands", STANDS, "--over", "add a retry to the uploader"].map(str::to_string);
    let ruling = world.json("jellymoon", &args)?;
    let waiting = fs::read_dir(world.at("jm").join("rulings")).map_or(0, Iterator::count);
    if ruling["stands"] != STANDS || waiting != 1 {
        return Err(format!("{waiting} files in the store, not 1: {ruling}"));
    }
    Ok("1 ruling waits".to_string())
}

/// `jellymoon hook` on one made-up step of a session: the lines it printed.
fn hook(world: &World) -> Result<Vec<Value>, String> {
    let step = json!({"session_id": "smoke-1", "hook_event_name": "PostToolUse", "tool_name": "Read"});
    let ran = world.fed("jellymoon", &["hook".to_string()], &step.to_string())?;
    let lines: Result<Vec<Value>, _> = ran.stdout.lines().map(serde_json::from_str).collect();
    lines.ok().filter(|_| ran.code == 0 && ran.stderr.is_empty()).ok_or_else(|| world.printed(&ran))
}

/// The session's next step is handed the ruling, as context on the event it ran for.
fn hook_told(world: &World) -> Result<String, String> {
    let told = hook(world)?;
    let output = told.first().map_or(&Value::Null, |line| &line["hookSpecificOutput"]);
    let context = output["additionalContext"].as_str().unwrap_or_default();
    if told.len() != 1 || output["hookEventName"] != "PostToolUse" || !context.contains(STANDS) {
        return Err(format!("the step was not told the ruling: {told:?}"));
    }
    Ok("told the ruling once".to_string())
}

/// A second step of the same session is told nothing: the hook prints nothing and exits 0.
fn hook_again(world: &World) -> Result<String, String> {
    let told = hook(world)?;
    if told.is_empty() { Ok("told nothing".to_string()) } else { Err(format!("told again: {told:?}")) }
}

/// The statement of the made-up day's clash that the tap chooses: its second.
const TAPPED: &str = "never retry uploads";

/// `jellymoon settle`, the tap, on the second statement of the kept page's clash makes one ruling, which waits beside
/// the first, and the page keeps it.
fn tap(world: &World) -> Result<String, String> {
    let args = ["settle", "--date", DAY, "--clash", "1", "--option", "2"].map(str::to_string);
    let said = world.json("jellymoon", &args)?;
    let waiting = fs::read_dir(world.at("jm").join("rulings")).into_iter().flatten().flatten();
    let waiting = waiting.filter(|entry| entry.path().extension().is_some_and(|kind| kind == "json")).count();
    let ruled = said["ruling"]["stands"] == TAPPED && said["ruling"]["over"] == "add a retry";
    if !ruled || said["stands"] != TAPPED || !said["unkept"].is_null() || waiting != 2 {
        return Err(format!("{waiting} rulings in the store, not 2: {said}"));
    }
    Ok("a tap on the clash made 1 ruling, and it waits".to_string())
}

/// The session's next step is handed the tapped ruling, and only that one: it heard the other already.
fn tap_told(world: &World) -> Result<String, String> {
    let told = hook(world)?;
    let context =
        told.first().map_or("", |line| line["hookSpecificOutput"]["additionalContext"].as_str().unwrap_or(""));
    let ours = format!("What stands: \"{TAPPED}\". It replaces: \"add a retry\".");
    if told.len() != 1 || !context.contains(&ours) || context.matches("What stands").count() != 1 {
        return Err(format!("the step was not told the tapped ruling alone: {told:?}"));
    }
    if !hook(world)?.is_empty() {
        return Err("the tapped ruling was told twice".to_string());
    }
    Ok("told the tapped ruling once".to_string())
}

/// `jellymoon-app layout` draws the clash settled, the replaced statement struck, and says nothing about delivery.
fn today_settled(world: &World) -> Result<String, String> {
    let pieces = world.lines("jellymoon-app", &world.day("layout"))?;
    let of_clash = |piece: &&Value| piece["id"] == "clash-1" && piece["kind"] == "option";
    let roles: Vec<&str> =
        pieces.iter().filter(of_clash).filter_map(|piece| piece["text"][0]["role"].as_str()).collect();
    let all = pieces.iter().map(Value::to_string).collect::<String>();
    if roles != ["option_struck", "option"] || !all.contains("you chose") || all.contains("not told yet") {
        return Err(format!("the clash is not drawn settled with delivery on: options {roles:?}"));
    }
    Ok("the clash is settled, its first option struck; no delivery note".to_string())
}

/// `jellymoon uninstall` removes the app and the command from where an install put them, and leaves nothing there.
/// It takes the delivery hook out too, and the settings are as they were.
fn uninstall(world: &World) -> Result<String, String> {
    let (apps, installed) = (world.at("apps"), world.at("installed"));
    write(&apps.join("Jellymoon.app").join("Contents").join("Info.plist"), "")?;
    write(&installed.join(format!("jellymoon{}", env::consts::EXE_SUFFIX)), "")?;
    let done = world.json("jellymoon", &["uninstall".to_string()])?;
    let removed = done["removed"].as_array().map_or(0, Vec::len);
    let left: usize = [apps, installed].iter().map(|folder| fs::read_dir(folder).map_or(1, Iterator::count)).sum();
    let restored = fs::read_to_string(settings(world)).is_ok_and(|now| now == SETTINGS);
    if (removed, left) != (2, 0) || done["hook"] != true || !restored {
        return Err(format!(
            "{removed} removed and {left} left, not 2 and 0, or the hook is still in the settings: {done}"
        ));
    }
    Ok("2 removed, nothing left where they were; the hook is out and the settings are as they were".to_string())
}

/// The app opens the page in a window and closes it by itself. It needs a display, so it sees this process's variables.
fn window(world: &World) -> Result<String, String> {
    let mut command = Command::new(world.program("jellymoon-app"));
    command.arg("--page").arg(world.at("page.json")).env("JELLYMOON_QUIT_AFTER", "3").stdin(Stdio::null());
    let ran = world.wait(command, Duration::from_secs(30))?;
    if ran.code != 0 || ran.stderr.contains("panicked") {
        return Err(world.printed(&ran));
    }
    Ok("opened, and closed by itself".to_string())
}
