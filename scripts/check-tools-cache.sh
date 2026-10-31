#!/usr/bin/env bash
# The shared toolchain cache must refuse a root, an entry, or a directory above the root that anyone but the current
# user (or root, above the root) could have written, before it runs anything in it (docs/development/toolchain-cache.md).
# Each case runs a copy of scripts/tools.sh in a throwaway repo against a fixture cache. The fixture go and bun print
# invented pinned versions and log every run; the only curl on PATH logs and fails, so nothing is downloaded. The ls on
# PATH lists a path named in the case's fake-ls file as the line given there, for shapes one user cannot make (a
# directory another user owns); every other path is listed by the real ls.
# Run: scripts/check-tools-cache.sh (part of make preflight).
set -euo pipefail
cd "$(dirname "$0")/.."
umask 022 # the fixtures' own directories; run() switches to 002 for tools.sh
tmp=$(mktemp -d "${TMPDIR:-/tmp}/check-tools-cache.XXXXXX"); st=""
trap '[ ! -e "$tmp/c/m5" ] || chmod -N "$tmp/c/m5"; rm -rf "$tmp" ${st:+"$st"}' EXIT # m5's deny delete entry would stop rm
tmp=$(cd -P "$tmp" && pwd -P) # tools.sh links to the physical path, so the fixtures name that one

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
cat > "$tmp/bin/ls" <<'EOF'
#!/bin/sh
for a; do p=$a; done
l=$(awk -F'|' -v p="$p" '$1 == p { print substr($0, length(p) + 2); f = 1 } END { exit !f }' "$JM_FAKE_LS") && { [ -z "$l" ] || echo "$l"; exit 0; }
exec /bin/ls "$@"
EOF
chmod 755 "$tmp/bin/curl" "$tmp/bin/ls"; : > "$tmp/fake-ls"

# A cache as the installer leaves it, but 0755 throughout: owner-writable is allowed, and cleanup needs no chmod.
make_cache() { # root
  mkdir -p "$1/$G/bin" "$1/$B"
  cp "$tmp/go" "$1/$G/bin/go"; cp "$tmp/go" "$1/$G/bin/gofmt"; cp "$tmp/bun" "$1/$B/bun"
  echo fixture > "$1/$G/.sha256"; echo fixture > "$1/$B/.sha256"
  chmod 755 "$1" "$1/$G" "$1/$G/bin" "$1/$G/bin/go" "$1/$G/bin/gofmt" "$1/$B" "$1/$B/bun"
}
fake() { printf '%s|%s\n' "$1" "$2" >> "$tmp/fake-ls"; } # path line: how the next case's ls lists path

fails=0; count=0
# run <name> <root> <want> [<path>]. want is "used" (<path>: the physical root every run and link must use, default
# <root>); "download" (the root and the two directories above it, all made by tools.sh, came out 0755 under umask 002
# and the install reached curl); or the refusal tools.sh must print, and then no curl and no fixture binary under <path>
# (default: the root) may have run.
run() {
  local name=$1 root=$2 want=$3 at=${4:-$2} out rc=0 bad="" log=$tmp/log
  : > "$log"; count=$((count+1))
  out=$(cd "$tmp/repo" && umask 002 && env -i PATH="$tmp/bin:/usr/bin:/bin:/usr/sbin:/sbin" JM_FAKE_LOG="$log" \
    JM_FAKE_LS="$tmp/fake-ls" JM_TOOLCHAIN_CACHE="$root" scripts/tools.sh 2>&1) || rc=$?
  : > "$tmp/fake-ls"
  case $want in
    used)
      [ "$rc" = 0 ] && grep -qF "ran $at/$G/bin/go version" "$log" && grep -qF "ran $at/$B/bun --version" "$log" \
        && ! grep -q '^ran curl' "$log" && [ "$(readlink "$tmp/repo/.tools/bin/go")" = "$at/$G/bin/go" ] \
        && [ "${out##*$'\n'}" = "tools: go$GO bun $BUN" ] || bad="the cache was not used" ;;
    download)
      [ "$rc" != 0 ] && grep -q '^ran curl' "$log" && ! grep -qv '^ran curl' "$log" \
        && [ "$(ls -ld "$root" "${root%/*}" "${root%/*/*}" | cut -c1-10 | sort -u)" = drwxr-xr-x ] \
        || bad="the root and its parents were not created 0755 and accepted" ;;
    *)
      [ "$rc" != 0 ] && case $out in *"tools: refusing $want"*) true ;; *) false ;; esac \
        && ! grep -qF -e "ran ${4:-$2/}" -e "ran curl" "$log" || bad="not refused before anything in it ran" ;;
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
run 'a missing root and its missing parents are created 0755 and accepted' "$c/fresh/p1/cache" download

# Above the root, and the forms of the root's path.
mkdir "$c/g-par"; chmod 775 "$c/g-par"; run 'group-writable parent, before the root exists' "$c/g-par/cache" "$c/g-par: $w"
make_cache "$c/o-par/cache"; chmod o+w "$c/o-par"; run 'other-writable parent of a cache' "$c/o-par/cache" "$c/o-par: $w"
make_cache "$c/sl/target"; ln -s "$c/sl/target" "$c/sl/link"
run 'a symlinked root written with a trailing /' "$c/sl/link/" "$c/sl/link: it is a symlink" "$c/sl/"
run 'a root written with /.' "$c/sl/link/." "$c/sl/link/.: it has a . or .. component" "$c/sl/"
make_cache "$c/vlink";     mv "$c/vlink/$G" "$c/vlink-go"; ln -s "$c/vlink-go" "$c/vlink/$G"
run 'a symlinked version dir' "$c/vlink" "$c/vlink/$G: it is a symlink" "$c/vlink"
make_cache "$c/real/cache"; ln -s "$c/real" "$c/via"
run 'a root reached through a symlinked parent is used by its physical path' "$c/via/cache" used "$c/real/cache"

# Shapes one user cannot make, listed by the fake ls.
make_cache "$c/f1/p/cache"; fake "$c/f1/p" "drwxr-xr-x 2 $(( $(id -u) + 1 )) 0 64 Jan 1 00:00 $c/f1/p"
run 'a parent owned by another user' "$c/f1/p/cache" "$c/f1/p: it is not owned by you or root"
make_cache "$c/f2/p/cache"; fake "$c/f2/p" ''
run 'a parent ls prints nothing for' "$c/f2/p/cache" "$c/f2/p: it could not be listed (ls printed nothing)"
make_cache "$c/f3/p/cache"; fake "$c/f3/p" "drwxrwxrwt 2 0 0 64 Jan 1 00:00 $c/f3/p"
run 'a root-owned sticky parent above a directory of yours is accepted' "$c/f3/p/cache" used
make_cache "$c/f4/p/q/cache"; fake "$c/f4/p" "drwxrwxrwt 2 0 0 64 Jan 1 00:00 $c/f4/p"; fake "$c/f4/p/q" "drwxr-xr-x 2 0 0 64 Jan 1 00:00 $c/f4/p/q"
run 'a root-owned sticky parent above a directory that is not yours' "$c/f4/p/q/cache" "$c/f4/p: it is writable by others, and $c/f4/p/q below it is not yours"
a='it has an ACL that may let others write'
make_cache "$c/f5/p/cache"; fake "$c/f5/p" "drwxr-xr-x+ 2 $(id -u) 0 64 Jan 1 00:00 $c/f5/p"
run 'a parent with a "+" after its mode, the only sign of an ACL on Linux' "$c/f5/p/cache" "$c/f5/p: $a"

# macOS ACLs, set for real. ls shows "@", not "+", when the path also has an extended attribute.
if [ "$os" = darwin ]; then
  make_cache "$c/m1"; chmod +a 'everyone allow write' "$c/m1"; run 'an ACL on the root' "$c/m1" "$c/m1: $a"
  make_cache "$c/m2"; xattr -w com.example.fixture 1 "$c/m2/$G/bin/go"; chmod +a 'everyone allow write' "$c/m2/$G/bin/go"
  run 'an ACL on the go binary, shown as "@" (it also has an extended attribute)' "$c/m2" "$c/m2/$G/bin/go: $a"
  mkdir "$c/m3"; chmod +a 'everyone allow write,file_inherit,directory_inherit' "$c/m3"
  run 'an inheritable ACL on a parent, before tools.sh creates the root' "$c/m3/cache" "$c/m3: $a"
  mkdir "$c/m4"; chmod +a 'everyone allow write,file_inherit,directory_inherit' "$c/m4"; make_cache "$c/m4/cache"; chmod -N "$c/m4"
  run 'an ACL the cache inherited from a parent that no longer has it' "$c/m4/cache" "$c/m4/cache: $a"
  make_cache "$c/m5"; chmod +a 'everyone deny delete' "$c/m5"
  run 'a deny-only ACL, as a home directory carries, is accepted' "$c/m5" used
else echo "skip the macOS ACL cases: not macOS"; fi

# The real thing: /tmp is a root-owned sticky directory on macOS and Linux, and a directory mktemp makes in it is yours.
tp=$(cd -P /tmp && pwd -P)
if ls -ldn "$tp" | awk '{ exit !($3 == 0 && $1 ~ /^d........[tT]/) }'; then
  st=$(mktemp -d /tmp/check-tools-cache.XXXXXX); make_cache "$st/cache"
  run 'a root in a directory of yours under /tmp (root-owned, sticky) is used' "$st/cache" used "$tp/${st##*/}/cache"
else echo "skip a root under /tmp: $tp is not a root-owned sticky directory here"; fi

echo "tools-cache: $count cases, $fails failed"
[ "$fails" = 0 ]
