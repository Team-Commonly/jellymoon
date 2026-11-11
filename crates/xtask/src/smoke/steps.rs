//! The smoke chain. A feature that adds a command adds its step here: one function, and one line in `STEPS` where it
//! belongs in the order. A step returns counts and states for its line, and its checks of what the made-up day holds
//! go through `World::made_up`, so `--real` runs the same step without them and never prints the person's words.

use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;
use std::time::Duration;
use std::{env, fs};

use serde_json::{Value, json};

use super::{DAY, Only, Step, World, text, write};

pub const STEPS: &[Step] = &[
    Step { name: "logs", only: Only::Synthetic, run: logs },
    Step { name: "day", only: Only::Any, run: day },
    Step { name: "page", only: Only::Any, run: page },
    Step { name: "layout-sticker", only: Only::Any, run: |world| layout(world, "sticker") },
    Step { name: "layout-plain", only: Only::Any, run: |world| layout(world, "plain") },
    Step { name: "broken-page", only: Only::Any, run: broken_page },
    Step { name: "empty-day", only: Only::Synthetic, run: empty_day },
    // Not with --real: an uninstall will also take the delivery hook out of the harness's own settings.
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
    let page = fs::read_to_string(world.at(file)).map_err(|e| format!("{file}: {e}"))?;
    let page: Value = serde_json::from_str(&page).map_err(|e| format!("{file}: {e}"))?;
    let (wishes, clashes) = held(&page)?;
    let args = ["layout", "--page", &text(&world.at(file)), "--theme", theme].map(str::to_string);
    let pieces = world.lines("jellymoon-app", &args)?;
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
    Ok(found)
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

/// A day nobody typed in is an empty page, made with no reading call, and the app draws it.
fn empty_day(world: &World) -> Result<String, String> {
    let seen = world.harness().with_file_name("stdin");
    fs::remove_file(&seen).map_err(|e| format!("what the harness saw of the page step: {e}"))?;
    let args = ["page", "--date", "2026-03-11", "--logs", &text(&world.at("logs"))].map(str::to_string);
    let page = world.json("jellymoon", &args)?;
    let (wishes, clashes) = held(&page)?;
    if (wishes.len(), clashes.len(), &page["read"]["turns"]) != (0, 0, &json!(0)) || seen.exists() {
        return Err(format!("not an empty page with no call (a call was made: {}): {page}", seen.exists()));
    }
    write(&world.at("empty.json"), &page.to_string())?;
    Ok(format!("an empty page and no call; {}", placed(world, "empty.json", "sticker")?))
}

/// `jellymoon uninstall` removes the app and the command from where an install put them, and leaves nothing there.
fn uninstall(world: &World) -> Result<String, String> {
    let (apps, installed) = (world.at("apps"), world.at("installed"));
    write(&apps.join("Jellymoon.app").join("Contents").join("Info.plist"), "")?;
    write(&installed.join(format!("jellymoon{}", env::consts::EXE_SUFFIX)), "")?;
    let done = world.json("jellymoon", &["uninstall".to_string()])?;
    let removed = done["removed"].as_array().map_or(0, Vec::len);
    let left: usize = [apps, installed].iter().map(|folder| fs::read_dir(folder).map_or(1, Iterator::count)).sum();
    if (removed, left) != (2, 0) {
        return Err(format!("{removed} removed and {left} left, not 2 and 0: {done}"));
    }
    Ok("2 removed, nothing left where they were".to_string())
}

/// The app opens the page in a window and closes it by itself. It needs a display, so it sees this process's variables.
fn window(world: &World) -> Result<String, String> {
    let mut command = Command::new(world.program("jellymoon-app"));
    command.arg("--page").arg(world.at("page.json")).env("JELLYMOON_QUIT_AFTER", "3");
    let ran = world.wait(command, Duration::from_secs(30))?;
    if ran.code != 0 || ran.stderr.contains("panicked") {
        return Err(world.printed(&ran));
    }
    Ok("opened, and closed by itself".to_string())
}
