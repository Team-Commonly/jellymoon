# Sharing the Go and Bun toolchains between worktrees

`scripts/tools.sh` installs the Go and Bun pinned in `.tool-versions`. By default they go into the
worktree's own `.tools/`, about 360 MB each time. With many worktrees on one machine, share one copy.

## Turn it on
Set an absolute path, once, in your shell profile or the launcher that starts your sessions:

    export JM_TOOLCHAIN_CACHE=/abs/path/to/jellymoon-toolchains

A worktree that already has its own `.tools/` must drop it first, or the old copies stay on disk:
`rm -rf .tools`. Then run `./init.sh` (or `make tools`) as usual. The first worktree downloads; the rest download
nothing. Each worktree keeps only `.tools/bin/` with three symlinks into the cache, so `du -sh .tools`
is a few kilobytes. Unset the variable and nothing changes: CI and fresh clones install under
`.tools/` exactly as before.

The cache holds `go-<ver>-<os>-<arch>/` and `bun-<ver>-<os>-<arch>/`. A version bump in
`.tool-versions` adds a new directory; old ones are safe to delete once no worktree links to them.

## Why it is safe to share
- **Checksummed.** Each archive is verified against the vendor's checksum (Go's `.sha256`, Bun's
  `SHASUMS256.txt`) before it is unpacked, then deleted. The verified checksum is kept in a
  `.sha256` file inside the version directory.
- **Atomic.** The installer unpacks into a temp directory inside the cache and renames it into
  place under a lock, so two worktrees installing at once never see a half-unpacked directory and
  the cache ends with one copy.
- **Refuses, never overwrites.** A cached directory is reused only if its `.sha256` exists and its
  `go version` / `bun --version` report the pinned version. Otherwise `tools.sh` stops with an error
  and leaves the directory alone, because another worktree may be running it. Delete it yourself
  if it is stale.
- **Read-only.** The unpacked tree is `chmod -R a-w`, so a unit cannot modify a shared toolchain.
  To remove a version: `chmod -R u+w <dir> && rm -rf <dir>`.
- **The clean shell is unchanged.** The pilot shell (`env -i`, an empty HOME, PATH of
  `$PWD/.tools/bin` plus system dirs) still finds `go` and `bun` only through `.tools/bin`. The
  variable is read by `tools.sh` alone and puts nothing new on PATH. Pass it inside the command
  (`JM_TOOLCHAIN_CACHE=... ./init.sh`) when you run under `env -i`.

- **Read-only caches work.** A version that is already in the cache is used without taking any lock
  and without write access, so a sandboxed reviewer can mount the cache read-only. Installing a missing
  version into a read-only cache fails at once with a message saying so.
- **Killed installers recover.** The lock directory records the installer's PID. A waiter that finds
  that process dead removes the lock and continues, and clears the half-written `.dl-*` and
  `.unpack-*` temp directories. If the holder is alive but stuck, a waiter gives up after 2 minutes
  and names the lock.
