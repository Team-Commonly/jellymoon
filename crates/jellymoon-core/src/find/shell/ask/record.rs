//! Which program is the person's login shell, when `JELLYMOON_SHELL` does not say: the system's record of the
//! user. The system keeps it where only its own resolver reads all of it (on a Mac, Directory Services and not
//! `/etc/passwd`), and Rust reaches that resolver only through `unsafe` or a dependency. So the system's own
//! program for it is asked, by its full path and with no variable of this process: `/usr/bin/id -P` on a Mac,
//! `/usr/bin/getent passwd <uid>` elsewhere. Both print the record as one line of fields parted by colons, with
//! the user's number third and the login shell last. No test starts either: every test names `JELLYMOON_SHELL`.

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use crate::call;

/// How long the system's program has to print the record.
const LIMIT: Duration = Duration::from_secs(2);
/// The most bytes read of it. A record is one short line.
const MOST: usize = 4096;

/// The login shell in the record of the user this process runs as. None when the program for it is not there,
/// fails, is too slow, or prints something that is not that user's record.
pub(super) fn shell() -> Option<PathBuf> {
    let user = rustix::process::getuid().as_raw();
    let mut command;
    if cfg!(target_os = "macos") {
        command = Command::new("/usr/bin/id");
        command.arg("-P");
    } else {
        command = Command::new("/usr/bin/getent");
        command.args(["passwd", &user.to_string()]);
    }
    command.env_clear().current_dir("/");
    let said = call::run_most(command, Vec::new(), LIMIT, None, MOST).ok().filter(|said| said.status.success())?;
    of(&said.stdout, user)
}

/// The login shell a record names: the last field of one whole line whose third field is that user's number, with
/// the seven fields `getent` prints or the ten `id -P` does.
fn of(record: &[u8], user: u32) -> Option<PathBuf> {
    let line = record.strip_suffix(b"\n").filter(|line| !line.contains(&b'\n') && !line.contains(&b'\r'))?;
    let fields: Vec<&[u8]> = line.split(|byte| *byte == b':').collect();
    let whose = std::str::from_utf8(fields.get(2)?).ok()?.parse::<u32>().ok()?;
    let shell = fields.last().filter(|shell| !shell.is_empty())?;
    (whose == user && [7, 10].contains(&fields.len())).then(|| PathBuf::from(OsStr::from_bytes(shell)))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Records typed by hand, in the two shapes. No record of this machine is read.
    #[test]
    fn the_login_shell_is_the_last_field_of_the_one_line_that_is_this_user_s_record() {
        let mac = "tester:********:501:20::0:0:A Tester:/var/tester:/bin/zsh\n";
        let linux = "tester:x:1000:1000:A Tester,,,:/var/tester:/usr/bin/bash\n";
        assert_eq!(of(mac.as_bytes(), 501), Some(PathBuf::from("/bin/zsh")));
        assert_eq!(of(linux.as_bytes(), 1000), Some(PathBuf::from("/usr/bin/bash")));
        // What it names is refused or not by the checks of a login shell, not here.
        assert_eq!(of(b"tester:x:1000:1000::/var/tester:fish\n", 1000), Some(PathBuf::from("fish")));
        let none: [(&str, u32); 8] = [
            (mac, 502),
            ("", 501),
            ("tester:x:1000:1000:A Tester:/var/tester:/bin/bash", 1000),
            ("tester:x:1000:1000:A Tester:/var/tester:\n", 1000),
            ("tester:x:1000:1000:A Tester:/var/tester:/bin/bash\nroot:x:0:0::/root:/bin/sh\n", 1000),
            ("tester:x:1000:/bin/bash\n", 1000),
            ("tester:x:many:1000:A Tester:/var/tester:/bin/bash\n", 1000),
            ("id: no such user\n", 501),
        ];
        for (record, user) in none {
            assert_eq!(of(record.as_bytes(), user), None, "{record:?}");
        }
    }
}
