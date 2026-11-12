//! A made-up asker for the black-box tests of a run started apart (F41). It has two parts, told apart by its first
//! argument. `ask <wish> <day> <home> <logs>` calls core's `run_apart` with this same program as the `jellymoon`
//! command, prints the run (or `none`) and the reason on a line each, and exits: an asker that is gone while its
//! run goes. Started with anything else, which is how core starts it, it starts the real `jellymoon`
//! (`FAKE_ASKER_JELLYMOON`) with the arguments and the output it was given itself, waits for it, and leaves that
//! command's exit code in the file `exit` of the folder `FAKE_ASKER_REC`. It kills the command when a file `cut` is
//! in that folder, which is how a test cuts off a run it has no other hold on. Two variables make it another
//! asker or another command: `FAKE_ASKER_PROGRAM` is given to core as the `jellymoon` command in place of this
//! program, and with `FAKE_ASKER_SAYS` the command says that on stderr, starts nothing and exits 1.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Duration;
use std::{env, fs, io, thread};

use jellymoon_core::{RunOptions, run_apart};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if let [ask, wish, day, home, logs] = &args[..]
        && ask == "ask"
    {
        let (logs, day) = (Some(PathBuf::from(logs)), Some(day.clone()));
        let options = RunOptions { wish: wish.clone(), day, logs, ..RunOptions::default() };
        let program = env::var_os("FAKE_ASKER_PROGRAM").map_or_else(|| env::current_exe().unwrap(), PathBuf::from);
        let said = match run_apart(Path::new(home), &program, &options) {
            Ok(started) => format!("run {}\n{}\n", started.run.as_deref().unwrap_or("none"), started.why),
            Err(e) => format!("refused\n{e}\n"),
        };
        io::stdout().write_all(said.as_bytes()).unwrap();
        return ExitCode::SUCCESS;
    }
    if let Some(says) = env::var_os("FAKE_ASKER_SAYS") {
        io::stderr().write_all(says.as_encoded_bytes()).unwrap();
        return ExitCode::from(1);
    }
    let rec = PathBuf::from(env::var_os("FAKE_ASKER_REC").unwrap());
    let mut command = Command::new(env::var_os("FAKE_ASKER_JELLYMOON").unwrap()).args(&args).spawn().unwrap();
    let status = loop {
        if let Some(status) = command.try_wait().unwrap() {
            break status;
        }
        if rec.join("cut").exists() {
            command.kill().unwrap();
            break command.wait().unwrap();
        }
        thread::sleep(Duration::from_millis(10));
    };
    // Written whole under another name first: a test that finds `exit` reads all of it.
    fs::write(rec.join("exit.new"), status.code().unwrap_or(-1).to_string()).unwrap();
    fs::rename(rec.join("exit.new"), rec.join("exit")).unwrap();
    ExitCode::SUCCESS
}
