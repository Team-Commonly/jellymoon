//! The `hear` step (F55): a made-up log of something typed in one folder, and `jellymoon hear` fed a hold of a
//! second, the ear's final words and five seconds of ticks. The words are shown as a heard wish for that folder's
//! project, the last line names the wish that was made, and the page shows it working in the same words. No key is
//! pressed, no sound is heard and no run is started. The project's name is whatever core read off the folder: the
//! chain's home may be inside a harness worktree of this repo, where a folder is read as the repo's own.

use serde_json::{Value, json};

use super::{DAY, World, text, write};

const SAY: &str = "say hello from the smoke chain";

pub fn hear(world: &World) -> Result<String, String> {
    let logs = world.at("hear-logs");
    let typed = json!({"type": "user", "timestamp": format!("{DAY}T16:00:00.000Z"), "cwd": text(&world.at("heard")),
        "sessionId": "s", "origin": {"kind": "human"}, "message": {"role": "user", "content": "look at the notes"}});
    write(&logs.join("-work-heard").join("s.jsonl"), &format!("{typed}\n"))?;
    let said = json!({"said": "final", "words": SAY});
    let events = format!(
        "{{\"tick\":0}}\n{{\"key\":\"down\"}}\n{{\"tick\":1000}}\n{{\"key\":\"up\"}}\n{said}\n{{\"tick\":5000}}\n"
    );
    let args = ["hear", "--date", DAY, "--logs", &text(&logs)].map(str::to_string);
    let ran = world.fed("jellymoon", &args, &events)?;
    let lines: Result<Vec<Value>, _> = ran.stdout.lines().map(serde_json::from_str).collect();
    let lines = lines.ok().filter(|_| ran.code == 0 && ran.stderr.is_empty()).ok_or_else(|| world.printed(&ran))?;
    let heard = lines.iter().find(|line| line["state"] == "heard");
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
