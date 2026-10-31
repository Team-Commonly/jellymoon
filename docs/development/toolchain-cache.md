# Sharing the Go and Bun toolchains between worktrees

`scripts/tools.sh` installs the Go and Bun pinned in `.tool-versions`. By default they go into the
worktree's own `.tools/`, about 360 MB each time. With many worktrees on one machine, share one copy.

## Turn it on
Set an absolute path under your home directory, once, in your shell profile or the launcher that starts
your sessions:

    export JM_TOOLCHAIN_CACHE=<your home directory>/.cache/jellymoon-toolchains

Use your home directory because every directory above the cache must be yours or root's (see below), and
a home directory and the system directories above it normally are. A shared directory such as
`/tmp` is accepted only when the directory you make in it is yours, and it is often cleared on restart.

A worktree that already has its own `.tools/` must drop it first, or the old copies stay on disk:
`rm -rf .tools`. Then run `./init.sh` (or `make tools`) as usual. The first worktree downloads; the rest
download nothing. Each worktree keeps only `.tools/bin/` with three symlinks into the cache, so
`du -sh .tools` is a few kilobytes.

The cache holds `go-<ver>-<os>-<arch>/` and `bun-<ver>-<os>-<arch>/`. A version bump in
`.tool-versions` adds a new directory; old ones are safe to delete once no worktree links to them.

## With the variable unset
`tools.sh` installs into `.tools/` as it did before the cache existed, and still uses a `go` or `bun` on
PATH that already reports the pinned version. Three things differ: the downloaded archives are deleted
once verified and unpacked (they used to stay in `.tools/dl/`), each version directory gains a `.sha256`
stamp, and the version directories are made `0755` whatever the umask (they used to follow it). CI does
not run `tools.sh`; its jobs install Go and Bun with their own setup steps.

## What is checked
With the variable set, `tools.sh` does the following before anything in the cache runs or is installed.
Before these checks, the only change it makes on the cache path is to create missing directories.

- **The path as written.** It must be absolute and have no `.` or `..` component. Trailing slashes are
  dropped, and the path itself must not be a symlink; a directory above it may be. Missing directories
  are created one at a time with mode `0755`, whatever the umask.
- **The physical path, and only that.** The path is then resolved (`cd -P`, `pwd -P`), and the result
  is used for every check, every install and every link written into `.tools/bin`, so no link passes
  through a symlink.
- **Every directory above the root.** From `/` down, each must be owned by you or by root and writable by
  no one else. Otherwise its owner, or anyone who can write to it, could rename the root aside and put a
  tree of their own at the path every worktree's links follow. The one exception is a root-owned
  directory with the sticky bit, such as `/tmp`: others may write to it, since they can add entries there
  but cannot rename or remove one they do not own. It is accepted when the directory below it is yours.
- **The root and what a worktree links to.** The root, each version directory already in the cache,
  Go's `bin/`, and the `go`, `gofmt` and `bun` binaries must be owned by you, must not be symlinks, and
  must be neither group- nor other-writable. A cache owned by another user is refused, so one cache
  serves one user's worktrees.
- **ACLs, on every path above.** An ACL can let others write whatever the mode says. On macOS the entries
  are listed (`ls -le`; `ls -l` prints `@`, not `+`, when a path also has extended attributes), and any
  entry other than a deny entry is refused. Deny entries cannot grant anything, and a macOS home
  directory carries one. Elsewhere a `+` after the mode is the only sign, and it is refused.
- **What it cannot read.** A path that is missing, or that `ls` lists as nothing, is refused as such.

A failure stops `tools.sh` with an error that names the path.

## Why it is safe to share
- **Checksummed at install, not on use.** Each archive is verified against the vendor's checksum
  (Go's `.sha256`, Bun's `SHASUMS256.txt`, fetched over HTTPS beside it) before it is unpacked, then
  deleted. The `.sha256` file in the version directory records that verified install. On use it must
  exist and not be empty, but its value is never re-verified. A version directory also needs the checks
  above and its own `go version` / `bun --version` reporting the pinned version. Those two are read from
  the directory itself, so they catch a partial or stale copy, not a forged one.
- **Refuses, never overwrites.** Anything that fails these checks is left alone, because another
  worktree may be running it. Delete it yourself, or fix it: `chmod -R go-w <dir>` for a version
  directory (it also covers the files the check does not read), `chmod -N <path>` to remove a macOS ACL
  from one path.
- **Read-only against accidents.** The unpacked tree is `chmod -R a-w`, which stops an accidental
  write. It is not a lock: you, or anything running as you, can undo it. To remove a version:
  `chmod -R u+w <dir> && rm -rf <dir>`.
- **Atomic.** The installer unpacks into a temp directory inside the cache and renames it into
  place under a lock, so two worktrees installing at once never see a half-unpacked directory and
  the cache ends with one copy.
- **Read-only caches work.** A version that is already in the cache is used without taking any lock
  and without write access, so a sandboxed process running as you can be given the cache read-only.
  Installing a missing version into a read-only cache fails at once with a message saying so.
- **The clean shell is unchanged.** The pilot shell (`env -i`, an empty HOME, PATH of
  `$PWD/.tools/bin` plus system dirs) still finds `go` and `bun` only through `.tools/bin`. The
  variable is read by `tools.sh` alone and puts nothing new on PATH. Pass it inside the command
  (`JM_TOOLCHAIN_CACHE=... ./init.sh`) when you run under `env -i`.
- **Killed installers recover.** The lock directory records the installer's PID. A waiter that finds
  that process dead removes the lock and continues, and clears the half-written `.dl-*` and
  `.unpack-*` temp directories. If the holder is alive but stuck, a waiter gives up after 2 minutes
  and names the lock.

## What it does not stop
Anything running as you, or as root, can still replace a cached toolchain, and every worktree that links
to it runs the replacement on its next `make`, without `tools.sh` in between. So a cache must never be
writable by anything you would not let edit your worktrees. The checks run when `tools.sh` runs, not each
time `make` follows a link; between runs, the path stays safe only because no one else can write to
anything on it. Of the Go tree, the check reads the version directory, `bin/` and the two binaries, not
the rest (`go.env`, `pkg/tool/`, `src/`): those rely on the installer's `chmod -R a-w` and on a version
directory no one else can write to.
