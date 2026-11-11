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
use crate::call::run;
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
const OFF: &[&str] =
    &["core.fsmonitor=false", "commit.gpgSign=false", "submodule.recurse=false", "protocol.allow=never"];

/// How much of what git printed is kept in an error, in characters.
const SAID_MAX: usize = 300;

/// How long one git child may run. One still running then is killed, and the command it was for fails.
const LIMIT: Duration = Duration::from_secs(300);

/// The limit, shorter for a test: the variable is read only by a build with debug assertions, which a release is not.
fn limit() -> Duration {
    let short = env::var("JELLYMOON_TEST_GIT_LIMIT_MS").ok().filter(|_| cfg!(debug_assertions));
    short.and_then(|ms| ms.parse().ok()).map_or(LIMIT, Duration::from_millis)
}

/// The `git` program, the empty folder it is started in and told to look for hooks in, and its time limit.
pub(super) struct Git {
    program: PathBuf,
    empty: PathBuf,
    limit: Duration,
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
    pub fn find(runs: &Path) -> Result<Git, String> {
        let name = format!("git{}", env::consts::EXE_SUFFIX);
        let folders = env::var_os("PATH").unwrap_or_default();
        let on_path = env::split_paths(&folders).filter(|folder| folder.is_absolute()).map(|folder| folder.join(&name));
        let program = on_path.into_iter().find(|program| program.is_file());
        let program = program.ok_or("no `git` program was found on PATH; a run's copy is made with git")?;
        let empty = runs.join("no-hooks");
        let held = fs::create_dir_all(&empty).and_then(|()| fs::read_dir(&empty)).map(|mut held| held.next().is_some());
        match held {
            Ok(false) => Ok(Git { program, empty, limit: limit() }),
            Ok(true) => Err("the folder Jellymoon keeps empty for git (runs/no-hooks) holds something".to_string()),
            Err(e) => Err(format!("the folder Jellymoon keeps empty for git (runs/no-hooks): {e}")),
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command.current_dir(&self.empty).env_clear();
        command.envs(ENVIRONMENT.iter().filter_map(|name| Some((name, env::var_os(name)?))));
        command.env("GIT_TERMINAL_PROMPT", "0").env("GIT_NO_LAZY_FETCH", "1");
        command.arg("--no-pager").arg("-c").arg(joined("core.hooksPath=", &self.empty));
        command.args(OFF.iter().flat_map(|off| ["-c", off]));
        command
    }

    /// What git finds at a folder: whether the repo is bare, whether the folder is inside a checkout, the git
    /// folder of that checkout and the git folder the repo's checkouts share, the last two as full paths. git looks
    /// in that folder and no higher: a plain folder inside some other repo is not that repo.
    pub fn discover(&self, folder: &Path) -> Result<Vec<String>, String> {
        let mut command = self.command();
        command.envs(folder.parent().map(|above| ("GIT_CEILING_DIRECTORIES", above)));
        command.arg("-C").arg(folder).args(["rev-parse", "--path-format=absolute", "--is-bare-repository"]);
        command.args(["--is-inside-work-tree", "--git-dir", "--git-common-dir"]);
        Ok(printed(command, self.limit)?.unwrap_or_default().lines().map(str::to_string).collect())
    }

    /// Names a repo by its git folder and reads which filters its config holds, so each call can blank them: a
    /// filter is a program git starts for every file it adds or checks out. One whose name holds `=` cannot be
    /// named on a command line, and then git is not run at all.
    pub fn open(&self, dir: &Path) -> Result<Repo<'_>, String> {
        let mut repo = Repo { git: self, dir: dir.to_path_buf(), blanks: Vec::new() };
        let filters = r"^filter\..*\.(clean|smudge|process)$";
        let named = repo.ask(None, &[], &["config", "-z", "--name-only", "--get-regexp", filters])?.unwrap_or_default();
        for key in named.split('\0').filter(|key| !key.is_empty()) {
            let Some((filter, _)) = key.rsplit_once('.').filter(|_| !key.contains('=')) else {
                return Err(format!(
                    "the repo's git config names a filter that cannot be switched off: {}",
                    line(key, 80)
                ));
            };
            repo.blanks.extend([format!("{key}="), format!("{filter}.required=false")]);
        }
        Ok(repo)
    }
}

impl Repo<'_> {
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
        printed(command, self.git.limit)
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
}

fn joined(flag: &str, path: &Path) -> OsString {
    let mut joined = OsString::from(flag);
    joined.push(path);
    joined
}

/// What the command printed. A failure is the start of what git said, made one line. A git still running at `limit`
/// is killed, by the reading call's own runner: git is given nothing to read.
fn printed(command: Command, limit: Duration) -> Result<Option<String>, String> {
    let output = run(command, Vec::new(), limit).map_err(|e| match e {
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
        _ => Err(said),
    }
}
