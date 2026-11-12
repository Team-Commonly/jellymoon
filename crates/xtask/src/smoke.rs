//! `cargo xtask smoke`: builds the real programs once, then runs them together in a fresh temporary home, as a chain
//! of named steps (`smoke/steps.rs`). Each step is a plain program run whose exit code and JSON output are checked;
//! the first one that fails ends the run with its name and what the program printed. Nothing here runs `sh`.
//!
//! By default every log is made up and the harness is the made-up `fake-claude`. `--real` runs the same steps over
//! the person's own logs and their own Claude Code, and then prints counts, states and exit codes only. The `run`
//! step (`smoke/run.rs`) then starts their Claude Code on a harmless wish, in a repo the chain makes.

mod apart;
mod copy;
mod off_path;
mod run;
mod steps;
mod stop;

use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::path::{self, Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};
use std::{env, fs, process, thread};

use serde_json::Value;

/// The made-up day, and the zone it is read in: UTC-7 on that day.
const DAY: &str = "2026-03-10";
const ZONE: &str = "America/Los_Angeles";

/// How much of what a failed program printed is shown, in characters.
const SHOWN: usize = 2000;

/// How the build arguments that put the programs outside `target/debug` start.
const ELSEWHERE: [&str; 3] = ["--release", "--target", "--profile"];

#[derive(Default)]
pub struct Options {
    window: bool,
    real: bool,
    skip: String,
    build: Vec<String>,
}

impl Options {
    /// Reads what follows `smoke`. `Err(None)` when it is not a smoke command line; `--skip` is the reader's and needs
    /// `--real`. `Err` with a line for a build argument that puts the programs where the chain does not look.
    pub fn parse(args: &[String]) -> Result<Self, Option<String>> {
        let (mut options, mut args) = (Self::default(), args.iter());
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--window" => options.window = true,
                "--real" => options.real = true,
                "--skip" => options.skip = args.next().ok_or(None)?.clone(),
                "--" => options.build = args.by_ref().cloned().collect(),
                _ => return Err(None),
            }
        }
        let elsewhere = |arg: &&String| ELSEWHERE.iter().any(|flag| arg.starts_with(flag)) || *arg == "-r";
        if let Some(arg) = options.build.iter().find(elsewhere) {
            let why = format!(
                "`{arg}` is not taken after `--`: the chain runs the programs of the debug build in the \
                               target folder, and with it cargo builds them somewhere else"
            );
            return Err(Some(why));
        }
        if options.real || options.skip.is_empty() { Ok(options) } else { Err(None) }
    }
}

/// When a step runs.
pub enum Only {
    /// In every run.
    Any,
    /// Not with `--real`: it needs the made-up logs, or would touch what is the person's.
    Synthetic,
    /// With `--window` only.
    Window,
}

/// One link of the chain. `run` returns what the step's line says after its time: counts and states, never page text.
pub struct Step {
    pub name: &'static str,
    pub only: Only,
    pub run: fn(&World) -> Result<String, String>,
}

/// Where a run happens: the temporary home everything is written in, and the folder the built programs are in.
pub struct World {
    home: PathBuf,
    bin: PathBuf,
    pub real: bool,
    skip: String,
}

pub struct Ran {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Builds, then runs the chain. The error starts with the name of the step that failed.
pub fn run(root: &Path, options: &Options) -> Result<(), String> {
    let cargo = env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    let mut build = Command::new(cargo);
    build.current_dir(root).args(["build", "-p", "jellymoon", "-p", "jellymoon-app"]).args(&options.build);
    if !build.status().is_ok_and(|status| status.success()) {
        return Err("build: cargo build failed".to_string());
    }
    let target = env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from);
    let target = path::absolute(target).map_err(|e| format!("build: the target folder: {e}"))?;
    sweep(&target);
    let home = target.join(format!("smoke-home-{}", process::id()));
    let held = lock_beside(&home).ok_or("home: the lock beside the temporary home could not be taken")?;
    let world = World { home, bin: target.join("debug"), real: options.real, skip: options.skip.clone() };
    let _ = fs::remove_dir_all(&world.home);
    fs::create_dir_all(&world.home).map_err(|e| format!("home: {e}"))?;
    let started = Instant::now();
    let chain = steps::STEPS.iter().filter(|step| match step.only {
        Only::Any => true,
        Only::Synthetic => !options.real,
        Only::Window => options.window,
    });
    let done = chain.map(|step| one(&world, step)).collect::<Result<Vec<()>, String>>();
    let _ = fs::remove_dir_all(&world.home);
    drop(held);
    let _ = fs::remove_file(world.home.with_extension("lock"));
    println!("smoke: ok, {} steps in {:.2} s", done?.len(), started.elapsed().as_secs_f32());
    Ok(())
}

/// The system's lock on the file beside a temporary home, `smoke-home-<pid>.lock`, which a run holds for as long as
/// it lives. None while another process holds it.
fn lock_beside(home: &Path) -> Option<File> {
    let lock = OpenOptions::new().write(true).create(true).truncate(false).open(home.with_extension("lock")).ok()?;
    lock.try_lock().ok().map(|()| lock)
}

/// Removes the temporary homes earlier runs left in the target folder: a run that was stopped never removed its
/// own, and with `--real` it holds the person's page. A home whose run still lives is left: that run holds its lock.
fn sweep(target: &Path) {
    let left = fs::read_dir(target).into_iter().flatten().flatten().map(|entry| entry.path());
    let homes = left.filter(|path| path.is_dir() && path.extension().is_none());
    for home in
        homes.filter(|path| path.file_name().is_some_and(|name| name.to_string_lossy().starts_with("smoke-home-")))
    {
        if let Some(held) = lock_beside(&home) {
            let _ = fs::remove_dir_all(&home);
            drop(held);
            let _ = fs::remove_file(home.with_extension("lock"));
        }
    }
}

fn one(world: &World, step: &Step) -> Result<(), String> {
    let started = Instant::now();
    let note = (step.run)(world).map_err(|why| format!("{}: {why}", step.name))?;
    println!("smoke: {:<15} ok {:>6} ms  {note}", step.name, started.elapsed().as_millis());
    Ok(())
}

impl World {
    /// A path in the temporary home.
    pub fn at(&self, name: &str) -> PathBuf {
        self.home.join(name)
    }

    /// A built program.
    pub fn program(&self, name: &str) -> PathBuf {
        self.bin.join(format!("{name}{}", env::consts::EXE_SUFFIX))
    }

    /// The made-up harness's place in the temporary home.
    pub fn harness(&self) -> PathBuf {
        self.at("bin").join(format!("claude{}", env::consts::EXE_SUFFIX))
    }

    /// `jellymoon <command>` over the day: the made-up one, or with `--real` today in the person's own logs.
    pub fn day(&self, command: &str) -> Vec<String> {
        let args: &[&str] = match (self.real, self.skip.is_empty()) {
            (false, _) => &[command, "--date", DAY, "--logs", &text(&self.at("logs"))],
            (true, true) => &[command],
            (true, false) => &[command, "--skip", &self.skip],
        };
        args.iter().map(|arg| (*arg).to_string()).collect()
    }

    /// Runs a built program and returns what it printed. Whatever Jellymoon writes goes to the temporary home. A
    /// made-up run sees no variable of this process and no harness but the made-up one.
    pub fn run(&self, program: &str, args: &[String]) -> Result<Ran, String> {
        self.fed(program, args, "")
    }

    /// The same, with `input` on the program's standard input.
    pub fn fed(&self, program: &str, args: &[String], input: &str) -> Result<Ran, String> {
        let given = self.at("stdin");
        write(&given, input)?;
        let mut command = Command::new(self.program(program));
        command.stdin(fs::File::open(&given).map_err(|e| format!("{}: {e}", given.display()))?);
        command.args(args);
        if !self.real {
            command.env_clear().env("TZ", ZONE).env("PATH", self.at("bin")).env("JELLYMOON_CLAUDE", self.harness());
            command.env("HOME", self.at("user")).env("USERPROFILE", self.at("user"));
            command.env("CLAUDE_CONFIG_DIR", self.at("harness")); // where the delivery hook's settings are
            command.env("JELLYMOON_SYSTEM_BIN", ""); // no folder of this machine is looked in for a harness
            if let Some(root) = env::var_os("SystemRoot").filter(|_| cfg!(windows)) {
                command.env("SystemRoot", root); // a Windows program needs it to start
            }
        }
        command.env("JELLYMOON_HOME", self.at("jm"));
        command.env("JELLYMOON_APP_DIR", self.at("apps")).env("JELLYMOON_BIN_DIR", self.at("installed"));
        // The reading call may take three minutes with a real harness; a made-up run takes well under a second.
        self.wait(command, Duration::from_secs(if self.real { 200 } else { 20 }))
    }

    /// Runs the command to its end, or stops it at the limit. Output goes to files, so a full pipe never holds it up.
    pub fn wait(&self, mut command: Command, limit: Duration) -> Result<Ran, String> {
        let (out, err) = (self.at("stdout"), self.at("stderr"));
        let file = |path: &Path| fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()));
        command.stdout(file(&out)?).stderr(file(&err)?);
        let mut child = command.spawn().map_err(|e| format!("starting {:?}: {e}", command.get_program()))?;
        let deadline = Instant::now() + limit;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
                Ok(None) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("still running after {} seconds", limit.as_secs()));
                }
                Err(e) => return Err(format!("waiting: {e}")),
            }
        };
        let read = |path: &Path| String::from_utf8_lossy(&fs::read(path).unwrap_or_default()).into_owned();
        Ok(Ran { code: status.code().unwrap_or(-1), stdout: read(&out), stderr: read(&err) })
    }

    /// Runs a program that must exit 0 and print JSON, one value a line.
    pub fn lines(&self, program: &str, args: &[String]) -> Result<Vec<Value>, String> {
        let ran = self.run(program, args)?;
        let lines: Result<Vec<Value>, _> = ran.stdout.lines().map(serde_json::from_str).collect();
        match lines {
            Ok(lines) if ran.code == 0 => Ok(lines),
            _ => Err(self.printed(&ran)),
        }
    }

    /// Runs a program that must exit 0 and print one line of JSON.
    pub fn json(&self, program: &str, args: &[String]) -> Result<Value, String> {
        let mut lines = self.lines(program, args)?;
        if lines.len() == 1 { Ok(lines.remove(0)) } else { Err(format!("{} lines of JSON, not one", lines.len())) }
    }

    /// What a program printed, for a failure. With `--real` it may quote the person's page, so only its size is said.
    pub fn printed(&self, ran: &Ran) -> String {
        if self.real {
            let (out, err) = (ran.stdout.len(), ran.stderr.len());
            return format!("exit {}, {out} bytes on stdout and {err} on stderr (not shown with --real)", ran.code);
        }
        let cut = |text: &str| text.chars().take(SHOWN).collect::<String>();
        format!("exit {}\nstdout: {}\nstderr: {}", ran.code, cut(&ran.stdout), cut(&ran.stderr))
    }

    /// A check only the made-up run can make, since it knows what the day holds.
    pub fn made_up(&self, holds: bool, why: impl FnOnce() -> String) -> Result<(), String> {
        if self.real || holds { Ok(()) } else { Err(why()) }
    }
}

/// A path as an argument.
pub fn text(path: &Path) -> String {
    path.display().to_string()
}

/// Writes a file and the folders above it.
pub fn write(path: &Path, text: &str) -> Result<(), String> {
    let wrote = path.parent().map_or(Ok(()), fs::create_dir_all).and_then(|()| fs::write(path, text));
    wrote.map_err(|e| format!("writing {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{Only, Options, lock_beside, steps::STEPS, sweep};

    fn parse(args: &[&str]) -> Result<Options, Option<String>> {
        Options::parse(&args.iter().map(|arg| (*arg).to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn the_command_line_takes_the_modes_and_passes_what_follows_two_dashes_to_the_build() {
        let options = parse(&["--window", "--real", "--skip", "notes", "--", "--features", "x/y"]);
        assert!(options.is_ok_and(|o| o.window && o.real && o.skip == "notes" && o.build == ["--features", "x/y"]));
        assert!(parse(&[]).is_ok_and(|o| !o.window && !o.real && o.build.is_empty()));
        for wrong in [&["--skip", "notes"][..], &["--real", "--skip"], &["--fast"], &["check"], &["--release"]] {
            assert!(matches!(parse(wrong), Err(None)), "{wrong:?}");
        }
    }

    // The chain runs what is in `target/debug`: a build that goes elsewhere would leave it running older programs.
    #[test]
    fn a_build_argument_that_puts_the_programs_elsewhere_is_refused_in_one_line() {
        for flag in ["--release", "-r", "--target", "--target=x86_64-apple-darwin", "--target-dir", "--profile"] {
            let Err(Some(why)) = parse(&["--window", "--", "--features", "x/y", flag, "more"]) else {
                panic!("{flag} was taken");
            };
            assert!(why.starts_with(&format!("`{flag}` is not taken after `--`: ")) && !why.contains('\n'), "{why}");
        }
        assert!(parse(&["--", "--features", "release", "--locked"]).is_ok());
    }

    #[test]
    fn the_homes_earlier_runs_left_are_removed_and_the_home_of_a_run_that_lives_is_not() {
        let target = tempfile::tempdir().unwrap();
        let at = |name: &str| target.path().join(name);
        for home in ["smoke-home-1", "smoke-home-2", "smoke-home-3", "debug"] {
            fs::create_dir_all(at(home).join("jm")).unwrap();
            fs::write(at(home).join("jm").join("page.json"), "{}").unwrap();
        }
        fs::write(at("smoke-home-1.lock"), "").unwrap();
        fs::write(at("smoke-home-9"), "a file").unwrap();
        let living = lock_beside(&at("smoke-home-2")).unwrap();
        sweep(target.path());
        let mut left: Vec<String> = fs::read_dir(target.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        left.sort();
        assert_eq!(left, ["debug", "smoke-home-2", "smoke-home-2.lock", "smoke-home-9"]);
        drop(living);
        sweep(target.path());
        assert!(!at("smoke-home-2").exists() && !at("smoke-home-2.lock").exists() && at("debug").is_dir());
    }

    #[test]
    fn every_step_has_its_own_name_and_the_real_chain_never_uninstalls() {
        let mut names: Vec<&str> = STEPS.iter().map(|step| step.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), STEPS.len());
        // Every step that runs with `--real`, over the person's own day. A step added to this list is one a
        // maintainer has read for what it writes and prints there.
        let real = STEPS.iter().filter(|step| matches!(step.only, Only::Any)).map(|step| step.name);
        assert_eq!(real.collect::<Vec<_>>(), ["day", "page", "layout-sticker", "layout-plain", "broken-page", "run"]);
    }
}
