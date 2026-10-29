#!/usr/bin/env bash
# The repo must work from a fresh clone on any machine: no tracked symlink may leave the
# repo, no tracked file may name a home directory or a machine-local path, and no installed
# tool or dependency (.tools/, node_modules/) or file over 1 MB may be committed.
set -euo pipefail
cd "$(dirname "$0")/.."
root=$(pwd -P); bad=0
while IFS= read -r f; do
  target=$(cd "$(dirname "$f")" && cd "$(dirname "$(readlink "$f")")" 2>/dev/null && pwd -P || echo /missing)
  case "$target/" in "$root"/*) ;; *) echo "symlink leaves the repo: $f -> $(readlink "$f")"; bad=1 ;; esac
done < <(git ls-files -s | awk '$1=="120000"{print $4}')
pattern='(~/[.A-Za-z]|/home/[a-z]|/opt/homebrew|/private/t[m]p|\$HOME/|\$\(HOME\)|(^|[^A-Za-z0-9_./-])/Users/[A-Za-z])'
if git grep -nIiE "$pattern" -- . ':!scripts/check-portable.sh'; then
  echo "machine-local path in a tracked file (above); use a repo-relative path"; bad=1
fi
if git ls-files | grep -E '(^|/)(\.tools|node_modules)/' | head -3 | grep .; then
  echo "installed tools or dependencies are tracked (above); they belong in .gitignore"; bad=1
fi
big=$(git ls-files -s | awk '$1!="120000"{print $2" "$4}' | git cat-file --batch-check='%(objectsize) %(rest)' | awk '$1>1048576{print $2}')
if [ -n "$big" ]; then echo "tracked files over 1 MB:"; echo "$big"; bad=1; fi
exit $bad
