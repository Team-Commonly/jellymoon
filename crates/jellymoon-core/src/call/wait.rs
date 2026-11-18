//! The wait on a started command: until it ends or its limit is reached, and what is done with one that is given
//! up on. It is apart from `call.rs` so that a wait that itself fails can be played with no process at all.

use std::path::Path;
use std::process::{Child, ExitStatus};
use std::time::{Duration, Instant};
use std::{io, thread};

use crate::Error;

/// What the wait asks of a started command: a `Child`, or a test's stand-in for one.
pub(super) trait Started {
    fn try_wait(&mut self) -> io::Result<Option<ExitStatus>>;
    fn kill(&mut self) -> io::Result<()>;
    fn wait(&mut self) -> io::Result<ExitStatus>;
}

impl Started for Child {
    fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        Child::try_wait(self)
    }

    fn kill(&mut self) -> io::Result<()> {
        Child::kill(self)
    }

    fn wait(&mut self) -> io::Result<ExitStatus> {
        Child::wait(self)
    }
}

/// How the started command ended. One still running at `limit` is given up on ([`Error::TooLong`]); with `from`,
/// a file of a test's, the limit is counted from when that file is there. A wait that itself fails says nothing
/// of how the command ends, so that one is given up on too, and the answer is [`Error::Wait`]: the command was
/// started, which [`Error::Call`] would deny. A command given up on is stopped and then waited for before this
/// returns, so the folder its caller made for it is never removed under a command that still runs.
pub(super) fn ended(child: &mut impl Started, limit: Duration, from: Option<&Path>) -> Result<ExitStatus, Error> {
    let mut deadline = Instant::now() + limit;
    loop {
        let given_up = match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {
                if from.is_some_and(|from| !from.exists()) {
                    deadline = Instant::now() + limit;
                }
                (Instant::now() >= deadline).then_some(Error::TooLong(limit))
            }
            Err(why) => Some(Error::Wait(why)),
        };
        if let Some(why) = given_up {
            let _ = child.kill();
            let _ = child.wait();
            return Err(why);
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A started command that cannot be waited for, and what was asked of it, in order.
    struct Lost(Vec<&'static str>);

    impl Started for Lost {
        fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
            self.0.push("asked");
            Err(io::Error::other("no child"))
        }

        fn kill(&mut self) -> io::Result<()> {
            self.0.push("stopped");
            Ok(())
        }

        fn wait(&mut self) -> io::Result<ExitStatus> {
            self.0.push("waited for");
            Err(io::Error::other("no child"))
        }
    }

    // The review of the second app notes, note 1. A real wait that fails needs a parent that ignores its
    // children's ends, which no test here can set up, so the command is a stand-in.
    #[test]
    fn a_started_command_that_cannot_be_waited_for_is_stopped_then_waited_for_and_is_not_said_as_never_started() {
        let mut lost = Lost(Vec::new());
        let ended = ended(&mut lost, Duration::from_secs(30), None);
        assert!(matches!(ended, Err(Error::Wait(_))), "{ended:?}");
        assert_eq!(lost.0, ["asked", "stopped", "waited for"]);
        let said = "the reading call was started, the wait for its end failed, and it was stopped: no child";
        assert_eq!(ended.unwrap_err().to_string(), said);
    }
}
