# Installing on a Mac

```sh
curl -fsSL https://raw.githubusercontent.com/Team-Commonly/jellymoon/main/install.sh | sh
```

`install.sh` is for the Mac only; on any other system it says so, fails, and installs nothing. It uses only what every
Mac has (`curl`, `shasum`, `tar`, `xattr`), never `sudo`, and edits no shell profile.

Before it downloads anything it refuses, with one line, a Mac older than macOS 11 (`MIN_MACOS` in `install.sh`, which
`bundle.sh` reads into the bundle's `Info.plist`) and a `jellymoon` in the bin folder that is a folder or a link to one.

1. It picks the archive for the Mac's chip, `jellymoon-macos-arm64.tar.gz` or `jellymoon-macos-x86_64.tar.gz`. A
   shell running under Rosetta on an Apple chip still gets `arm64`.
2. It downloads `SHA256SUMS` from the latest GitHub release, over https only, into a temp folder. When it does not
   list the archive, it stops with one line, "this release has no build for an Intel Mac yet; nothing was installed"
   (or "an Apple-chip Mac"), before it downloads the archive: an Intel Mac reads this while a release has only the
   Apple-chip archive (see Cutting a release).
3. It downloads the archive, hashes it and compares. A mismatch stops it with nothing installed. Nothing downloaded is
   unpacked or run before this.
4. It copies `Jellymoon.app` next to where it goes in `~/Applications` and `jellymoon` next to where it goes in
   `~/.local/bin`. The command is renamed over the old one. The app takes three steps, since a folder cannot be
   renamed over a folder: the old bundle is moved aside as `.Jellymoon.app.old` in the same folder, the new one is
   moved in, and the old one is deleted. Running it again is how to update.
5. It says in one line if `~/.local/bin` is not on `PATH`.
6. It opens the installed `Jellymoon.app` with macOS `open`, by the path it was installed at, and says so first in one
   line that keeps what opening does: the app shows today's page, read with one call through the person's own
   Claude Code. Then it prints the lines that are left to run: the app's program started from a terminal, if the
   opened app says Claude Code was not found, and `jellymoon uninstall`.

**Opening.** Only after everything is in place and the checksum has passed: a refused install, a failed download
and a mismatch never reach it. It is skipped when `JELLYMOON_NO_OPEN` is set to anything that is not empty, and the
closing lines then say the `open` command to run, as they did before the installer opened anything. If `open` is
not there or fails, the install still succeeded and the exit code is 0; the line "Could not open it from here; the
install itself is done." comes before the same `open` command to run by hand.

**An update.** The app may be running when `install.sh` runs again, and nothing in the script stops it: the new
bundle is moved in under the running program, which goes on with the code it had loaded. `open` on an app that is
running does not start a second copy: LaunchServices brings the running one to the front. So an update opens the
app the same way, and says one more line: "If it was already open, it stays the version you had until you quit it
and open it again." The script does not look for a running app, quit it or restart it. Quitting it is the person's
to do, and the next open after that is the new version.

The open window says it too, since a line in a terminal is not read after the install: within a minute or so of the
new bundle being in place, its page has the note "A newer Jellymoon is installed. Quit Jellymoon and open it again
to use it." as the last of its column, until the app is quit. The app reads the version in its own bundle's
`Info.plist` for that and does nothing else: it does not quit, restart or download ([app.md](app.md), "A newer
build is installed"). A window of a build from before this note says nothing, so the script's line stays.

A folder the script has to make (`~/Applications`, `~/.local`, `~/.local/bin`) is made for you alone to write, mode
0755, whatever the umask: the app turns delivery on only for a command nobody else could replace
([delivery.md](delivery.md)). A folder that was there is never changed. If the bin folder was there and its group or
everyone can write to it (and it is not sticky), one closing line says the app will not offer to turn delivery on
and gives the two ways out: `jellymoon hook install` from a terminal, or `chmod go-w` on the folder. Only the bin
folder is looked at; an open folder above it has the same effect and is not said.

The temp folder and anything half placed are removed on every exit, a failure or an interrupt included. An update
that stops between its two moves puts the old bundle back on that same exit; if the script was killed outright, the
next run puts it back before doing anything else. For the moment between the two moves there is no `Jellymoon.app`.

| Variable | What it sets | Default |
|---|---|---|
| `JELLYMOON_APP_DIR` | the folder `Jellymoon.app` goes in | `$HOME/Applications` |
| `JELLYMOON_BIN_DIR` | the folder `jellymoon` goes in | `$HOME/.local/bin` |
| `JELLYMOON_RELEASE_URL` | the folder the release is read from, `https://` or `file://` | the latest GitHub release |
| `JELLYMOON_NO_OPEN` | when set to anything not empty, install and do not open the app | unset: the app is opened |

**A release candidate.** The plain command reads `releases/latest`, which is the newest full release and never a
candidate. To install a candidate, name its tag's folder, and set the variable for `sh`, not for `curl`:

```sh
curl -fsSL https://raw.githubusercontent.com/Team-Commonly/jellymoon/main/install.sh | \
  JELLYMOON_RELEASE_URL=https://github.com/Team-Commonly/jellymoon/releases/download/<tag> sh
```

(`<tag>` is for instance `v0.2.0-rc1`.) Set before `curl`, the variable only reaches `curl`, which ignores it, and
the latest full release is installed.

**How the tests keep the real `open` away.** No test starts the real `open`. Every world that runs the installer
either sets `JELLYMOON_NO_OPEN` (what the test support's `Mac::command` does for every world that names a
release) or has a made-up `open` first on its `PATH` (`Mac::install_opening`, which records its arguments). The door
refuses a world that runs the installer, by `JELLYMOON_RELEASE_URL` or by an argument named `install.sh`, with
neither (`unguarded_installer` in `crates/jellymoon/tests/support/door/world.rs`).

**What the checksum proves.** That the archive is the one the release's `SHA256SUMS` describes: a cut-off or damaged
download is refused. It does not prove who made the release, since both files come from the same place. That rests on
https and on the GitHub repository; the release carries no Apple Developer signature and is not notarised.

**Why Gatekeeper does not ask.** Gatekeeper checks files that carry the quarantine flag, which a browser sets on what
it downloads and `curl` does not. The script never asks anyone to switch Gatekeeper off and never runs `spctl`. If a
quarantine flag is on what it unpacked, it takes the flag off those files only. The app and the binary are signed ad
hoc (`codesign --sign -`), which an Apple chip needs before it starts any program. Someone who downloads the archive
with a browser instead gets the flag, and the Finder will refuse the app: use the script.

## Uninstall

```sh
jellymoon uninstall
```

It prints `{"removed":[...],"hook":false,"hook_by_hand":false}`: the paths it removed, whether it took the delivery
hook out of the harness's settings, and whether it left a hook entry the person had edited, which they then remove
by hand. It reads the same `JELLYMOON_APP_DIR` and `JELLYMOON_BIN_DIR`, removes `Jellymoon.app` and `jellymoon` from
them and nothing else, and takes no arguments. The journal's own files (`~/.jellymoon`) stay, and
the projects made with `jellymoon project new` are in that folder, in `projects`: clearing it as journal data
removes them. It is a command and not
a flag of the script because the hook lives in the harness's settings, which only core reads and writes, and because
the command is already on the machine and needs no network. The hook goes first, as `jellymoon hook uninstall`
removes it ([delivery.md](delivery.md)).

## Cutting a release

`.github/workflows/release.yml` runs when a tag `v<version>` is pushed. Its first step asks GitHub whether the tagged
commit is on `main` and builds nothing when it is not, so a tag pushed from a branch publishes nothing (this step has
not run on a tag yet). It builds `jellymoon` and `jellymoon-app` for
both chips on a Mac runner, with no `target` folder restored from an earlier run, and runs `app/macos/bundle.sh` (the
bundle, its `Info.plist`, the ad-hoc signatures, the archive), one job per chip. As soon as the Apple-chip archive is
built, a job writes `SHA256SUMS` with its one line, creates the release as a draft with both files attached, and only
then publishes it: an Apple-chip release never waits for the Intel build. A tag with a dash in it (`v0.2.0-rc1`) is
published as a pre-release. The Intel archive follows when it is built: a later job attaches it, and only then
replaces `SHA256SUMS` with one that names both archives. Until then an Intel Mac's installer says the release has no
build for it yet and installs nothing; while `SHA256SUMS` is being replaced, every installer says it could not
download it and installs nothing. `SHA256SUMS` never names an archive that is not attached. If the Intel build fails,
is cancelled or never gets a runner, the release stays Apple-chip only and the run's last job says so in a warning
(none of this has run yet). Run by hand from the Actions tab, from a branch or from a tag, it builds the same archives as
workflow artifacts and publishes nothing, which is the way to rehearse. `install.sh` reads `releases/latest`, which
GitHub points at the newest release that is neither a draft nor a pre-release.

## What was proved, and what was not

All of this is from the maintainers' own runs, on one Mac.

- **The release.** The release workflow ran on the tag `v0.1.0-rc1`. Both chips built, the Intel one cross-compiled
  on an Apple-chip runner, and it published a pre-release with two archives and `SHA256SUMS`.
- **The install.** `install.sh`, pointed at that release with `JELLYMOON_RELEASE_URL`, installed into scratch
  folders. A second run said "updated" and left no old bundle. The signature verified, nothing carried the quarantine
  flag, and the bundle's version was `0.1.0-rc1`. `jellymoon uninstall` then left both folders empty.
- **Opening the app.** The installed app, started through LaunchServices (`open`), which gives it no shell `PATH`,
  found Claude Code in one of the places `page.md` lists, read the day with one call and kept the page. Since F78,
  when those places hold no harness the app asks the login shell for its `PATH` (`page.md`, "The login shell"), and
  `jellymoon harness` says how the harness was found. That way is not proved on a person's Mac yet.

Not proved yet:

- **A Mac that has never seen the app.** That it opens with no Gatekeeper prompt there, only a person can see.
- **The installer's own `open` on a real Mac.** The tests use a made-up `open`; the real one is not started by any
  test, and has not been run by the maintainers since the installer began to call it.
- **A person's own double click.** `open` takes the same road, and nobody has yet clicked.
- **The Intel archive on an Intel Mac.** It was built and published, and never run.
- **An older macOS.** `LSMinimumSystemVersion` is 11.0, a guess: the app ran on one, current, macOS only.
- **No icon.** The repo holds none, so the bundle has the system's plain one.
