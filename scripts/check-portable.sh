#!/usr/bin/env bash
# The repo must work from a fresh clone on any machine: no tracked symlink may leave the
# repo, no tracked file may name a home directory or a machine-local path, and no installed
# tool or dependency (.tools/, node_modules/) or file over 1 MB may be committed.
set -euo pipefail
cd "$(dirname "$0")/.."
root=$(pwd -P); bad=0
while IFS= read -r -d '' rec; do
  [ "${rec%% *}" = 120000 ] || continue
  f=${rec#*$'\t'} # NUL-separated records: a path with a space or a newline stays whole
  # the FULL target, followed physically through every component (a link inside a link included), must stay in the repo
  target=$(perl -MCwd=abs_path -e 'print abs_path($ARGV[0]) // "/unresolvable"' "$f")
  case "$target/" in "$root"/*) ;; *) echo "symlink leaves the repo: $f -> $(readlink "$f") (resolves to $target)"; bad=1 ;; esac
done < <(git ls-files -s -z)
# `/private/t[m]p` is written with a bracket so this line does not match its own pattern, and neither does any other tracked file's text of it
pattern='(~/[.A-Za-z]|/home/[a-z]|/opt/homebrew|/private/t[m]p|\$HOME/|\$\(HOME\)|(^|[^A-Za-z0-9_./-])/Users/[A-Za-z])'
if git grep -nIiE "$pattern" -- . ':!scripts/check-portable.sh'; then
  echo "machine-local path in a tracked file (above); use a repo-relative path"; bad=1
fi
# any path component named .tools or node_modules, in any case (APFS folds it), as a directory or as an entry itself (a symlink). The list is
# read whole before printing: piped into head under pipefail, a list longer than a pipe buffer failed the pipeline and passed the check.
# grep's "no match" (1) is the only failure let through; any other failure stops the script (set -e), which fails the check.
tracked=$(git ls-files -z | tr '\0' '\n' | { grep -iE '(^|/)(\.tools|node_modules)(/|$)' || [ $? = 1 ]; })
if [ -n "$tracked" ]; then
  head -3 <<<"$tracked"; echo "installed tools or dependencies are tracked (above); they belong in .gitignore"; bad=1
fi
big=$(git ls-files -s | awk '$1!="120000"{print $2" "$4}' | git cat-file --batch-check='%(objectsize) %(rest)' | awk '$1>1048576{print $2}')
if [ -n "$big" ]; then echo "tracked files over 1 MB:"; echo "$big"; bad=1; fi
exit $bad
