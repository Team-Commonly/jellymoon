//! `voice.toml` in Jellymoon's home, read by `jellymoon hear --settings`: the key and the language, the defaults,
//! and every file that is refused, each in one line that names what is wrong.

use std::fs;

use serde_json::{Value, json};

use super::*;
use crate::support::{Ran, hub_key, shows, tree};

impl World {
    /// Leaves `text` as `voice.toml`, for its owner alone to write, whatever the umask the tests run under.
    fn voice_toml(&self, text: impl AsRef<[u8]>) {
        fs::create_dir_all(self.at("jm")).unwrap();
        fs::write(self.jm("voice.toml"), text).unwrap();
        #[cfg(unix)]
        fs::set_permissions(self.jm("voice.toml"), std::os::unix::fs::PermissionsExt::from_mode(0o644)).unwrap();
    }

    fn settings(&self) -> Ran {
        self.run(&["hear", "--settings"])
    }

    /// The settings `text` gives.
    fn voice(&self, text: &str) -> Value {
        self.voice_toml(text);
        printed(&self.settings())
    }

    /// The one line `text` is refused with, after `voice.toml is not used: `. Nothing is written.
    fn unused(&self, text: &str) -> String {
        self.voice_toml(text);
        let before = tree(self.dir.path());
        let ran = self.settings();
        let line = refused(&ran).strip_prefix("jellymoon: voice.toml is not used: ").expect(&ran.stderr).to_string();
        assert!(tree(self.dir.path()) == before);
        line.trim_end().to_string()
    }
}

fn voice(key: &str, language: &str) -> Value {
    json!({"key": key, "language": language})
}

#[test]
fn a_file_that_is_absent_gives_the_defaults_and_a_name_left_out_has_its_default() {
    let w = heard_world();
    let defaults = voice("Control+Shift+Space", "default");
    assert_eq!(printed(&w.settings()), defaults);
    assert!(!w.at("jm").exists(), "asking for the settings made something");
    assert_eq!(w.voice(""), defaults);
    assert_eq!(w.voice("# nothing chosen yet\n\n   \n"), defaults);
    assert_eq!(w.voice("language = \"en-US\"\n"), voice("Control+Shift+Space", "en-US"));
    assert_eq!(w.voice("key = \"Option+J\""), voice("Option+J", "default"));
}

#[test]
fn the_file_names_the_key_and_the_language() {
    let w = heard_world();
    let text =
        "# the key that is held\r\nkey = \"Control+Option+Command+Shift+F12\"\r\n  language=\"zh-Hans-CN\"  \r\n";
    assert_eq!(w.voice(text), voice("Control+Option+Command+Shift+F12", "zh-Hans-CN"));
    for key in ["Command+A", "Option+Z", "Control+0", "Option+9", "Control+F1", "Shift+Command+Space"] {
        assert_eq!(w.voice(&format!("key = \"{key}\"\n"))["key"], key);
    }
    for language in ["default", "en", "fil", "en-GB", "es-419", "zh-Hant", "sr-Latn-RS"] {
        assert_eq!(w.voice(&format!("language = \"{language}\"\n"))["language"], language);
    }
}

#[test]
fn a_key_core_does_not_know_is_refused_in_one_line_that_names_it() {
    let w = heard_world();
    let takes = "is not one or more of Control, Shift, Option and Command, each once, and then one of A to Z, 0 to \
                 9, F1 to F12 or Space, joined by +";
    let keys = [
        "Space",
        "A",
        "",
        "Control",
        "Control+",
        "+Space",
        "Control+Shift",
        "Control+Control+Space",
        "Hyper+Space",
        "control+space",
        "Control+a",
        "Control+AB",
        "Control+F0",
        "Control+F13",
        "Control+F01",
        "Control+Space+Shift",
        "Control + Space",
        "Ctrl+Space",
        "Control+Escape",
        "Control+É",
    ];
    for key in keys {
        assert_eq!(w.unused(&format!("key = \"{key}\"\n")), format!("the key \"{key}\" {takes}"), "{key}");
    }
    // What is named is cleaned and cut as any text core prints: a credential's shape is not said back.
    let secret = hub_key();
    let line = w.unused(&format!("key = \"Control+{secret}\"\n"));
    assert!(!shows(&line, &secret) && line.starts_with("the key \"Control+"), "{line}");
    // A key or a language that holds what cannot be seen would be named back as one that reads right: it is not named.
    for unseen in ["Control+Shift+Space\u{200b}", "Con\u{2065}trol+Space", "Control+\u{1b}[31mSpace"] {
        let line = w.unused(&format!("key = \"{unseen}\"\n"));
        assert_eq!(line, "the key holds a character that cannot be seen", "{unseen:?}");
    }
    let line = w.unused("language = \"en\u{feff}-US\"\n");
    assert_eq!(line, "the language holds a character that cannot be seen");
    // Shift alone is how capitals and signs are typed: such a key would take them from every program.
    for typing in ["Shift+A", "Shift+Space", "Shift+7", "Shift+F5"] {
        let line = w.unused(&format!("key = \"{typing}\"\n"));
        let alone = "the key is held with Shift alone, which typing uses: add Control, Option or Command to it";
        assert_eq!(line, alone, "{typing}");
    }
    assert_eq!(w.voice("key = \"Control+Shift+A\"\n")["key"], "Control+Shift+A");
    let line = w.unused(&format!("key = \"{}\"\n", "Hyper+".repeat(50)));
    assert!(line.starts_with("the key \"Hyper+Hyper+") && line.chars().count() < 40 + takes.len() + 12, "{line}");
}

#[test]
fn a_language_core_does_not_know_is_refused_in_one_line_that_names_it() {
    let w = heard_world();
    let languages =
        ["", "English", "EN", "en_US", "en-us", "en-USA", "en-US-Latn", "e", "en-", "en-Latn-US-x", "ën", "abcd"];
    for language in languages {
        let line = format!("the language \"{language}\" is not `default` or a tag such as en-US");
        assert_eq!(w.unused(&format!("language = \"{language}\"\n")), line, "{language}");
    }
}

#[test]
fn a_file_that_is_not_two_plain_names_is_refused_whole() {
    let w = heard_world();
    let odd = |line: usize| format!("line {line} is not `key = \"...\"` or `language = \"...\"`");
    let cases = [
        ("volume = \"11\"\n", odd(1)),
        ("# fine\nkey = \"Option+J\"\nlanguage\n", odd(3)),
        ("key = Option+J\n", odd(1)),
        ("key = 'Option+J'\n", odd(1)),
        ("key = \"Option+J\" # the key\n", odd(1)),
        ("key = \"Option+\\u004a\"\n", odd(1)),
        ("key = \"Option+\"J\"\"\n", odd(1)),
        ("key = \"\n", odd(1)),
        ("[voice]\nkey = \"Option+J\"\n", odd(1)),
        ("Key = \"Option+J\"\n", odd(1)),
        ("key = \"Option+J\"\nlanguage = \"en\"\nkey = \"Option+K\"\n", "line 3 names key a second time".to_string()),
        ("language = \"en\"\nlanguage = \"en\"\n", "line 2 names language a second time".to_string()),
    ];
    for (text, line) in cases {
        assert_eq!(w.unused(text), line, "{text:?}");
    }
    assert_eq!(w.unused(&format!("# {}\n", "x".repeat(4096))), "it is too large");
    assert_eq!(w.voice(&format!("#{}\n", "x".repeat(4094))), voice("Control+Shift+Space", "default"), "4096 bytes");
    w.voice_toml(b"key = \"Option+\xff\"\n");
    assert_eq!(refused(&w.settings()), "jellymoon: voice.toml is not used: it is not text\n");
    fs::remove_file(w.jm("voice.toml")).unwrap();
    fs::create_dir(w.jm("voice.toml")).unwrap();
    assert_eq!(refused(&w.settings()), "jellymoon: voice.toml is not used: it is not a plain file\n");
}

#[cfg(unix)]
#[test]
fn a_file_that_is_a_link_or_that_others_can_write_to_is_refused() {
    let w = heard_world();
    w.voice_toml("key = \"Option+J\"\n");
    fs::set_permissions(w.jm("voice.toml"), std::os::unix::fs::PermissionsExt::from_mode(0o664)).unwrap();
    assert_eq!(refused(&w.settings()), "jellymoon: voice.toml is not used: others can write to it\n");
    fs::rename(w.jm("voice.toml"), w.at("elsewhere.toml")).unwrap();
    fs::set_permissions(w.at("elsewhere.toml"), std::os::unix::fs::PermissionsExt::from_mode(0o644)).unwrap();
    std::os::unix::fs::symlink(w.at("elsewhere.toml"), w.jm("voice.toml")).unwrap();
    assert_eq!(refused(&w.settings()), "jellymoon: voice.toml is not used: it is not a plain file\n");
}

#[test]
fn settings_takes_nothing_else() {
    let w = heard_world();
    for more in [&["--run"][..], &["--date", DAY], &["--logs", "x"], &["--settings"]] {
        let ran = w.run(&[&["hear", "--settings"][..], more].concat());
        assert_eq!((ran.code, ran.stdout.as_str()), (2, ""), "{more:?}");
        assert!(ran.stderr.starts_with("jellymoon: hear --settings takes nothing else\nusage:"), "{more:?}");
    }
}
