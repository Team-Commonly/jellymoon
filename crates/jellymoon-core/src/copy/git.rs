//! The `git` child of a run's copy (`docs/development/copy.md`, "What is switched off"). Every call core makes goes
//! through [`Git::command`]: the program found on `PATH`, a short list of this process's variables, an empty folder of
//! Jellymoon's own to start in, and on the command line the settings that stop git starting a program of the repo's
//! or the person's config. After the one look at the person's folder in [`Git::discover`], git is always told the
//! repo, and the work tree when it needs one: it never finds either from the folder it is in.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use std::{env, fs};

use crate::Error;
use crate::busy::{or_test, test_file};
use crate::call::{full_folders, passed_on, run, run_most};
use crate::redact::line;

/// What git may see of this process's environment: where its programs and the person's config are, and the two
/// variables that say which config files are read. No `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE` or `GIT_CONFIG_*`
/// that names a setting gets through. The second line is what a Windows program needs.
const ENVIRONMENT: &[&str] = &[
    "PATH",
    "HOME",
    "TMPDIR",
    "XDG_CONFIG_HOME",
    "GIT_CONFIG_GLOBAL",
    "GIT_CONFIG_NOSYSTEM",
    "PATHEXT",
    "SystemRoot",
    "SystemDrive",
    "USERPROFILE",
    "HOMEDRIVE",
    "HOMEPATH",
    "APPDATA",
    "LOCALAPPDATA",
    "ProgramData",
    "TEMP",
    "TMP",
];

/// Settings given on every command line, where they win over every config file: no file-system monitor program, no
/// signing program, no command run in a submodule on checkout, and no transport, so no fetch of a missing object can
/// start ssh, a credential helper or a remote helper. `core.hooksPath` is given beside them, as an empty folder.
/// And a copy's `gitdir` file is written as a whole path, which is how a look knows the copy (`record::asked`).
const OFF: &[&str] = &[
    "core.fsmonitor=false",
    "commit.gpgSign=false",
    "submodule.recurse=false",
    "protocol.allow=never",
    "worktree.useRelativePaths=false",
];

/// How much of what git printed is kept in an error, in characters.
const SAID_MAX: usize = 300;

/// How long one git child may run. One still running then is killed, and the command it was for fails. A test's
/// build reads a shorter one from `JELLYMOON_TEST_GIT_LIMIT_MS`, counted from when the file that
/// `JELLYMOON_TEST_GIT_LIMIT_FROM` names is there, if it names one.
const LIMIT: Duration = Duration::from_secs(300);

/// What stands where git's own words would, for a repo whose folder was read from a session log.
const UNSAID: &str = "what git said of a folder read from a session log is not repeated";

/// The `git` program, the empty folder it is started in and told to look for hooks in, and its time limit.
pub(super) struct Git {
    program: PathBuf,
    empty: PathBuf,
    limit: Duration,
    /// The repo's folder was read from a session log: nothing git says of it, and no name from its config, is
    /// put in an error, since git's words name the folder.
    pub quiet: bool,
}

/// One repo, named to git by its git folder, with the filters its config names switched off.
pub(super) struct Repo<'a> {
    git: &'a Git,
    dir: PathBuf,
    blanks: Vec<String>,
}

impl Git {
    /// Finds `git` in the folders of `PATH` that are full paths (a relative one would be read from the folder git
    /// starts in), and makes the empty folder, under `runs`. A folder there that holds anything is refused: a file
    /// in it could be run as a hook.
    pub fn find(runs: &Path, quiet: bool) -> Result<Git, String> {
        let name = format!("git{}", env::consts::EXE_SUFFIX);
        let on_path = full_folders(&env::var_os("PATH").unwrap_or_default());
        let program = on_path.into_iter().map(|folder| folder.join(&name)).find(|program| program.is_file());
        let program = program.ok_or("no `git` program was found on PATH; a run's copy is made with git")?;
        let empty = runs.join("no-hooks");
        let held = fs::create_dir_all(&empty).and_then(|()| fs::read_dir(&empty)).map(|mut held| held.next().is_some());
        match held {
            Ok(false) => Ok(Git { program, empty, limit: or_test(LIMIT, "JELLYMOON_TEST_GIT_LIMIT_MS"), quiet }),
            Ok(true) => Err("the folder Jellymoon keeps empty for git (runs/no-hooks) holds something".to_string()),
            Err(e) => Err(format!("the folder Jellymoon keeps empty for git (runs/no-hooks): {e}")),
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command.current_dir(&self.empty).env_clear();
        command.envs(passed_on(ENVIRONMENT));
        command.env("GIT_TERMINAL_PROMPT", "0").env("GIT_NO_LAZY_FETCH", "1");
        command.arg("--no-pager").arg("-c").arg(joined("core.hooksPath=", &self.empty));
        command.args(OFF.iter().flat_map(|off| ["-c", off]));
        command
    }

    /// What git finds at a folder: whether the repo is bare, whether the folder is inside a checkout, where the
    /// folder is below the checkout's top (an empty line at the top), the git folder of that checkout and the git
    /// folder the repo's checkouts share, the last two as full paths. The ceiling stops git looking above the folder
    /// as it was named. git goes by the folder's real path, so under another name (a link, `sub/..`) it can still
    /// find a repo above: the caller refuses whatever is not a top.
    pub fn discover(&self, folder: &Path) -> Result<Vec<String>, String> {
        let mut command = self.command();
        command.envs(folder.parent().map(|above| ("GIT_CEILING_DIRECTORIES", above)));
        command.arg("-C").arg(folder).args(["rev-parse", "--path-format=absolute", "--is-bare-repository"]);
        command.args(["--is-inside-work-tree", "--show-prefix", "--git-dir", "--git-common-dir"]);
        Ok(printed(command, self)?.unwrap_or_default().lines().map(str::to_string).collect())
    }

    /// Names a repo by its git folder and reads which filters, merge drivers and transports its config names, so
    /// each call can switch them off ([`blanks`]).
    pub fn open(&self, dir: &Path) -> Result<Repo<'_>, String> {
        let mut repo = Repo { git: self, dir: dir.to_path_buf(), blanks: Vec::new() };
        let keys = r"^(filter\..*\.(clean|smudge|process)|protocol\..*\.allow|merge\..*\.driver)$";
        let named = repo.ask(None, &[], &["config", "-z", "--name-only", "--get-regexp", keys])?.unwrap_or_default();
        repo.blanks = blanks(&named).map_err(|named| if self.quiet { UNSAID.to_string() } else { named })?;
        Ok(repo)
    }
}

/// The settings that switch off what a repo's config names, from the names as `git config -z` prints them. A filter
/// is a program git starts for every file it adds or checks out: its command is set to nothing. A transport's own
/// `protocol.<name>.allow` wins over `protocol.allow`, so each one named is set to `never` itself. A merge driver
/// is a program git starts for a file both sides of a merge changed (`keep.rs`): its command is set to nothing, and
/// git then starts nothing and takes the file as one that clashes (measured on git 2.50.1). A name that holds `=`
/// cannot be given on a command line, and then git is not run at all.
fn blanks(named: &str) -> Result<Vec<String>, String> {
    let mut blanks = Vec::new();
    for key in named.split('\0').filter(|key| !key.is_empty()) {
        match key.rsplit_once('.').filter(|_| !key.contains('=')) {
            Some((_, "allow")) => blanks.push(format!("{key}=never")),
            Some((_, "driver")) => blanks.push(format!("{key}=")),
            Some((filter, _)) => blanks.extend([format!("{key}="), format!("{filter}.required=false")]),
            None => {
                let what = "the repo's git config names a filter or a transport that cannot be switched off";
                return Err(format!("{what}: {}", line(key, 80)));
            }
        }
    }
    Ok(blanks)
}

impl Repo<'_> {
    /// The same repo asked through the git folder of one of its worktrees, where `HEAD` is that worktree's.
    pub fn through(&self, dir: &Path) -> Repo<'_> {
        Repo { git: self.git, dir: dir.to_path_buf(), blanks: self.blanks.clone() }
    }

    /// Runs git on this repo, with `tree` as the work tree when the command reads or writes files, and returns
    /// what it printed with the last line break cut. None when git exited 1 and said nothing: its way of saying a
    /// setting or a name is not there.
    pub fn ask<S: AsRef<OsStr>>(
        &self,
        tree: Option<&Path>,
        vars: &[(&str, &str)],
        args: &[S],
    ) -> Result<Option<String>, String> {
        let mut command = self.git.command();
        command.args(self.blanks.iter().flat_map(|blank| ["-c", blank])).arg(joined("--git-dir=", &self.dir));
        command.args(tree.map(|tree| joined("--work-tree=", tree))).args(args).envs(vars.iter().copied());
        printed(command, self.git)
    }

    /// The same, for a command that must work.
    pub fn run<S: AsRef<OsStr>>(
        &self,
        tree: Option<&Path>,
        vars: &[(&str, &str)],
        args: &[S],
    ) -> Result<String, String> {
        self.ask(tree, vars, args)?.ok_or_else(|| "git failed and did not say why".to_string())
    }

    /// What git printed, as bytes, with `input` on its input: for what is not a line of text, and for what must
    /// not be read whole (`work.rs`). No more than `most` bytes are read. Fewer are all that git printed, and git
    /// must have worked; `most` of them say only that there was that much or more. An object is read as it is,
    /// whatever the repo names to stand in its place, and no word of git's is kept of a failure.
    pub fn bytes(&self, args: &[&str], input: &[u8], most: usize) -> Option<Vec<u8>> {
        let (code, printed) = self.answer(None, &[], args, input, most)?;
        (code == Some(0) || printed.len() >= most).then_some(printed)
    }

    /// The code git ended with and what it printed, read as [`Repo::bytes`] reads: for a step whose `1` is an
    /// answer (`keep.rs`), with `tree` as the work tree and `vars` beside git's own. No code is one of a git that
    /// a signal ended. None when git could not be started or was stopped at its limit.
    pub fn answer(
        &self,
        tree: Option<&Path>,
        vars: &[(&str, &str)],
        args: &[&str],
        input: &[u8],
        most: usize,
    ) -> Option<(Option<i32>, Vec<u8>)> {
        let mut command = self.git.command();
        command.args(self.blanks.iter().flat_map(|blank| ["-c", blank])).arg(joined("--git-dir=", &self.dir));
        command.args(tree.map(|tree| joined("--work-tree=", tree))).arg("--no-replace-objects");
        command.args(args).envs(vars.iter().copied());
        let from = test_file("JELLYMOON_TEST_GIT_LIMIT_FROM");
        let output = run_most(command, input.to_vec(), self.git.limit, from.as_deref(), most).ok()?;
        Some((output.status.code(), output.stdout))
    }
}

fn joined(flag: &str, path: &Path) -> OsString {
    let mut joined = OsString::from(flag);
    joined.push(path);
    joined
}

/// What the command printed. A failure is the start of what git said, made one line. A git still running at `limit`
/// is killed, by the reading call's own runner: git is given nothing to read.
fn printed(command: Command, git: &Git) -> Result<Option<String>, String> {
    let from = test_file("JELLYMOON_TEST_GIT_LIMIT_FROM");
    let output = run(command, Vec::new(), git.limit, from.as_deref()).map_err(|e| match e {
        Error::TooLong(limit) => format!("git did not finish in {} seconds and was stopped", limit.as_secs_f32()),
        e => format!("git could not be started: {e}"),
    })?;
    let said = line(&String::from_utf8_lossy(&output.stderr), SAID_MAX);
    match (output.status.success(), output.status.code()) {
        (true, _) => match String::from_utf8(output.stdout) {
            Ok(printed) => Ok(Some(printed.trim_end_matches(['\r', '\n']).to_string())),
            Err(_) => Err("git printed a name that is not text".to_string()),
        },
        (false, Some(1)) if said.is_empty() => Ok(None),
        _ if git.quiet => Err(UNSAID.to_string()),
        _ => Err(said),
    }
}

#[cfg(test)]
mod tests {
    use super::blanks;

    #[test]
    fn every_filter_and_every_transport_the_config_names_is_switched_off_by_its_name() {
        let named =
            "filter.lfs.clean\0protocol.file.allow\0filter.a.b.process\0protocol.Ext\nX.allow\0merge.m.driver\0";
        let off = [
            "filter.lfs.clean=",
            "filter.lfs.required=false",
            "protocol.file.allow=never",
            "filter.a.b.process=",
            "filter.a.b.required=false",
            "protocol.Ext\nX.allow=never",
            "merge.m.driver=",
        ];
        assert_eq!(blanks(named).unwrap(), off);
        assert_eq!(blanks("").unwrap(), Vec::<String>::new());
        for unnamed in ["protocol.a=b.allow", "filter.x=y.smudge"] {
            let said = blanks(&format!("filter.ok.clean\0{unnamed}\0")).unwrap_err();
            assert!(said.ends_with(&format!("cannot be switched off: {unnamed}")), "{said}");
        }
    }
}
