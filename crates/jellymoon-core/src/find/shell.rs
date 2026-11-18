//! The person's login shell, asked for its `PATH` when the harness is in no listed place (ADR 0021, decision 1;
//! `docs/development/page.md`, "The login shell"). Here is how an ask can end and the fixed words for each end;
//! the ask itself is `shell/ask.rs`, on a Mac and on Linux. Windows asks nothing.

#[cfg(unix)]
mod ask;

#[cfg(unix)]
pub(super) use ask::folders;

/// How the ask of the login shell ended. Every end but the first names no folder, and each has fixed words.
#[allow(dead_code)] // Windows makes `NotHere` alone, and no other system makes it
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Shell {
    /// It named its folders. A harness that passes the checks was in none of them, or this would not be told.
    Answered,
    /// `JELLYMOON_SHELL` is set to nothing: the person's word that no login shell is asked.
    Off,
    /// This system has no login shell to ask.
    NotHere,
    /// The system's record of the user could not be read, or named none.
    NoRecord,
    NotFull,
    NotAProgram,
    Open,
    Unlisted,
    NotStarted,
    TooLong,
    NoMarks,
    TwoPairs,
}

impl Shell {
    /// Why no folder came of a login shell that was to be asked. None when it answered, or was not to be asked.
    pub(crate) fn why(self) -> Option<&'static str> {
        Some(match self {
            Shell::Answered | Shell::Off | Shell::NotHere => return None,
            Shell::NoRecord => "the system's record of the user named none",
            Shell::NotFull => "it is not named by a full path",
            Shell::NotAProgram => "it is not a file that can be run",
            Shell::Open => "others can write to it, or everyone can write to a folder it is in",
            Shell::Unlisted => "it is not zsh or bash",
            Shell::NotStarted => "it could not be started",
            Shell::TooLong => "it did not answer in 5 seconds",
            Shell::NoMarks => "it answered with no pair of marks on one line",
            Shell::TwoPairs => "it answered with more than one pair of marks",
        })
    }

    /// The login shell as one of the places looked in, when it was one: with why, when no folder came of it.
    pub(crate) fn place(self) -> Option<String> {
        match self {
            Shell::Answered => Some("the login shell's PATH".to_string()),
            refused => refused.why().map(|why| format!("the login shell ({why})")),
        }
    }
}

/// On Windows there is no login shell to ask.
#[cfg(not(unix))]
pub(super) fn folders() -> Result<Vec<std::path::PathBuf>, Shell> {
    Err(Shell::NotHere)
}

#[cfg(test)]
mod tests {
    use super::Shell::{self, *};

    const ALL: [Shell; 12] = [
        Answered,
        Off,
        NotHere,
        NoRecord,
        NotFull,
        NotAProgram,
        Open,
        Unlisted,
        NotStarted,
        TooLong,
        NoMarks,
        TwoPairs,
    ];

    // On every system: a login shell that was not to be asked is no place looked in, so the line of a search
    // that found no harness is then the line it was before there was one to ask; every other end is a place.
    #[test]
    fn a_login_shell_that_was_not_to_be_asked_is_no_place_looked_in_and_every_other_end_is_one() {
        for shell in ALL {
            let place = shell.place();
            match shell {
                Off | NotHere => assert_eq!((place, shell.why()), (None, None)),
                Answered => assert_eq!((place.as_deref(), shell.why()), (Some("the login shell's PATH"), None)),
                refused => {
                    let why = refused.why().unwrap();
                    assert_eq!(place, Some(format!("the login shell ({why})")));
                    assert!(!why.contains('/') && !why.contains('\n'), "{why}");
                }
            }
        }
        let mut whys: Vec<&str> = ALL.iter().filter_map(|shell| shell.why()).collect();
        whys.sort_unstable();
        whys.dedup();
        assert_eq!(whys.len(), 9, "each refusal has words of its own");
    }
}
