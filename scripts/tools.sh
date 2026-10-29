#!/usr/bin/env bash
# Installs the pinned Go and Bun from .tool-versions. Never installs anything outside the repo
# or the cache below. A matching go/bun already on PATH is used as is (unset-cache mode only).
#
#   JM_TOOLCHAIN_CACHE unset  -> everything under .tools/ (what CI and a fresh clone get)
#   JM_TOOLCHAIN_CACHE=/abs   -> versions live in $JM_TOOLCHAIN_CACHE/{go,bun}-<ver>-<os>-<arch>/,
#                                shared by every worktree; .tools/ keeps only bin/ symlinks.
#                                See docs/development/toolchain-cache.md.
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$PWD/.tools/bin:$PATH"
want() { awk -v t="$1" '$1==t{print $2}' .tool-versions; }
GO=$(want go); BUN=$(want bun)
os=$(uname -s | tr '[:upper:]' '[:lower:]'); arch=$(uname -m)
case "$arch" in x86_64|amd64) goarch=amd64; bunarch=x64 ;; arm64|aarch64) goarch=arm64; bunarch=aarch64 ;; *) echo "unsupported arch $arch"; exit 1 ;; esac # goarch also names the cache dirs
mkdir -p .tools/bin
sha() { if command -v sha256sum >/dev/null; then sha256sum "$1"; else shasum -a 256 "$1"; fi | cut -d' ' -f1; }
die() { echo "tools: $*" >&2; exit 1; }

CACHE=${JM_TOOLCHAIN_CACHE:-}
if [ -n "$CACHE" ]; then
  case "$CACHE" in /*) ;; *) die "JM_TOOLCHAIN_CACHE must be an absolute path (got $CACHE)" ;; esac
  mkdir -p "$CACHE"
  ROOT=$CACHE
  godir="$CACHE/go-$GO-$os-$goarch"; bundir="$CACHE/bun-$BUN-$os-$goarch"
else
  ROOT=.tools
  godir=".tools/go-$GO"; bundir=".tools/bun-$os-$bunarch"
fi

go_ok() { [ "$("$1/bin/go" version 2>/dev/null | cut -d' ' -f3)" = "go$GO" ]; }
bun_ok() { [ "$("$1/bun" --version 2>/dev/null)" = "$BUN" ]; }

# A cache entry is trusted only with a stamp and the pinned version. Anything else is refused,
# never re-downloaded over, because another worktree may be running it.
check_cached() { # dir checker
  [ -s "$1/.sha256" ] || die "$1 has no .sha256 stamp; remove it deliberately if it is stale"
  "$2" "$1" || die "$1 does not report the pinned version; remove it deliberately if it is stale"
}

LOCK=""; WORK=""; UNPACK=""
cleanup() { # runs on EXIT, INT and TERM; a SIGKILL skips it, which acquire_lock recovers from
  for d in "$WORK" "$UNPACK"; do [ -z "$d" ] || { chmod -R u+w "$d" 2>/dev/null; rm -rf "$d"; }; done
  [ -z "$LOCK" ] || rm -rf "$LOCK"
  WORK=""; UNPACK=""; LOCK=""
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

# The lock dir holds the installer's PID. A waiter that finds that PID dead removes the lock and
# takes over; the timeout (2 min, an install takes about 7 s) is only a backstop.
acquire_lock() { # lockdir
  local n=0 pid
  until mkdir "$1" 2>/dev/null; do
    pid=$(cat "$1/pid" 2>/dev/null || true)
    if [ -n "$pid" ] && ! kill -0 "$pid" 2>/dev/null; then rm -rf "$1"; continue; fi
    n=$((n+1)); [ "$n" -le 120 ] || die "waited 2 min for $1 (holder pid ${pid:-unknown}); remove it if no installer is running"
    sleep 1
  done
  LOCK=$1; echo $$ > "$1/pid"
}

# install_into <dir> <fetch-fn> <checker-fn>: download+verify+unpack into a sibling temp dir,
# stamp, then rename into place under a lock, so a concurrent installer never sees a half-unpacked
# dir. A cache version dir only ever appears by rename, so an existing one is used with no lock
# and no write access to the cache.
install_into() {
  local dir=$1 fetch=$2 checker=$3 name; name=$(basename "$1")
  if [ -n "$CACHE" ] && [ -e "$dir" ]; then check_cached "$dir" "$checker"; return; fi
  [ -w "$ROOT" ] || die "$name is not in $ROOT and $ROOT is not writable; ask whoever owns the cache to install it"
  acquire_lock "$ROOT/.lock-$name"
  # Leftovers of a killed installer. Names are per tool so the other tool's live install is safe.
  for d in "$ROOT"/.dl-"$name".* "$ROOT"/.unpack-"$name".*; do [ -e "$d" ] && { chmod -R u+w "$d"; rm -rf "$d"; }; done
  if [ -n "$CACHE" ] && [ -e "$dir" ]; then check_cached "$dir" "$checker"; else
    WORK=$(mktemp -d "$ROOT/.dl-$name.XXXXXX"); UNPACK=$(mktemp -d "$ROOT/.unpack-$name.XXXXXX")
    "$fetch" "$WORK" "$UNPACK"
    rm -rf "$WORK"; WORK=""                           # the archive is dead weight once verified
    "$checker" "$UNPACK" || die "unpacked toolchain does not report the pinned version"
    [ -z "$CACHE" ] || chmod -R a+rX,a-w "$UNPACK"    # a unit must not modify a shared toolchain
    [ -n "$CACHE" ] || rm -rf "$dir"
    mv "$UNPACK" "$dir"; UNPACK=""
  fi
  cleanup
}

fetch_go() { # workdir unpackdir
  local f=go$GO.$os-$goarch.tar.gz
  curl -fsSL "https://dl.google.com/go/$f" -o "$1/$f"
  local got; got=$(sha "$1/$f")
  [ "$got" = "$(curl -fsSL "https://dl.google.com/go/$f.sha256")" ] || die "go checksum mismatch"
  tar -xzf "$1/$f" -C "$2" --strip-components 1
  echo "$got" > "$2/.sha256"
}

fetch_bun() {
  local d=bun-$os-$bunarch base=https://github.com/oven-sh/bun/releases/download/bun-v$BUN
  curl -fsSL "$base/$d.zip" -o "$1/$d.zip"
  curl -fsSL "$base/SHASUMS256.txt" -o "$1/SHASUMS256.txt"
  local got; got=$(sha "$1/$d.zip")
  grep -q "$got  $d.zip" "$1/SHASUMS256.txt" || die "bun checksum mismatch"
  unzip -q -j -o "$1/$d.zip" -d "$2"
  echo "$got" > "$2/.sha256"
}

# Cache mode links to the absolute cache path; unset mode keeps today's relative links.
put_link() { if [ -n "$CACHE" ]; then ln -sfn "$1" ".tools/bin/$2"; else ln -sf "$3" ".tools/bin/$2"; fi; }

if [ -n "$CACHE" ] || [ "$(go env GOVERSION 2>/dev/null)" != "go$GO" ]; then
  install_into "$godir" fetch_go go_ok
  for b in go gofmt; do put_link "$godir/bin/$b" "$b" "../go-$GO/bin/$b"; done
fi

if [ -n "$CACHE" ] || [ "$(bun --version 2>/dev/null)" != "$BUN" ]; then
  install_into "$bundir" fetch_bun bun_ok
  put_link "$bundir/bun" bun "../bun-$os-$bunarch/bun"
fi
echo "tools: $(go env GOVERSION) bun $(bun --version)"
