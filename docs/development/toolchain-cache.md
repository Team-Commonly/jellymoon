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
- **Yours alone, or refused.** Before it uses or installs anything, `tools.sh` checks the cache
  root (after `mkdir -p`, so a directory someone else created there first is refused), each version
  directory it uses, Go's `bin/`, and the `go`, `gofmt` and `bun` binaries a worktree links to. Each
  must be owned by you, must not be a symlink, and must be neither group- nor other-writable.
  Otherwise `tools.sh` stops and names the path. A root it creates is `0755`. A cache owned by
  another user is refused, so one cache serves one user's worktrees. The check reads mode bits, not
  ACLs, and does not look above the root: keep the root where no one else can rename it.
- **Checksummed at install, not on use.** Each archive is verified against the vendor's checksum
  (Go's `.sha256`, Bun's `SHASUMS256.txt`, fetched over HTTPS beside it) before it is unpacked, then
  deleted. The `.sha256` file in the version directory records that verified install and is never
  checked again. On use, a version directory needs the ownership check above, the stamp, and its
  own `go version` / `bun --version` reporting the pinned version. The last two are read from the
  directory itself, so they catch a partial or stale copy, not a forged one.
- **Refuses, never overwrites.** Anything that fails these checks stops `tools.sh` with an error and
  is left alone, because another worktree may be running it. Fix it (`chmod go-w <path>` for a mode)
  or delete it yourself.
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

**What it does not stop.** Anything running as you can still replace a cached toolchain, and every
worktree that links to it runs the replacement on its next `make`. So a cache must never be writable
by anything you would not let edit your worktrees.
