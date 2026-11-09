#!/usr/bin/env bash
# Downloads the Go pinned in .tool-versions from dl.google.com into <dest>/go, after checking its sha256
# against go.dev's release index (a different host from the archive). Installs nothing outside <dest>.
# Prints the bin directory on the last line. Any failure exits non-zero.
#
#   fallback.sh <dest-dir> [tool-versions-file]
set -euo pipefail
dest=${1:?usage: fallback.sh <dest-dir> [tool-versions-file]}
pins=${2:-.tool-versions}
die() { echo "go fallback: $*" >&2; exit 1; }

ver=$(awk '$1=="go"{print $2; exit}' "$pins")
[[ "$ver" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "$pins has no exact go version (got '$ver')"
os=$(uname -s | tr '[:upper:]' '[:lower:]')
case "$(uname -m)" in x86_64|amd64) arch=amd64 ;; arm64|aarch64) arch=arm64 ;; *) die "unsupported arch $(uname -m)" ;; esac
file=go$ver.$os-$arch.tar.gz

sha() { if command -v sha256sum >/dev/null; then sha256sum "$1"; else shasum -a 256 "$1"; fi | cut -d' ' -f1; }

mkdir -p "$dest"
work=$(mktemp -d "$dest/.dl.XXXXXX")
trap 'rm -rf "$work"' EXIT

want=$(curl -fsSL --retry 3 'https://go.dev/dl/?mode=json&include=all' \
  | jq -r --arg f "$file" '[.[].files[] | select(.filename == $f) | .sha256][0] // empty') || die "cannot read go.dev release index"
[[ "$want" =~ ^[0-9a-f]{64}$ ]] || die "go.dev lists no sha256 for $file"

curl -fsSL --retry 3 "https://dl.google.com/go/$file" -o "$work/$file" || die "download of $file failed"
got=$(sha "$work/$file")
[ "$got" = "$want" ] || die "checksum mismatch for $file: got $got, go.dev says $want"

rm -rf "$dest/go"
mkdir -p "$dest/go"
tar -xzf "$work/$file" -C "$dest/go" --strip-components 1
reported=$("$dest/go/bin/go" env GOVERSION)
[ "$reported" = "go$ver" ] || die "unpacked toolchain reports '$reported', wanted go$ver"
echo "go fallback: $reported (sha256 $got)" >&2
echo "$dest/go/bin"
