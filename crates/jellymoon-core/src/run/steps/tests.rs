use super::*;

/// What is read of a steps file that holds `text`.
fn read(text: &str) -> Steps {
    let dir = tempfile::tempdir().unwrap();
    let at = dir.path().join(FILE);
    crate::hook::put(&at, text, false).unwrap();
    kept(&at)
}

// The file is anyone's who can write the home: it is read as the reader writes it, and its names are cleaned again.
#[test]
fn kept_steps_are_read_back_as_written_and_as_nothing_else() {
    let (read_a, glob) = (Step { tool: Tool::Read, file: Some("a.txt".into()) }, Step { tool: Tool::Glob, file: None });
    let steps = Steps { count: 7, last: vec![read_a.clone(), glob.clone()] };
    let dir = tempfile::tempdir().unwrap();
    keep(&dir.path().join(FILE), &steps);
    let written = fs::read_to_string(dir.path().join(FILE)).unwrap();
    assert_eq!(written, "{\"count\":7,\"last\":[{\"tool\":\"read\",\"file\":\"a.txt\"},{\"tool\":\"glob\"}]}\n");
    assert_eq!(kept(&dir.path().join(FILE)), steps);
    let five = format!("{{\"count\":9,\"last\":[{}]}}", [r#"{"tool":"grep"}"#; 5].join(","));
    let none = [
        five.as_str(),
        r#"{"count":0,"last":[{"tool":"grep"}]}"#,
        r#"{"count":1,"last":[{"tool":"bash"}]}"#,
        r#"{"count":1,"last":[{"tool":"Read"}]}"#,
        r#"{"count":1,"last":[],"text":"more"}"#,
        r#"{"count":1,"last":[{"tool":"read","file":"a.txt","input":"x"}]}"#,
        r#"{"count":-1,"last":[]}"#,
        "[]",
    ];
    for text in none {
        assert_eq!(read(text), Steps::default(), "{text}");
    }
    // A name that does not pass is no name, and a tool that names no file has none.
    let names = [("a\\u200bb.txt", "read"), ("2 more.txt", "edit"), ("/etc/passwd", "read"), ("a.txt", "grep")];
    for (name, tool) in names {
        let text = format!(r#"{{"count":3,"last":[{{"tool":"{tool}","file":"{name}"}}]}}"#);
        assert_eq!(read(&text).last[0].file, None, "{text}");
    }
    assert_eq!(kept(&dir.path().join("none.json")), Steps::default());
}

// A link the repo tracks can lead out of the copy: a file named through one has no name. Unix only: a link cannot be
// made on Windows without a right a test does not have.
#[cfg(unix)]
#[test]
fn a_file_named_through_a_link_in_the_place_has_no_name() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let (place, outside) = (dir.path().join("copy"), dir.path().join("outside"));
    fs::create_dir_all(place.join("src")).unwrap();
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("secret.txt"), "not the run's\n").unwrap();
    fs::write(place.join("src").join("real.txt"), "the run's\n").unwrap();
    symlink(&outside, place.join("inner")).unwrap();
    symlink(outside.join("secret.txt"), place.join("src").join("near.txt")).unwrap();
    let places = [place.clone()];
    let name = |within: &str| inside(&place.join(within).display().to_string(), &places);
    assert_eq!(name("inner/secret.txt"), None, "a folder that is a link");
    assert_eq!(name("src/near.txt"), None, "the file itself a link");
    assert_eq!(name("inner"), None);
    assert_eq!(name("src/real.txt").as_deref(), Some("src/real.txt"));
    assert_eq!(name("src/new.txt").as_deref(), Some("src/new.txt"), "a file about to be made");
    // The place itself may be reached through a link: only the parts under it are asked.
    let through = dir.path().join("through");
    symlink(&place, &through).unwrap();
    let file = through.join("src").join("real.txt").display().to_string();
    assert_eq!(inside(&file, &[through]).as_deref(), Some("src/real.txt"));
}

// The name bounds the path before the disk is asked: a path that fills a line of the stream (1 MiB) is no name at
// once, and its parts are not walked one by one. Walked, it took many seconds: longer than a run's end waits.
#[test]
fn a_path_too_long_to_be_a_name_is_refused_before_the_disk_is_asked() {
    let dir = tempfile::tempdir().unwrap();
    let places = [dir.path().to_path_buf()];
    let long = format!("{}{}", places[0].display(), "/a".repeat(500_000));
    let started = std::time::Instant::now();
    assert_eq!(inside(&long, &places), None);
    assert!(started.elapsed() < std::time::Duration::from_secs(5), "walked: {:?}", started.elapsed());
    let near = format!("{}/{}", places[0].display(), "b/".repeat(40) + "c.txt");
    assert_eq!(inside(&near, &places).map(|name| name.len()), Some(85), "a name within the bound is kept");
}

// A line can hold thousands of steps and only the last few can stay kept: the others are counted and never named,
// so a line costs no more names than are kept.
#[test]
fn only_the_steps_of_a_line_that_can_stay_kept_are_named() {
    let block = |nth: usize| format!(r#"{{"type":"tool_use","name":"Read","input":{{"file_path":"f{nth}"}}}}"#);
    let blocks: Vec<String> = (0..50).map(block).collect();
    let line = format!(r#"{{"type":"assistant","message":{{"content":[{}]}}}}"#, blocks.join(","));
    let (mut steps, mut asked) = (Steps::default(), Vec::new());
    let took = steps.took_as(line.as_bytes(), |path| {
        asked.push(path.to_string());
        Some(path.to_string())
    });
    assert!(took);
    assert_eq!(asked, ["f46", "f47", "f48", "f49"]);
    assert_eq!(steps.count, 50);
    let names: Vec<_> = steps.last.iter().map(|step| step.file.as_deref()).collect();
    assert_eq!(names, [Some("f46"), Some("f47"), Some("f48"), Some("f49")]);
}

// A name cleaning would cut or change is refused before it is cleaned: cleaning a long name with a line break in
// it costs many times its length, and a line of the stream can hold hundreds of them.
#[test]
fn a_name_cleaning_would_refuse_is_refused_before_it_is_cleaned() {
    let long = format!("{}\n{}", "a ".repeat(1000), "b ".repeat(1000));
    let started = std::time::Instant::now();
    for _ in 0..300 {
        assert_eq!(own(Path::new(&long)), None);
    }
    assert!(started.elapsed() < std::time::Duration::from_secs(1), "cleaned: {:?}", started.elapsed());
    assert_eq!(own(Path::new("a\tb.txt")), None, "a control character");
    assert_eq!(own(Path::new(&"a".repeat(801))), None, "longer than any name");
    assert_eq!(own(Path::new("src/a b.txt")).as_deref(), Some("src/a b.txt"));
}
