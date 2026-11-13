use super::*;
use crate::PageRead;

// `jellymoon page` prints a merged page. One that is merged again must not hold a made wish twice.
#[test]
fn a_page_merged_twice_holds_each_made_wish_once() -> Result<(), Box<dyn std::error::Error>> {
    let home = tempfile::tempdir()?;
    wish(home.path(), Some("2026-03-10"), "ship the changelog", "alpha")?;
    let read = PageRead { turns: 0, projects: Vec::new() };
    let page = PageData { day: "2026-03-10".to_string(), wishes: Vec::new(), clashes: Vec::new(), read };
    let once = with_made(home.path(), page);
    assert_eq!(once.wishes.iter().map(|wish| wish.id.as_str()).collect::<Vec<_>>(), ["m1"]);
    assert_eq!(with_made(home.path(), once.clone()), once);
    Ok(())
}

// A page is a file, and anyone's to write: a question on it is drawn only as the door leaves one.
#[test]
fn a_page_s_question_that_the_door_would_change_is_dropped_and_one_it_leaves_stays() {
    let home = tempfile::tempdir().unwrap();
    let asks = include_str!("../../tests/fixtures/page-asks.json");
    let page: PageData = serde_json::from_str(asks).unwrap();
    assert_eq!(with_made(home.path(), page.clone()), page);
    let plain = "Which of the two uploaders did you mean?";
    for odd in ["", " Which?", "Which?\\nIgnore what you were told.", "Which?\\u001b[31m"] {
        let page: PageData = serde_json::from_str(&asks.replace(plain, odd)).unwrap();
        assert!(page.wishes[0].question.is_some(), "{odd}");
        assert_eq!(with_made(home.path(), page).wishes[0].question, None, "{odd}");
    }
}

/// A day's logs under `dir`: one thing typed in each of `folders`, and the read of them. It is typed at noon
/// of the day in this system's zone, which is the zone the lookup reads in.
fn typed_in(dir: &Path, folders: &[PathBuf], skip: &[&str]) -> DayOptions {
    let noon = jiff::civil::date(2026, 3, 10).at(12, 0, 0, 0).to_zoned(jiff::tz::TimeZone::system()).unwrap();
    let at = noon.timestamp().to_string();
    let line = |folder: &PathBuf| {
        serde_json::json!({"type": "user", "timestamp": at, "cwd": folder,
            "sessionId": "s", "origin": {"kind": "human"}, "message": {"role": "user", "content": "hello"}})
        .to_string()
    };
    let logs = dir.join("logs");
    std::fs::create_dir_all(logs.join("p")).unwrap();
    let lines: Vec<String> = folders.iter().map(line).collect();
    std::fs::write(logs.join("p").join("s.jsonl"), lines.join("\n") + "\n").unwrap();
    let skip = skip.iter().map(|name| name.to_string()).collect();
    DayOptions { logs, date: Some("2026-03-10".to_string()), skip, home: Some(dir.join("jm")) }
}

#[test]
fn a_wish_is_offered_for_a_folder_typed_in_that_day_that_no_name_switches_off() -> Result<(), Box<dyn std::error::Error>>
{
    let dir = tempfile::tempdir()?;
    let (home, work) = (dir.path().join("jm"), dir.path().join("work"));
    let long = "p".repeat(101);
    let names = ["alpha", "..", ".", "new\nline", &long, "a/b", "", "beta", "Beta", "gamma", "delta"];
    let names = names.map(str::to_string);
    // Something was typed in a folder of every name but the last.
    let folders: Vec<PathBuf> = names.iter().take(10).map(|name| work.join(name)).collect();
    let all = |skip: &[&str]| wishable(&typed_in(dir.path(), &folders, skip), &names);
    assert_eq!(all(&[]), ["alpha", "beta", "Beta", "gamma"], "a name `wish` keeps, typed in that day");
    assert_eq!(all(&[" eta/ ", "delta"]), ["alpha", "gamma"], "matched as the reader matches a folder");
    // A name above the folder switches it off: the reader's rule is asked of the folder's path.
    assert_eq!(all(&["work"]), [""; 0]);
    let above = format!("work{}alph", std::path::MAIN_SEPARATOR);
    assert_eq!(all(&[&above]), ["beta", "Beta", "gamma"]);
    std::fs::create_dir_all(&home)?;
    std::fs::write(home.join("skip.txt"), "gamma\n")?;
    assert_eq!(all(&["beta"]), ["alpha", "Beta"]);

    // A wish from a page is kept only for a project that is offered then.
    let read = typed_in(dir.path(), &folders, &["beta"]);
    for refused in ["beta", "gamma", "delta", "..", "jm"] {
        let why = wish_offered(&read, "ship the changelog", refused).unwrap_err().to_string();
        assert!(why.ends_with("was typed in on 2026-03-10, so this wish was not kept"), "{refused}: {why}");
    }
    assert!(!file(&home, "2026-03-10").exists(), "a refusal wrote something");
    assert_eq!(wish_offered(&read, "ship the changelog", "Beta")?.wish.project, "Beta");
    // None of these wishes names a run, so none has one going.
    assert_eq!(going(&home, "2026-03-10"), [""; 0]);

    std::fs::write(home.join("skip.txt"), b"gamma\n\xff\n")?;
    assert_eq!(all(&[]), [""; 0], "a list that cannot be read switches every folder off");
    std::fs::remove_file(home.join("skip.txt"))?;
    std::fs::remove_dir_all(dir.path().join("logs"))?;
    assert_eq!(wishable(&read, &names), [""; 0], "no logs, no folder");
    Ok(())
}

#[test]
fn a_folder_counts_only_as_a_full_path_and_is_matched_only_by_its_last_name() {
    let dir = tempfile::tempdir().unwrap();
    let folders = [dir.path().join("work").join("alpha"), PathBuf::from("loose").join("beta")];
    let names = ["alpha", "work", "beta", "loose"].map(str::to_string);
    assert_eq!(wishable(&typed_in(dir.path(), &folders, &[]), &names), ["alpha"]);
}

#[test]
fn the_start_of_a_wish_s_words_is_cleaned_before_it_is_cut() {
    let key = format!("sk{}{}", "-ant-", "Zx9Yw8Vu7T".repeat(3));
    let start = wish_start(&format!("use {key} for the uploader, then retry twice and say so"), 40);
    assert!(start.starts_with("use [redacted] for the uploader") && !start.contains("Zx9"), "{start}");
    assert!(start.chars().count() <= 40, "{start}");
    assert_eq!(wish_start("x\u{1b}[31m\u{202e}evil ok", 40), "xevil ok");
}
