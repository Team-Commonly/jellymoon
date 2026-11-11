//! F29: removal never writes settings that are not a JSON object, whatever a person put beside the entry since it
//! was installed. `@` in a case stands for the entry exactly as install wrote it.

use std::fs;

use serde_json::{Value, json};

use super::{World, world};

const A: &str = r#"{"matcher":"Bash","hooks":[]}"#;
const B: &str = r#"{"matcher":"Edit","hooks":[]}"#;

/// Installs into settings with an empty `PostToolUse` list, so the entry went in with no comma, and returns its text.
fn installed() -> (World, String) {
    let w = world();
    fs::write(w.settings(), r#"{"hooks":{"PostToolUse":[]}}"#).unwrap();
    assert_eq!(w.install().code, 0);
    let text = fs::read_to_string(w.settings()).unwrap();
    let entry = text.strip_prefix(r#"{"hooks":{"PostToolUse":["#).unwrap().strip_suffix("]}}").unwrap().to_string();
    (w, entry)
}

#[test]
fn a_neighbour_added_by_hand_beside_the_entry_never_leaves_a_comma_of_its_own() {
    let lists = [
        "[@,A]".to_string(),
        "[A,@]".to_string(),
        "[@,A,B]".to_string(),
        "[A,@,B]".to_string(),
        "[A,B,@]".to_string(),
        "[\n  A,\n  @\n]".to_string(),
        "[\n  @ ,\n  A\n]".to_string(),
        "[@, // mine\n A]".to_string(),
        "[A, /* mine */ B, @]".to_string(),
    ];
    for list in lists {
        let (w, entry) = installed();
        let with = |entry: &str| {
            format!("{{\"hooks\":{{\"PostToolUse\":{}}}}}", list.replace('B', B).replace('A', A).replace('@', entry))
        };
        fs::write(w.settings(), with(&entry)).unwrap();
        assert_eq!(w.ok(&["hook", "uninstall"]), json!({"installed": false, "changed": true}), "{list}");
        let left = fs::read_to_string(w.settings()).unwrap();
        // Only the entry, one comma and blank space went: the rest of the text is there in order, comments too.
        let squeezed = |text: &str| text.split_whitespace().collect::<String>();
        let expected = squeezed(&with("")).replace(",]", "]").replace("[,", "[").replace(",,", ",");
        assert_eq!(squeezed(&left), squeezed(&expected), "{list}");
        assert_eq!(left.contains("mine"), list.contains("mine"), "{list}: {left}");
    }
}

#[test]
fn a_key_added_by_hand_after_the_entry_in_settings_that_were_empty_stays_and_the_file_stays_json() {
    let w = world();
    fs::write(w.settings(), "{}").unwrap();
    assert_eq!(w.install().code, 0);
    let installed = fs::read_to_string(w.settings()).unwrap();
    fs::write(w.settings(), installed.strip_suffix('}').unwrap().to_string() + ",\"model\":\"sonnet\"}").unwrap();
    let mut vars: Vec<_> = w.vars.iter().map(|(name, value)| (*name, value.as_os_str())).collect();
    let (apps, bin) = (w.dir.path().join("Applications"), w.dir.path().join("bin"));
    vars.extend([("JELLYMOON_APP_DIR", apps.as_os_str()), ("JELLYMOON_BIN_DIR", bin.as_os_str())]);
    let ran = super::support::jellymoon(&vars, &["uninstall"]);
    assert_eq!((ran.code, ran.stdout.as_str()), (0, "{\"removed\":[],\"hook\":true,\"hook_by_hand\":false}\n"));
    let left: Value = serde_json::from_str(&fs::read_to_string(w.settings()).unwrap()).expect("still JSON");
    assert_eq!(left, json!({"hooks": {"PostToolUse": []}, "model": "sonnet"}));
}

#[test]
fn an_entry_that_cannot_be_cut_out_safely_is_left_with_its_record_for_the_person() {
    // A comment between the entry and its comma; the entry moved under another event and spaced anew (`#`); and a
    // second copy of it.
    for hooks in [
        "{\"PostToolUse\":[A, /* mine */ @]}",
        "{\"PreToolUse\":[#],\"PostToolUse\":[]}",
        "{\"PostToolUse\":[@,A],\"Stop\":[@]}",
    ] {
        let (w, entry) = installed();
        let edited = format!(
            "{{\"hooks\":{}}}",
            hooks.replace('A', A).replace('@', &entry).replace('#', &entry.replace(",\"args\"", ", \"args\""))
        );
        fs::write(w.settings(), &edited).unwrap();
        for _ in 0..2 {
            let ran = w.fed(&["hook", "uninstall"], "");
            assert_eq!((ran.code, ran.stdout.as_str()), (1, ""), "{hooks}");
            assert!(ran.stderr.contains("remove it by hand"), "{}", ran.stderr);
            assert_eq!(fs::read_to_string(w.settings()).unwrap(), edited, "{hooks}");
        }
        assert!(w.dir.path().join("jm").join("hook.json").exists(), "the record is kept");
    }
}

#[test]
fn settings_whose_hooks_are_no_longer_an_object_hold_no_entry_so_the_record_is_forgotten() {
    for gone in ["{\"hooks\": 5}", "{\"hooks\": {\"PostToolUse\": \"none\"}}", "{\"model\": \"opus\"}", ""] {
        let (w, _) = installed();
        fs::write(w.settings(), gone).unwrap();
        assert_eq!(w.ok(&["hook", "uninstall"]), json!({"installed": false, "changed": false}), "{gone}");
        assert_eq!(fs::read_to_string(w.settings()).unwrap(), gone);
        assert!(!w.dir.path().join("jm").join("hook.json").exists());
    }
}
