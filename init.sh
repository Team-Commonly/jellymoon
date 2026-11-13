#!/bin/sh
# One command: the pinned tools, the baseline, the feature scoreboard. Every session runs it first (AGENTS.md).
# Everything lands in .tools/ inside the repo. Nothing is installed globally and no shell profile is edited.
#
#   JM_RUST_HOME unset  -> rustup and cargo live in .tools/rustup and .tools/cargo
#   JM_RUST_HOME=/abs   -> a folder you own that already holds rustup/ and cargo/, shared by several worktrees
set -eu
cd "$(dirname "$0")"
die() { echo "init: $*" >&2; exit 1; }
sha() { if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1"; else shasum -a 256 "$1"; fi | cut -d' ' -f1; }
fetch() { curl --proto '=https' --tlsv1.2 -fsSL -o "$2" "$1" || die "download failed: $1"; }
verify() { [ "$(sha "$1")" = "$2" ] || die "sha256 mismatch for $1; nothing was installed from it"; }

os=$(uname -s | tr '[:upper:]' '[:lower:]')
arch=$(uname -m)
case "$arch" in x86_64|amd64) arch=x64 ;; arm64|aarch64) arch=arm64 ;; *) die "unsupported architecture $arch" ;; esac
mkdir -p .tools/bin .tools/dl

echo "== rust (rust-toolchain.toml)"
[ -f rust-toolchain.toml ] || die "rust-toolchain.toml is missing: it names the toolchain to install"
if [ -n "${JM_RUST_HOME:-}" ]; then
  [ -d "$JM_RUST_HOME/rustup" ] && [ -x "$JM_RUST_HOME/cargo/bin/rustup" ] ||
    die "JM_RUST_HOME=$JM_RUST_HOME does not hold rustup/ and cargo/; unset it to install into .tools/"
  rust_home=$JM_RUST_HOME
else
  rust_home=$PWD/.tools
fi
export RUSTUP_HOME="$rust_home/rustup" CARGO_HOME="$rust_home/cargo"
export PATH="$CARGO_HOME/bin:$PATH"
if [ ! -x "$CARGO_HOME/bin/rustup" ]; then
  # rustup-init 1.29.0; sha256 from the .sha256 file published beside each binary.
  case "$os-$arch" in
    darwin-arm64) triple=aarch64-apple-darwin; sum=aeb4105778ca1bd3c6b0e75768f581c656633cd51368fa61289b6a71696ac7e1 ;;
    darwin-x64) triple=x86_64-apple-darwin; sum=33cf85df9142bc6d29cbc62fa5ca1d4c29622cddb55213a4c1a43c457fb9b2d7 ;;
    linux-x64) triple=x86_64-unknown-linux-gnu; sum=4acc9acc76d5079515b46346a485974457b5a79893cfb01112423c89aeb5aa10 ;;
    linux-arm64) triple=aarch64-unknown-linux-gnu; sum=9732d6c5e2a098d3521fca8145d826ae0aaa067ef2385ead08e6feac88fa5792 ;;
    *) die "no pinned rustup-init for $os-$arch; on Windows install rustup yourself and run the cargo commands in the Makefile" ;;
  esac
  fetch "https://static.rust-lang.org/rustup/archive/1.29.0/$triple/rustup-init" .tools/dl/rustup-init
  verify .tools/dl/rustup-init "$sum"
  chmod +x .tools/dl/rustup-init
  .tools/dl/rustup-init -y --no-modify-path --default-toolchain none --profile minimal
fi
# With no toolchain named, rustup installs the one rust-toolchain.toml pins, with its components.
rustup toolchain install
cargo --version

echo "== gitleaks"
GITLEAKS=8.30.1 # sha256 from the release's gitleaks_8.30.1_checksums.txt
if [ ! -x .tools/bin/gitleaks ] || [ ! -f ".tools/bin/.gitleaks-$GITLEAKS" ]; then
  case "$os-$arch" in
    darwin-arm64) sum=b40ab0ae55c505963e365f271a8d3846efbc170aa17f2607f13df610a9aeb6a5 ;;
    darwin-x64) sum=dfe101a4db2255fc85120ac7f3d25e4342c3c20cf749f2c20a18081af1952709 ;;
    linux-x64) sum=551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb ;;
    linux-arm64) sum=e4a487ee7ccd7d3a7f7ec08657610aa3606637dab924210b3aee62570fb4b080 ;;
    *) die "no pinned gitleaks for $os-$arch" ;;
  esac
  tarball=gitleaks_${GITLEAKS}_${os}_${arch}.tar.gz
  fetch "https://github.com/gitleaks/gitleaks/releases/download/v$GITLEAKS/$tarball" ".tools/dl/$tarball"
  verify ".tools/dl/$tarball" "$sum"
  tar -xzf ".tools/dl/$tarball" -C .tools/bin gitleaks
  : > ".tools/bin/.gitleaks-$GITLEAKS"
fi
.tools/bin/gitleaks version

echo "== baseline: make preflight"
make preflight

echo "== features"
awk -F'"' '
  # A feature is blocked only while something its blockedBy names does not pass: a feature that is still
  # false, or words that name no feature at all.
  /^ *"id":/ { n++; id[n] = $4; inby = 0 }
  /^ *"description":/ { desc[n] = $4 }
  /^ *"passes": *true/ { pass[id[n]] = 1; passing++ }
  /^ *"blockedBy":/ { inby = 1; for (i = 4; i <= NF; i += 2) by[n] = by[n] "\"" $i; if ($0 ~ /\]/) inby = 0; next }
  inby { for (i = 2; i <= NF; i += 2) by[n] = by[n] "\"" $i }
  inby && /\]/ { inby = 0 }
  END {
    for (f = 1; f <= n; f++) {
      blocked = 0
      k = split(by[f], names, "\"")
      for (i = 2; i <= k; i++) if (!pass[names[i]]) blocked = 1
      printf "%s  %-7s  %s\n", id[f], (pass[id[f]] ? "pass" : blocked ? "blocked" : "todo"), desc[f]
    }
    printf "\n%d/%d passing\n", passing, n
  }
' feature_list.json
