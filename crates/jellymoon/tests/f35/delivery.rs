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
type Case = (&'static str, &'static str, fn(&World));

/// Each case is made in a world of its own, and the install is refused in one line with nothing written.
#[cfg(unix)]
fn each_is_refused(cases: &[Case]) {
    for (case, why, make) in cases {
        let w = world();
        fs::create_dir_all(w.bin()).unwrap();
        make(&w);
        w.refused(&[], &format!("{NOT_USED}{why}"), case);
    }
}

#[cfg(unix)]
#[test]
fn a_command_that_is_a_link_or_cannot_be_run_or_that_others_can_change_is_never_named_by_the_hook() {
    use std::os::unix::fs::symlink;

    use super::chmod;
    each_is_refused(&[
        ("a text file that cannot be run", "it cannot be run", |w| {
            fs::write(w.bin().join(name()), "not a program").unwrap();
            chmod(&w.bin().join(name()), 0o644);
        }),
        ("a program that cannot be run", "it cannot be run", |w| chmod(&program(&w.bin(), &name()), 0o644)),
        ("a program anyone can write to", "others can write to it", |w| chmod(&program(&w.bin(), &name()), 0o777)),
        ("a program its group can write to", "others can write to it", |w| chmod(&program(&w.bin(), &name()), 0o775)),
        ("a setuid program", "it is setuid or setgid", |w| chmod(&program(&w.bin(), &name()), 0o4755)),
        ("a setgid program", "it is setuid or setgid", |w| chmod(&program(&w.bin(), &name()), 0o2755)),
        ("a link to another program", "it is a link", |w| {
            let other = program(&w.dir.path().join("other"), "another-program");
            symlink(other, w.bin().join(name())).unwrap();
        }),
        // The installer copies a plain file, so even a link to a sound `jellymoon` is not what it put there.
        ("a link to a sound jellymoon elsewhere", "it is a link", |w| {
            symlink(program(&w.dir.path().join("cellar"), &name()), w.bin().join(name())).unwrap();
        }),
    ]);
    // The owner of the command is compared with the owner of the person's home. With the root folder as the home,
    // a file of the test's own is someone else's (unless the tests run as the owner of the root folder).
    use std::os::unix::fs::MetadataExt;
    let w = world();
    program(&w.bin(), &name());
    if fs::metadata("/").unwrap().uid() != fs::metadata(w.bin()).unwrap().uid() {
        let (bin, config) = (w.bin(), w.settings().parent().unwrap().to_path_buf());
        let more = [
            ("HOME", "/"),
            ("JELLYMOON_BIN_DIR", bin.to_str().unwrap()),
            ("CLAUDE_CONFIG_DIR", config.to_str().unwrap()),
        ];
        w.refused(&more, &format!("{NOT_USED}it is not a file of your own"), "a home of another owner");
    }
}

// Whoever can write to a folder on the way to the command can put another program in its place.
#[cfg(unix)]
#[test]
fn a_folder_on_the_way_to_the_command_that_others_can_write_to_is_refused_unless_it_is_sticky() {
    use std::os::unix::fs::symlink;

    use super::chmod;
    let open = "others can write to a folder on the way to it";
    each_is_refused(&[
        ("a folder anyone can write to", open, |w| {
            program(&w.bin(), &name());
            chmod(&w.bin(), 0o777);
        }),
        ("a folder its group can write to", open, |w| {
            program(&w.bin(), &name());
            chmod(&w.bin(), 0o775);
        }),
        ("a folder above it that anyone can write to", open, |w| {
            program(&w.bin(), &name());
            chmod(w.bin().parent().unwrap(), 0o777);
        }),
        ("a folder that is a link into a folder anyone can write to", open, |w| {
            fs::remove_dir(w.bin()).unwrap();
            let real = w.dir.path().join("open").join("real");
            program(&real, &name());
            symlink(&real, w.bin()).unwrap();
            chmod(real.parent().unwrap(), 0o777);
        }),
        ("two links in a row whose middle folder anyone can write to", open, |w| {
            fs::remove_dir(w.bin()).unwrap();
            let (cellar, middle) = (w.dir.path().join("cellar"), w.dir.path().join("middle"));
            program(&cellar, &name());
            fs::create_dir(&middle).unwrap();
            symlink(&cellar, middle.join("hop")).unwrap();
            symlink(middle.join("hop"), w.bin()).unwrap();
            chmod(&middle, 0o777);
        }),
    ]);
    // What is still taken: a sticky folder above it (only a file's owner can replace it there), and a folder that is
    // a link to a sound one. The entry names the command as the install named it.
    let sound: [(&str, super::Spoil); 2] = [
        ("a sticky folder above it", |w| {
            program(&w.bin(), &name());
            chmod(w.bin().parent().unwrap(), 0o1777);
        }),
        ("a folder that is a link to a sound folder", |w| {
            fs::remove_dir(w.bin()).unwrap();
            let cellar = w.dir.path().join("cellar");
            program(&cellar, &name());
            symlink(&cellar, w.bin()).unwrap();
        }),
    ];
    for (case, make) in sound {
        let w = world();
        fs::create_dir_all(w.bin()).unwrap();
        make(&w);
        assert_eq!(w.turn_on(&[]).stdout, "{\"installed\":true,\"changed\":true}\n", "{case}");
        assert!(fs::read_to_string(w.settings()).unwrap().contains(w.bin().join(name()).to_str().unwrap()), "{case}");
    }
}
