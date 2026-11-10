//! A made-up harness for the black-box tests. It leaves what it was given in the folder its own program file is in (the
//! one place a test can name, since `jellymoon` passes it no argument or variable of the test's): one file per argument,
//! then the folder it ran in, what was there, its environment and its input. It answers with the `reply` and the `exit`
//! code the test left in that folder.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{Read, Write};
use std::{env, fs, io, process};

fn main() {
    let rec = env::current_exe().unwrap().parent().unwrap().to_path_buf();
    for (n, arg) in env::args().skip(1).enumerate() {
        fs::write(rec.join(format!("arg{n}")), arg).unwrap();
    }
    let cwd = env::current_dir().unwrap();
    fs::write(rec.join("cwd"), cwd.to_string_lossy().as_bytes()).unwrap();
    let held: Vec<String> =
        fs::read_dir(&cwd).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    fs::write(rec.join("ls"), held.join("\n")).unwrap();
    let vars: Vec<String> =
        env::vars_os().map(|(k, v)| format!("{}={}", k.to_string_lossy(), v.to_string_lossy())).collect();
    fs::write(rec.join("env"), vars.join("\n")).unwrap();
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).unwrap();
    fs::write(rec.join("stdin"), input).unwrap();

    io::stdout().write_all(&fs::read(rec.join("reply")).unwrap()).unwrap();
    io::stdout().flush().unwrap();
    process::exit(fs::read_to_string(rec.join("exit")).unwrap().trim().parse().unwrap());
}
