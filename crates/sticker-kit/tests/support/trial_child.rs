//! What the kit's tests start as a trial's child: the kit's own, and the made-up ones a trial has to survive.
//! Never shipped. Its first argument says which. A trial adds its limit in milliseconds as the last argument.
//!
//! - `trial`: the kit's child, as a caller's program is one.
//! - `counted <file>`: the same, after a byte is added to the file: one for every start.
//! - `held <file>`: the same, holding a lock on the file for as long as it lives, with `<file>.at` left once it
//!   has it.
//! - `bare`: the same, when it was given nothing of its caller's: no `PATH`, and no argument but the limit.
//! - `parent <theme file> <file> <ms>`: a caller. It tries the theme's art with `held <file>` for the child and a
//!   limit of `<ms>`.
//! - `unruled`: draws the picture with the renderer, on a stack of the page's size, and holds it to no written
//!   rule and no cost, so that what a trial does with a picture the rules would have refused first can be seen.
//! - `overflows`, `fails`, `garbage`, `twice`, `unsure`: a child that ends on a stack too deep, ends with a code
//!   after a good answer, answers with what is no answer, answers twice, and says it could not try.
//! - `wordy`: writes eight megabytes and ends well. `deaf`: never reads what it is given. `over`: ends as a child
//!   its own clock ended.
//! - `alone <file>`: fails when another child holds the file's lock, and holds it itself for a while.
//! - `never <file>`, `floods <file>`: a child that never ends, and one that never stops writing. Each holds a lock
//!   on the file for as long as it lives, and leaves `<file>.at` once it has it.
//!
//! A made-up child that would never end has a clock of its own, so that a test of a caller that fails to end it
//! fails, and does not hang or leave a process behind.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

/// A good answer, as the kit's child writes one.
const GOOD: &str = "accepted 0 0 1 0 0";

/// How long a made-up child lives at most.
const OWN_CLOCK: Duration = Duration::from_secs(30);

fn main() -> ExitCode {
    let mut arguments = std::env::args().skip(1);
    let which = arguments.next().unwrap_or_default();
    std::thread::spawn(|| {
        std::thread::sleep(OWN_CLOCK);
        std::process::exit(7);
    });
    match which.as_str() {
        "trial" => sticker_kit::theme::stand_trial("trial"),
        "counted" => {
            let file = arguments.next().unwrap_or_default();
            let opened = std::fs::OpenOptions::new().create(true).append(true).open(file);
            if opened.and_then(|mut file| file.write_all(b"1")).is_err() {
                return ExitCode::from(9);
            }
            sticker_kit::theme::stand_trial("counted");
        }
        "held" => {
            let Some(_held) = held(arguments.next()) else { return ExitCode::from(9) };
            sticker_kit::theme::stand_trial("held");
        }
        "bare" => {
            // A system may set a variable of its own in every program it starts; `PATH` it does not.
            let limit = arguments.next().is_none_or(|limit| limit.parse::<u64>().is_ok());
            if std::env::var_os("PATH").is_some() || !limit || arguments.next().is_some() {
                return ExitCode::from(4);
            }
            sticker_kit::theme::stand_trial("bare");
        }
        "parent" => return parent(arguments),
        "unruled" => return unruled(),
        "deaf" => std::thread::sleep(OWN_CLOCK),
        "alone" => {
            let file = std::fs::File::create(arguments.next().unwrap_or_default());
            // Held by name until the process ends: a lock nobody holds on to is let go at once.
            let Ok(_held) = file.and_then(|file| Ok(file.try_lock().map(|()| file)?)) else {
                return ExitCode::from(5);
            };
            std::thread::sleep(Duration::from_millis(300));
            println!("{GOOD}");
            return ExitCode::SUCCESS;
        }
        "never" | "floods" => {
            let Some(_held) = held(arguments.next()) else { return ExitCode::from(9) };
            // A child that floods goes on when nobody reads, as one that means harm would.
            loop {
                if which == "never" || std::io::stdout().write_all(&[b'a'; 4096]).is_err() {
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        }
        _ => return made_up(&which),
    }
    ExitCode::from(2)
}

/// The made-up children that only say something and end.
fn made_up(which: &str) -> ExitCode {
    match which {
        "overflows" => println!("accepted {} 0 1 0 0", deeper(1)),
        "fails" => {
            println!("{GOOD}");
            return ExitCode::from(3);
        }
        "garbage" => {
            print!("{}", format!("{GOOD} and more\u{7}\n").repeat(64));
            return ExitCode::SUCCESS;
        }
        "twice" => {
            print!("{GOOD}\n{GOOD}\n");
            return ExitCode::SUCCESS;
        }
        "over" => return ExitCode::from(124),
        "unsure" => {
            println!("unsure: the made-up child could not try it");
            return ExitCode::SUCCESS;
        }
        "wordy" => {
            let wrote = std::io::stdout().write_all(&vec![b'a'; 8 * 1024 * 1024]);
            return if wrote.is_ok() { ExitCode::SUCCESS } else { ExitCode::from(9) };
        }
        _ => {}
    }
    ExitCode::from(2)
}

/// The lock on the file, held for as long as what is returned lives, and `<file>.at` left to say so.
fn held(file: Option<String>) -> Option<std::fs::File> {
    let file = PathBuf::from(file?);
    let held = std::fs::File::create(&file).and_then(|held| held.lock().map(|()| held)).ok()?;
    std::fs::write(file.with_extension("at"), "").ok()?;
    Some(held)
}

/// A caller that tries a theme's art with `held <file>` for its child.
fn parent(mut arguments: impl Iterator<Item = String>) -> ExitCode {
    let (theme, file, limit) = (arguments.next(), arguments.next(), arguments.next());
    let (Some(theme), Some(file), Some(Ok(limit))) = (theme, file, limit.map(|ms| ms.parse())) else {
        return ExitCode::from(9);
    };
    let Ok(loaded) = sticker_kit::Theme::load(std::path::Path::new(&theme)) else { return ExitCode::from(9) };
    let mut trial = sticker_kit::theme::Trial::new(std::env::current_exe().unwrap_or_default(), "held", None, "");
    trial.arguments.push(file);
    trial.limit = Duration::from_millis(limit);
    println!("{:?}", loaded.theme.try_art(&trial).refused);
    ExitCode::SUCCESS
}

/// Calls itself until the stack ends. The array is read back so that no build makes a loop of it.
#[allow(unconditional_recursion)]
fn deeper(depth: u64) -> u64 {
    let held = std::hint::black_box([depth; 64]);
    deeper(depth + 1) + held[(depth % 64) as usize]
}

#[cfg(feature = "gpui")]
fn unruled() -> ExitCode {
    let mut svg = Vec::new();
    if std::io::stdin().read_to_end(&mut svg).is_err() {
        return ExitCode::from(9);
    }
    let draw = move || {
        let renderer = gpui::SvgRenderer::new(std::sync::Arc::new(()));
        match renderer.render_single_frame(&svg, 1.0) {
            Ok(_) => println!("{GOOD}"),
            Err(error) => println!("refused: the renderer does not draw it: {error}"),
        }
    };
    let page = std::thread::Builder::new().stack_size(sticker_kit::theme::PAGE_STACK).spawn(draw);
    if page.is_ok_and(|page| page.join().is_ok()) { ExitCode::SUCCESS } else { ExitCode::from(9) }
}

#[cfg(not(feature = "gpui"))]
fn unruled() -> ExitCode {
    let _ = std::io::stdin().read_to_end(&mut Vec::new());
    ExitCode::from(2)
}
