use super::*;

/// What is read of a steps file that holds `text`.
fn read(text: &str) -> Steps {
    let dir = tempfile::tempdir().unwrap();
    let at = dir.path().join(FILE);
    hook::put(&at, text, false).unwrap();
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
