#!/usr/bin/env bash
# One deploy of the hosted instance, run only by .github/workflows/hosted.yml (never by hand: docs/runbooks/hosted-instance.md).
#   ship.sh <hosted dir> <secrets file> <sha> <message>
# Runs `wrangler deploy` from <hosted dir> (the runner's Docker builds the image), parses the workers.dev URL, waits for /healthz
# (10 minutes at most), then runs the read-only live check (make deployed-check, ADR 0006). Prints url=<base> to $GITHUB_OUTPUT.
# Any failure exits non-zero; the workflow decides whether to roll back.
set -euo pipefail
dir=$1 secrets=$2 sha=$3 message=$4
repo=$(cd "$dir/.." && pwd)
log=$(mktemp)

cd "$dir"
node_modules/.bin/wrangler --version
# wrangler's own output, unfiltered: a missing token permission is named here. It never prints secret values.
node_modules/.bin/wrangler deploy --secrets-file "$secrets" --var "JM_BUILD:$sha" --tag "${sha:0:12}" --message "$message" 2>&1 | tee "$log"
url=$(grep -oE 'https://[A-Za-z0-9.-]+\.workers\.dev' "$log" | head -n1 || true)
if [ -z "$url" ]; then echo "::error::no workers.dev URL in the deploy output"; exit 1; fi
echo "url=$url" >> "${GITHUB_OUTPUT:-/dev/null}"
echo "Deployed $sha to $url"

# A deploy starts the container rollout; it does not wait for it (https://developers.cloudflare.com/containers/platform-details/rollouts/),
# so the old container can answer /healthz for a while. Health counts only from the container that reports this commit in
# X-Jm-Build (server/src/app.ts); a new container that never comes up fails here, and the workflow rolls back.
start=$(date +%s)
while :; do
  hdr=$(curl -q -s -D - -o /dev/null --max-time 30 "$url/healthz" || true)
  code=$(printf '%s' "$hdr" | awk 'NR==1 {print $2}')
  build=$(printf '%s' "$hdr" | tr -d '\r' | awk -F': ' 'tolower($1)=="x-jm-build" {print $2}')
  elapsed=$(( $(date +%s) - start ))
  if [ "$code" = 200 ] && [ "$build" = "$sha" ]; then echo "healthz 200 from $sha after ${elapsed}s"; break; fi
  if [ "$elapsed" -ge 600 ]; then echo "::error::no healthz 200 from $sha after ${elapsed}s (last code ${code:-none}, build ${build:-none})"; exit 1; fi
  echo "healthz ${code:-none} from ${build:-unknown build} after ${elapsed}s; waiting for $sha"
  sleep 10
done

make -C "$repo" --no-print-directory deployed-check DEPLOYED_URL="$url"
