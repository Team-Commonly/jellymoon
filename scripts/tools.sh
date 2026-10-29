#!/usr/bin/env bash
# Installs the pinned Go and Bun from .tool-versions into .tools/ (repo-local, gitignored).
# Never installs anything outside the repo. A matching go/bun already on PATH is used as is.
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$PWD/.tools/bin:$PATH"
want() { awk -v t="$1" '$1==t{print $2}' .tool-versions; }
GO=$(want go); BUN=$(want bun)
os=$(uname -s | tr '[:upper:]' '[:lower:]'); arch=$(uname -m)
case "$arch" in x86_64|amd64) goarch=amd64; bunarch=x64 ;; arm64|aarch64) goarch=arm64; bunarch=aarch64 ;; *) echo "unsupported arch $arch"; exit 1 ;; esac
mkdir -p .tools/bin .tools/dl
sha() { if command -v sha256sum >/dev/null; then sha256sum "$1"; else shasum -a 256 "$1"; fi | cut -d' ' -f1; }

if [ "$(go env GOVERSION 2>/dev/null)" != "go$GO" ]; then
  f=go$GO.$os-$goarch.tar.gz
  curl -fsSL "https://dl.google.com/go/$f" -o ".tools/dl/$f"
  [ "$(sha ".tools/dl/$f")" = "$(curl -fsSL "https://dl.google.com/go/$f.sha256")" ] || { echo "go checksum mismatch"; exit 1; }
  rm -rf ".tools/go-$GO" && mkdir -p ".tools/go-$GO" && tar -xzf ".tools/dl/$f" -C ".tools/go-$GO" --strip-components 1
  ln -sf "../go-$GO/bin/go" .tools/bin/go && ln -sf "../go-$GO/bin/gofmt" .tools/bin/gofmt
fi

if [ "$(bun --version 2>/dev/null)" != "$BUN" ]; then
  d=bun-$os-$bunarch; base=https://github.com/oven-sh/bun/releases/download/bun-v$BUN
  curl -fsSL "$base/$d.zip" -o ".tools/dl/$d.zip"
  curl -fsSL "$base/SHASUMS256.txt" -o .tools/dl/bun-SHASUMS256.txt
  grep -q "$(sha ".tools/dl/$d.zip")  $d.zip" .tools/dl/bun-SHASUMS256.txt || { echo "bun checksum mismatch"; exit 1; }
  rm -rf ".tools/$d" && unzip -q -o ".tools/dl/$d.zip" -d .tools
  ln -sf "../$d/bun" .tools/bin/bun
fi
echo "tools: $(go env GOVERSION) bun $(bun --version)"
