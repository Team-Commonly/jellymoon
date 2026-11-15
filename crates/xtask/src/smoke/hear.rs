//! The `hear` step (F55): a made-up log of something typed in one folder, and `jellymoon hear` fed a hold of a
//! second, the ear's final words and five seconds of ticks. The words are shown as a heard wish for that folder's
//! project, the last line names the wish that was made, and the page shows it working in the same words. No key is
//! pressed, no sound is heard and no run is started. The project's name is whatever core read off the folder: the
//! chain's home may be inside a harness worktree of this repo, where a folder is read as the repo's own.
//! The `jelly` step (F56) draws the line that showed the heard wish: `jellymoon-app layout --jelly` places the
//! jellyfish and its bubble on a screen, and opens no window.

use serde_json::{Value, json};

use super::{DAY, World, text, write};

const SAY: &str = "say hello from the smoke chain";
const HEARD_LINE: &str = "heard-line.json";

pub fn hear(world: &World) -> Result<String, String> {
    let logs = world.at("hear-logs");
    let typed = json!({"type": "user", "timestamp": format!("{DAY}T16:00:00.000Z"), "cwd": text(&world.at("heard")),
        "sessionId": "s", "origin": {"kind": "human"}, "message": {"role": "user", "content": "look at the notes"}});
    write(&logs.join("-work-heard").join("s.jsonl"), &format!("{typed}\n"))?;
    // The chain's runs listed their repos (F62). They are forgotten, so the one folder typed in is the one on offer.
    let listed = world.json("jellymoon", &["project", "list"].map(str::to_string))?;
    for known in listed["projects"].as_array().into_iter().flatten() {
        let name = known["name"].as_str().unwrap_or_default();
        world.json("jellymoon", &["project", "forget", name].map(str::to_string))?;
    }
    let said = json!({"said": "final", "words": SAY});
    let events = format!(
        "{{\"tick\":0}}\n{{\"key\":\"down\"}}\n{{\"tick\":1000}}\n{{\"key\":\"up\"}}\n{said}\n{{\"tick\":1000}}\n{{\"tick\":5000}}\n"
    );
    let args = ["hear", "--date", DAY, "--logs", &text(&logs)].map(str::to_string);
    let ran = world.fed("jellymoon", &args, &events)?;
    let lines: Result<Vec<Value>, _> = ran.stdout.lines().map(serde_json::from_str).collect();
    let lines = lines.ok().filter(|_| ran.code == 0 && ran.stderr.is_empty()).ok_or_else(|| world.printed(&ran))?;
    let heard = lines.iter().find(|line| line["state"] == "heard");
    // The line as it was printed, for the `jelly` step to draw.
    let printed = ran.stdout.lines().find(|line| line.contains("\"state\":\"heard\"")).unwrap_or_default();
    write(&world.at(HEARD_LINE), &format!("{printed}\n"))?;
    let shown = heard.is_some_and(|line| line["words"] == SAY && line["left"] == 4000);
    let project = heard.and_then(|line| line["project"].as_str()).unwrap_or_default();
    let made = lines.last().and_then(|line| line["made"].as_str()).unwrap_or_default();

    let page =
        world.json("jellymoon", &["page", "--date", DAY, "--logs", &text(&world.at("no-logs"))].map(str::to_string))?;
    let on_page = page["wishes"].as_array().and_then(|wishes| wishes.iter().find(|on| on["id"] == made));
    let kept = on_page.is_some_and(|on| on["said"] == SAY && on["project"] == project && on["state"] == "working");
    let yes = |so: bool| if so { "yes" } else { "no" };
    let found = format!(
        "{} lines, the words shown as a heard wish for 4000 ms {}, a project picked {}, made {}, on the page working \
         in the same words {}",
        lines.len(),
        yes(shown),
        yes(!project.is_empty()),
        if made.is_empty() { "nothing" } else { made },
        yes(kept)
    );
    if shown && !project.is_empty() && !made.is_empty() && kept { Ok(found) } else { Err(found) }
}

/// `jellymoon-app layout --jelly` draws the heard wish's line: the jellyfish holding it up in its corner window, and
/// in the bubble's window the words, the project and a full ring.
pub fn jelly(world: &World) -> Result<String, String> {
    let args = ["layout", "--jelly", &text(&world.at(HEARD_LINE)), "--screen", "1440x900"].map(str::to_string);
    let pieces = world.lines("jellymoon-app", &args)?;
    let of = |kind: &'static str| pieces.iter().filter(move |piece| piece["kind"] == kind);
    let windows: Vec<&str> = of("window").filter_map(|window| window["id"].as_str()).collect();
    let holds = of("character").any(|jelly| jelly["state"] == "heard" && jelly["mood"] == "holds_up");
    let runs = of("bubble").filter_map(|bubble| bubble["text"].as_array()).flatten();
    let words: Vec<&str> =
        runs.filter(|run| run["role"] == "row_main").filter_map(|run| run["text"].as_str()).collect();
    let said = words.join(" ") == SAY;
    let full = of("ring").any(|ring| ring["ring"] == 1.0);
    // What each window is asked for (F57): a pop-up one sees through that never has the keys.
    let asked = |window: &serde_json::Value| {
        let opens = &window["opens"];
        opens["kind"] == "pop_up" && opens["focus"] == false && opens["see_through"] == true
    };
    let pop_ups = of("window").all(asked);
    let yes = |so: bool| if so { "yes" } else { "no" };
    let found = format!(
        "{} lines, windows {}, each asked as a pop-up that takes no keys {}, the jellyfish holds the wish up {}, \
         the bubble says its words {}, a full ring {}",
        pieces.len(),
        windows.join(" and "),
        yes(pop_ups),
        yes(holds),
        yes(said),
        yes(full)
    );
    if windows == ["jelly", "bubble"] && pop_ups && holds && said && full { Ok(found) } else { Err(found) }
}
