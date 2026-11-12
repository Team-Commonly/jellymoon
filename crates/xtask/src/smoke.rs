//! `cargo xtask smoke`: builds the real programs once, then runs them together in a fresh temporary home, as a chain
//! of named steps (`smoke/steps.rs`). Each step is a plain program run whose exit code and JSON output are checked;
//! the first one that fails ends the run with its name and what the program printed. Nothing here runs `sh`.
//!
//! By default every log is made up and the harness is the made-up `fake-claude`. `--real` runs the same steps over
//! the person's own logs and their own Claude Code, and then prints counts, states and exit codes only. The `run`
//! step (`smoke/run.rs`) then starts their Claude Code on a harmless wish, in a repo the chain makes.

mod copy;
mod run;
mod steps;
mod stop;

use std::ffi::OsString;
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

#[derive(Default)]
pub struct Options {
    window: bool,
    real: bool,
    skip: String,
    build: Vec<String>,
}

impl Options {
    /// Reads what follows `smoke`. None when it is not a smoke command line; `--skip` is the reader's and needs `--real`.
    pub fn parse(args: &[String]) -> Option<Self> {
        let (mut options, mut args) = (Self::default(), args.iter());
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--window" => options.window = true,
                "--real" => options.real = true,
                "--skip" => options.skip = args.next()?.clone(),
                "--" => options.build = args.by_ref().cloned().collect(),
                _ => return None,
            }
        }
        (options.real || options.skip.is_empty()).then_some(options)
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
    let home = target.join(format!("smoke-home-{}", process::id()));
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
    println!("smoke: ok, {} steps in {:.2} s", done?.len(), started.elapsed().as_secs_f32());
    Ok(())
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
    use super::{Only, Options, steps::STEPS};

    fn parse(args: &[&str]) -> Option<Options> {
        Options::parse(&args.iter().map(|arg| (*arg).to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn the_command_line_takes_the_modes_and_passes_what_follows_two_dashes_to_the_build() {
        let options = parse(&["--window", "--real", "--skip", "notes", "--", "--features", "x/y"]);
        assert!(options.is_some_and(|o| o.window && o.real && o.skip == "notes" && o.build == ["--features", "x/y"]));
        assert!(parse(&[]).is_some_and(|o| !o.window && !o.real && o.build.is_empty()));
        for wrong in [&["--skip", "notes"][..], &["--real", "--skip"], &["--fast"], &["check"]] {
            assert!(parse(wrong).is_none(), "{wrong:?}");
        }
    }

    #[test]
    fn every_step_has_its_own_name_and_the_real_chain_never_uninstalls() {
        let mut names: Vec<&str> = STEPS.iter().map(|step| step.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), STEPS.len());
        let real = |name: &str| STEPS.iter().any(|step| step.name == name && matches!(step.only, Only::Any));
        assert!(real("day") && real("page") && real("run") && !real("logs") && !real("uninstall"));
    }
}
