#!/usr/bin/env bash
# The shared toolchain cache must refuse a root or entry that anyone but the current user could have
# written, before it runs anything in it (docs/development/toolchain-cache.md). Each case runs a copy of
# scripts/tools.sh in a throwaway repo against a fixture cache. The fixture go and bun print invented
# pinned versions and log every run; the only curl on PATH logs and fails, so nothing is downloaded.
# Run: scripts/check-tools-cache.sh (part of make preflight).
set -euo pipefail
cd "$(dirname "$0")/.."
tmp=$(mktemp -d "${TMPDIR:-/tmp}/check-tools-cache.XXXXXX")
trap 'rm -rf "$tmp"' EXIT

GO=1.99.0; BUN=9.9.9 # invented pins for the throwaway repo
os=$(uname -s | tr '[:upper:]' '[:lower:]')
case $(uname -m) in x86_64|amd64) arch=amd64 ;; arm64|aarch64) arch=arm64 ;; *) echo "unsupported arch"; exit 1 ;; esac
G=go-$GO-$os-$arch; B=bun-$BUN-$os-$arch # the entry names tools.sh looks for

mkdir -p "$tmp/repo/scripts" "$tmp/bin" "$tmp/c"
cp scripts/tools.sh "$tmp/repo/scripts/tools.sh"
printf 'go %s\nbun %s\n' "$GO" "$BUN" > "$tmp/repo/.tool-versions"
printf '#!/bin/sh\necho "ran curl $*" >> "$JM_FAKE_LOG"; exit 1\n' > "$tmp/bin/curl"
printf '#!/bin/sh\necho "ran $0 $*" >> "$JM_FAKE_LOG"\ncase "$1" in version) echo "go version go%s fake" ;; env) echo go%s ;; esac\n' "$GO" "$GO" > "$tmp/go"
printf '#!/bin/sh\necho "ran $0 $*" >> "$JM_FAKE_LOG"\necho %s\n' "$BUN" > "$tmp/bun"
chmod 755 "$tmp/bin/curl"

# A cache as the installer leaves it, but 0755 throughout: owner-writable is allowed, and cleanup needs no chmod.
make_cache() { # root
  mkdir -p "$1/$G/bin" "$1/$B"
  cp "$tmp/go" "$1/$G/bin/go"; cp "$tmp/go" "$1/$G/bin/gofmt"; cp "$tmp/bun" "$1/$B/bun"
  echo fixture > "$1/$G/.sha256"; echo fixture > "$1/$B/.sha256"
  chmod 755 "$1" "$1/$G" "$1/$G/bin" "$1/$G/bin/go" "$1/$G/bin/gofmt" "$1/$B" "$1/$B/bun"
}

fails=0; count=0
# run <name> <root> <want> [<forbid>]. want is "used"; "download" (the root passed and the install reached
# curl); or the refusal tools.sh must print. A refused case must run no curl and no fixture binary under
# <forbid> (default: the whole root). umask 002 throughout: a root tools.sh creates must still be 0755.
run() {
  local name=$1 root=$2 want=$3 forbid=${4:-$2/} out rc=0 bad="" log=$tmp/log
  : > "$log"; count=$((count+1))
  out=$(cd "$tmp/repo" && umask 002 && env -i PATH="$tmp/bin:/usr/bin:/bin:/usr/sbin:/sbin" JM_FAKE_LOG="$log" \
    JM_TOOLCHAIN_CACHE="$root" scripts/tools.sh 2>&1) || rc=$?
  case $want in
    used)
      [ "$rc" = 0 ] && grep -qF "ran $root/$G/bin/go version" "$log" && grep -qF "ran $root/$B/bun --version" "$log" \
        && ! grep -q '^ran curl' "$log" && [ "$(readlink "$tmp/repo/.tools/bin/go")" = "$root/$G/bin/go" ] \
        && [ "${out##*$'\n'}" = "tools: go$GO bun $BUN" ] || bad="the cache was not used" ;;
    download)
      [ "$rc" != 0 ] && grep -q '^ran curl' "$log" && ! grep -qv '^ran curl' "$log" \
        && [ "$(ls -ld "$root" | cut -c1-10)" = drwxr-xr-x ] || bad="the root was not created 0755 and accepted" ;;
    *)
      [ "$rc" != 0 ] && case $out in *"tools: refusing $want"*) true ;; *) false ;; esac \
        && ! grep -qF -e "ran $forbid" -e "ran curl" "$log" || bad="not refused before anything in it ran" ;;
  esac
  if [ -z "$bad" ]; then echo "ok   $name"; else
    fails=$((fails+1)); echo "FAIL $name: $bad"; echo "$out" | sed 's/^/       /'; sed 's/^/       log: /' "$log"
  fi
}

w='it is group- or other-writable'
c=$tmp/c
make_cache "$c/ok";        run 'a correctly owned 0755 cache is used' "$c/ok" used
make_cache "$c/g-root";    chmod g+w "$c/g-root";           run 'group-writable root' "$c/g-root" "$c/g-root: $w"
make_cache "$c/o-root";    chmod o+w "$c/o-root";           run 'other-writable root' "$c/o-root" "$c/o-root: $w"
make_cache "$c/o-entry";   chmod o+w "$c/o-entry/$G";       run 'other-writable go entry' "$c/o-entry" "$c/o-entry/$G: $w"
make_cache "$c/o-bin";     chmod o+w "$c/o-bin/$G/bin";     run 'other-writable bin/' "$c/o-bin" "$c/o-bin/$G/bin: $w"
make_cache "$c/g-go";      chmod g+w "$c/g-go/$G/bin/go";   run 'group-writable go binary' "$c/g-go" "$c/g-go/$G/bin/go: $w"
make_cache "$c/o-gofmt";   chmod o+w "$c/o-gofmt/$G/bin/gofmt"; run 'other-writable gofmt' "$c/o-gofmt" "$c/o-gofmt/$G/bin/gofmt: $w"
make_cache "$c/g-bunentry"; chmod g+w "$c/g-bunentry/$B";   run 'group-writable bun entry' "$c/g-bunentry" "$c/g-bunentry/$B: $w" "$c/g-bunentry/$B/"
make_cache "$c/o-bun";     chmod o+w "$c/o-bun/$B/bun";     run 'other-writable bun binary' "$c/o-bun" "$c/o-bun/$B/bun: $w" "$c/o-bun/$B/"
make_cache "$c/link";      mv "$c/link/$G/bin/go" "$c/go-elsewhere"; ln -s "$c/go-elsewhere" "$c/link/$G/bin/go"
run 'a symlinked go binary, though its target is yours' "$c/link" "$c/link/$G/bin/go: it is a symlink"
# A directory another user owns cannot be made as one user, but / is owned by root wherever this runs.
if [ "$(id -u)" = 0 ]; then echo "skip a root owned by another user: running as root, who owns /"
else run 'a root owned by another user (/, owned by root)' / "/: it is not owned by you"; fi
run 'a missing root is created 0755 and accepted' "$c/fresh/cache" download

echo "tools-cache: $count cases, $fails failed"
[ "$fails" = 0 ]
