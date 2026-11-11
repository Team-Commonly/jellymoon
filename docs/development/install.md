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
2. It downloads that archive and `SHA256SUMS` from the latest GitHub release, over https only, into a temp folder.
3. It hashes the archive and compares. A mismatch, or a `SHA256SUMS` that does not list the archive, stops it with
   nothing installed. Nothing downloaded is unpacked or run before this.
4. It copies `Jellymoon.app` next to where it goes in `~/Applications` and `jellymoon` next to where it goes in
   `~/.local/bin`. The command is renamed over the old one. The app takes three steps, since a folder cannot be
   renamed over a folder: the old bundle is moved aside as `.Jellymoon.app.old` in the same folder, the new one is
   moved in, and the old one is deleted. Running it again is how to update.
5. It says in one line if `~/.local/bin` is not on `PATH`, and prints what to run next.

The temp folder and anything half placed are removed on every exit, a failure or an interrupt included. An update
that stops between its two moves puts the old bundle back on that same exit; if the script was killed outright, the
next run puts it back before doing anything else. For the moment between the two moves there is no `Jellymoon.app`.

| Variable | What it sets | Default |
|---|---|---|
| `JELLYMOON_APP_DIR` | the folder `Jellymoon.app` goes in | `$HOME/Applications` |
| `JELLYMOON_BIN_DIR` | the folder `jellymoon` goes in | `$HOME/.local/bin` |
| `JELLYMOON_RELEASE_URL` | the folder the release is read from, `https://` or `file://` | the latest GitHub release |

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
them and nothing else, and takes no arguments. The journal's own files (`~/.jellymoon`) stay. It is a command and not
a flag of the script because the hook lives in the harness's settings, which only core reads and writes, and because
the command is already on the machine and needs no network. The hook goes first, as `jellymoon hook uninstall`
removes it ([delivery.md](delivery.md)).

## Cutting a release

`.github/workflows/release.yml` runs when a tag `v<version>` is pushed: it builds `jellymoon` and `jellymoon-app` for
both chips on a Mac runner, with no `target` folder restored from an earlier run, and runs `app/macos/bundle.sh` (the
bundle, its `Info.plist`, the ad-hoc signatures, the archive). It then writes `SHA256SUMS`, creates the release as a
draft with all three files attached, and only then publishes it. A tag with a dash in it (`v0.2.0-rc1`) is published
as a pre-release. Run by hand from the Actions tab, from a branch or from a tag, it builds the same archives as
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
  found Claude Code in one of the places `page.md` lists, read the day with one call and kept the page.

Not proved yet:

- **A Mac that has never seen the app.** That it opens with no Gatekeeper prompt there, only a person can see.
- **A person's own double click.** `open` takes the same road, and nobody has yet clicked.
- **The Intel archive on an Intel Mac.** It was built and published, and never run.
- **An older macOS.** `LSMinimumSystemVersion` is 11.0, a guess: the app ran on one, current, macOS only.
- **No icon.** The repo holds none, so the bundle has the system's plain one.
