//! What the kit's tests start as a trial's child: the kit's own, and the made-up ones a trial has to survive.
//! Never shipped. Its first argument says which:
//!
//! - `trial`: the kit's child, as a caller's program is one.
//! - `counted <file>`: the same, after a byte is added to the file: one for every start.
//! - `bare`: the same, when it was given nothing of its caller's: no `PATH`, and no argument after its own.
//! - `unruled`: draws the picture with the renderer and holds it to no written rule, so that what a trial does
//!   with a picture the rules would have refused first can be seen.
//! - `overflows`, `fails`, `garbage`: a child that ends on a stack too deep, ends with a code after a good answer,
//!   and answers with what is no answer.
//! - `never <file>`, `floods <file>`: a child that never ends, and one that never stops writing. Each holds a lock
//!   on the file for as long as it lives, and leaves `<file>.at` once it has it.

use std::io::{Read, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut arguments = std::env::args().skip(1);
    let which = arguments.next().unwrap_or_default();
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
        "bare" => {
            // A system may set a variable of its own in every program it starts; `PATH` it does not.
            if std::env::var_os("PATH").is_some() || arguments.next().is_some() {
                return ExitCode::from(4);
            }
            sticker_kit::theme::stand_trial("bare");
        }
        "unruled" => return unruled(),
        "overflows" => println!("accepted {} 0 1", deeper(1)),
        "fails" => {
            println!("accepted 0 0 1");
            return ExitCode::from(3);
        }
        "garbage" => {
            print!("{}", "accepted 0 0 1 and more\u{7}\n".repeat(64));
            return ExitCode::SUCCESS;
        }
        "never" | "floods" => {
            let file = std::path::PathBuf::from(arguments.next().unwrap_or_default());
            let held = std::fs::File::create(&file).and_then(|held| held.lock().map(|()| held));
            if held.is_err() || std::fs::write(file.with_extension("at"), "").is_err() {
                return ExitCode::from(9);
            }
            // A child that floods goes on when nobody reads, as one that means harm would.
            loop {
                if which == "never" || std::io::stdout().write_all(&[b'a'; 4096]).is_err() {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            }
        }
        _ => {}
    }
    ExitCode::from(2)
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
    let renderer = gpui::SvgRenderer::new(std::sync::Arc::new(()));
    match renderer.render_single_frame(&svg, 1.0) {
        Ok(_) => println!("accepted 0 0 1"),
        Err(error) => println!("refused: the renderer does not draw it: {error}"),
    }
    ExitCode::SUCCESS
}

#[cfg(not(feature = "gpui"))]
fn unruled() -> ExitCode {
    let _ = std::io::stdin().read_to_end(&mut Vec::new());
    ExitCode::from(2)
}
