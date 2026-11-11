//! The tap that turns delivery on: `jellymoon hook install --installed` installs the hook for the `jellymoon` command
//! in the folder `install.sh` uses, and only for a file core has checked, since Claude Code runs it at every step.
//! The stand-in for the installed command is a copy of the built `jellymoon`: a real program, never a script.

use std::path::{Path, PathBuf};
use std::{env, fs};

use serde_json::{Value, json};

use super::{World, tree, world};
use crate::support::jellymoon_in;

fn name() -> String {
    format!("jellymoon{}", env::consts::EXE_SUFFIX)
}

/// Puts a runnable program in `folder` under `name`, for its owner alone to write.
fn program(folder: &Path, name: &str) -> PathBuf {
    fs::create_dir_all(folder).unwrap();
    let program = folder.join(name);
    fs::copy(env!("CARGO_BIN_EXE_jellymoon"), &program).unwrap();
    #[cfg(unix)]
    super::chmod(&program, 0o755);
    program
}

impl World {
    fn bin(&self) -> PathBuf {
        self.dir.path().join("user").join(".local").join("bin")
    }

    fn settings(&self) -> PathBuf {
        self.dir.path().join("user").join(".claude").join("settings.json")
    }

    /// `jellymoon hook install --installed`, started in the test's folder with these variables over the world's.
    fn turn_on(&self, more: &[(&'static str, &str)]) -> super::Ran {
        fs::create_dir_all(self.settings().parent().unwrap()).unwrap();
        let mut vars: Vec<_> = self.vars.iter().map(|(name, value)| (*name, value.as_os_str())).collect();
        vars.extend(more.iter().map(|(name, value)| (*name, std::ffi::OsStr::new(*value))));
        jellymoon_in(self.dir.path(), &vars, &["hook", "install", "--installed"])
    }

    /// The install was refused in this one line, and nothing was written: no settings, no record.
    fn refused(&self, more: &[(&'static str, &str)], why: &str, case: &str) {
        fs::create_dir_all(self.settings().parent().unwrap()).unwrap();
        let before = tree(self.dir.path());
        let ran = self.turn_on(more);
        assert_eq!(
            (ran.code, ran.stdout.as_str(), ran.stderr.as_str()),
            (1, "", format!("jellymoon: {why}\n").as_str()),
            "{case}"
        );
        assert_eq!(tree(self.dir.path()), before, "{case}: nothing was written");
        assert!(!self.settings().exists() && !self.home().join("hook.json").exists(), "{case}");
    }
}

const NOT_USED: &str = "the installed jellymoon command is not used: ";

#[test]
fn delivery_is_turned_on_for_the_installed_command_and_for_no_other_file() {
    let w = world();
    fs::create_dir_all(w.settings().parent().unwrap()).unwrap();
    w.refused(&[], &format!("{NOT_USED}it was not found"), "no command");
    fs::create_dir_all(w.bin().join(name())).unwrap();
    w.refused(&[], &format!("{NOT_USED}it is not a program"), "a folder at its place");
    fs::remove_dir(w.bin().join(name())).unwrap();
    fs::write(w.bin().join(name()), "").unwrap();
    #[cfg(unix)]
    super::chmod(&w.bin().join(name()), 0o755);
    w.refused(&[], &format!("{NOT_USED}it is not a program"), "an empty file");

    let command = program(&w.bin(), &name());
    // A folder that is not a full path is not looked in, though from the folder the command starts in it leads there.
    let relative = Path::new("user").join(".local").join("bin");
    w.refused(
        &[("JELLYMOON_BIN_DIR", relative.to_str().unwrap())],
        &format!("{NOT_USED}its folder is not an absolute path"),
        "a relative folder",
    );
    let config = "CLAUDE_CONFIG_DIR must be an absolute path; nothing was changed";
    w.refused(&[("CLAUDE_CONFIG_DIR", "user/.claude")], config, "a relative settings folder");

    let ran = w.turn_on(&[]);
    assert_eq!(
        (ran.code, ran.stdout.as_str(), ran.stderr.as_str()),
        (0, "{\"installed\":true,\"changed\":true}\n", "")
    );
    let now: Value = serde_json::from_str(&fs::read_to_string(w.settings()).unwrap()).unwrap();
    let entry = json!({"type": "command", "command": command.to_str().unwrap(), "args": ["hook"]});
    assert_eq!(now, json!({"hooks": {"PostToolUse": [{"hooks": [entry]}]}}), "the entry names the installed command");
    assert_eq!(w.turn_on(&[]).stdout, "{\"installed\":true,\"changed\":false}\n");
    assert_eq!(w.fed(&["hook", "status"], "").stdout, "{\"installed\":true,\"changed\":false}\n");
}

// The plain `hook install`, which names the running program, refuses a relative settings folder too.
#[test]
fn a_relative_claude_config_dir_is_refused_by_hook_install() {
    let w = world();
    let before = tree(w.dir.path());
    let mut vars: Vec<_> = w.vars.iter().map(|(name, value)| (*name, value.as_os_str())).collect();
    vars.push(("CLAUDE_CONFIG_DIR", std::ffi::OsStr::new("user/.claude")));
    let ran = jellymoon_in(w.dir.path(), &vars, &["hook", "install"]);
    let line = "jellymoon: CLAUDE_CONFIG_DIR must be an absolute path; nothing was changed\n";
    assert_eq!((ran.code, ran.stdout.as_str(), ran.stderr.as_str()), (1, "", line));
    assert_eq!(tree(w.dir.path()), before);
}

#[cfg(unix)]
#[test]
fn a_command_that_cannot_be_run_or_that_others_can_change_is_never_named_by_the_hook() {
    use std::os::unix::fs::symlink;

    use super::chmod;
    type Case = (&'static str, &'static str, fn(&World));
    let cases: [Case; 7] = [
        ("a text file that cannot be run", "it cannot be run", |w| {
            fs::write(w.bin().join(name()), "not a program").unwrap();
            chmod(&w.bin().join(name()), 0o644);
        }),
        ("a program that cannot be run", "it cannot be run", |w| chmod(&program(&w.bin(), &name()), 0o644)),
        ("a program anyone can write to", "others can write to it", |w| chmod(&program(&w.bin(), &name()), 0o777)),
        ("a program its group can write to", "others can write to it", |w| chmod(&program(&w.bin(), &name()), 0o775)),
        ("a folder anyone can write to", "everyone can write to its folder", |w| {
            program(&w.bin(), &name());
            chmod(&w.bin(), 0o777);
        }),
        ("a link to another program", "it is a link to another program", |w| {
            let other = program(&w.dir.path().join("other"), "another-program");
            symlink(other, w.bin().join(name())).unwrap();
        }),
        ("a link to a program in a folder anyone can write to", "everyone can write to its folder", |w| {
            let open = w.dir.path().join("open");
            symlink(program(&open, &name()), w.bin().join(name())).unwrap();
            chmod(&open, 0o777);
        }),
    ];
    for (case, why, make) in cases {
        let w = world();
        fs::create_dir_all(w.bin()).unwrap();
        make(&w);
        w.refused(&[], &format!("{NOT_USED}{why}"), case);
    }
    // A link to a sound `jellymoon` elsewhere is taken, and the entry names the link the install made.
    let w = world();
    fs::create_dir_all(w.bin()).unwrap();
    symlink(program(&w.dir.path().join("cellar"), &name()), w.bin().join(name())).unwrap();
    assert_eq!(w.turn_on(&[]).stdout, "{\"installed\":true,\"changed\":true}\n");
    assert!(fs::read_to_string(w.settings()).unwrap().contains(w.bin().join(name()).to_str().unwrap()));
}
