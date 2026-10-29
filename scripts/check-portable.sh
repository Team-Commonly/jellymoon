#!/usr/bin/env bash
# The repo must work from a fresh clone on any machine: no tracked symlink may leave the
# repo, and no tracked file may name a home directory or a machine-local path.
set -euo pipefail
cd "$(dirname "$0")/.."
root=$(pwd -P); bad=0
while IFS= read -r f; do
  target=$(cd "$(dirname "$f")" && cd "$(dirname "$(readlink "$f")")" 2>/dev/null && pwd -P || echo /missing)
  case "$target/" in "$root"/*) ;; *) echo "symlink leaves the repo: $f -> $(readlink "$f")"; bad=1 ;; esac
done < <(git ls-files -s | awk '$1=="120000"{print $4}')
pattern='(~/[.A-Za-z]|/Users/[a-z]|/home/[a-z]|\$HOME/|\$\(HOME\))'
if git grep -nIE "$pattern" -- . ':!scripts/check-portable.sh'; then
  echo "machine-local path in a tracked file (above); use a repo-relative path"; bad=1
fi
exit $bad
