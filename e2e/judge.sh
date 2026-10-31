#!/usr/bin/env bash
# Runs the contract suite and judges its closing line, which no test can see. The last line before go test's own ok/FAIL/PASS
# must be E2E_MS=<n> STEPS=<k>/<n>, with k = n when the suite passed and k < n when it did not. The target is the environment:
# JM_E2E_SERVER (a local server, make e2e-a) or JM_E2E_URL plus JM_E2E_TEST_BEARER (a deployed one, worker/preview.sh).
# JM_E2E_RUN, when set, is go test's -run pattern (the CI preview step sets it to name the tests it must run).
set -uo pipefail
cd "$(dirname "$0")/.."
out=$(mktemp)
go test ./e2e -count=1 -v ${JM_E2E_RUN:+-run "$JM_E2E_RUN"} > "$out" 2>&1
rc=$?
cat "$out"
last=$(grep -v -E '^(ok|FAIL|PASS)([[:space:]]|$)|^$' "$out" | tail -1)
rm -f "$out"
echo "$last" | grep -E -q '^E2E_MS=[0-9]+ STEPS=[0-9]+/[0-9]+$' || { echo "e2e: the closing line is not E2E_MS=<n> STEPS=<k>/<n>: $last"; exit 1; }
k=$(echo "$last" | sed -E 's/.*STEPS=([0-9]+)\/([0-9]+)/\1/')
n=$(echo "$last" | sed -E 's/.*STEPS=([0-9]+)\/([0-9]+)/\2/')
if [ "$n" = 0 ]; then echo "e2e: STEPS=0/0, no test ran against the server"; exit 1; fi
if [ $rc -eq 0 ] && [ "$k" != "$n" ]; then echo "e2e: green run but STEPS=$k/$n"; exit 1; fi
if [ $rc -ne 0 ] && [ "$k" = "$n" ]; then echo "e2e: red run but STEPS=$k/$n"; exit 1; fi
exit $rc
