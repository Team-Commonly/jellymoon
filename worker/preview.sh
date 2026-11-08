#!/usr/bin/env bash
# A per-run Workers preview: deploys this checkout's test entry (src/test/index.ts, ENV unset) to its own Worker, its own Durable
# Object namespace and its own D1 database, runs the contract suite against it over HTTPS and the pod WebSocket, and removes both.
#   preview.sh need-secrets  fail unless JM_CF_PREVIEW_TOKEN and JM_CF_ACCOUNT_ID are both set (the CI job's first step; a skip would pass untested)
#   preview.sh up <name>    create the database, apply schema.sql, deploy, set the bearer, wait for /healthz
#   preview.sh run          run the suite against $JM_E2E_URL with $JM_E2E_TEST_BEARER (from `up`)
#   preview.sh down <name>  delete the Worker and the database, then check both are gone; safe after a partial `up`
#   preview.sh all <name>   up, run, down (down always runs); the laptop equivalent of the CI job (`make preview PREVIEW=<name>`)
# <name> is pr-<number> or main, so the Worker and the database are jellymoon-spike-<name>: nothing else is ever deployed or deleted.
# workers.dev only: no routes, zones or DNS. Takes JM_CF_PREVIEW_TOKEN, and optionally JM_CF_ACCOUNT_ID, from the environment only.
# Without an account id it asks the API, and stops unless the token sees exactly one account. Neither the token, the account id
# nor the workers.dev subdomain is printed: wrangler's output goes to a log, and what is shown of it is scrubbed.
set -euo pipefail
cd "$(dirname "$0")"
ROOT=$(cd .. && pwd)
export PATH="$ROOT/.tools/bin:$PATH"
CFG=.preview-wrangler.toml
API=https://api.cloudflare.com/client/v4
die() { echo "preview: $*" >&2; exit 1; }

setup() {
  [[ "${1:-}" =~ ^(main|pr-[0-9]+)$ ]] || die "name must be pr-<number> or main, got '${1:-}'"
  NAME="jellymoon-spike-$1"
  [ -n "${JM_CF_PREVIEW_TOKEN:-}" ] || die "JM_CF_PREVIEW_TOKEN is not set"
  TOKEN=$JM_CF_PREVIEW_TOKEN
  unset CLOUDFLARE_API_KEY CLOUDFLARE_EMAIL
  WORK=$(mktemp -d)
  LOG="$WORK/wrangler.log"
  export CLOUDFLARE_API_TOKEN=$TOKEN WRANGLER_SEND_METRICS=false NO_UPDATE_NOTIFIER=1 CI=1 XDG_CONFIG_HOME="$WORK/cfg" WRANGLER_LOG_PATH="$WORK/logs"
  ACCT=${JM_CF_ACCOUNT_ID:-}
  if [ -z "$ACCT" ]; then
    local list n
    list=$(cf /accounts) || die "the token cannot list its account; set JM_CF_ACCOUNT_ID or fix the token"
    n=$(jq '.result | length' <<< "$list")
    [ "$n" = 1 ] || die "the token lists $n accounts, not exactly 1; set JM_CF_ACCOUNT_ID"
    ACCT=$(jq -r '.result[0].id' <<< "$list")
  fi
  [ "$ACCT" != null ] && [ -n "$ACCT" ] || die "the token cannot list its account; set JM_CF_ACCOUNT_ID or fix the token"
  export CLOUDFLARE_ACCOUNT_ID=$ACCT
}

# cf <path>: GET on the API with the token passed on stdin, never in argv.
cf() { printf 'header = "Authorization: Bearer %s"\n' "$TOKEN" | curl -sS --fail-with-body -K - "$API$1"; }

scrub() { sed -e "s/${ACCT:-NOACCOUNT}/<account>/g" -e "s/${SUB:-NOSUBDOMAIN}/<subdomain>/g" -e "s/${TOKEN:-NOTOKEN}/<token>/g"; }
wr() { bun "$ROOT/node_modules/wrangler/bin/wrangler.js" "$@" >> "$LOG" 2>&1 || { echo "preview: wrangler $1 $2 failed:" >&2; tail -n 15 "$LOG" | scrub >&2; return 1; }; }

up() {
  setup "$1"
  down_quiet
  wr d1 create "$NAME" || die "cannot create the database"
  DBID=$(cf "/accounts/$ACCT/d1/database?name=$NAME" | jq -r --arg n "$NAME" '.result[] | select(.name == $n) | .uuid')
  [ -n "$DBID" ] || die "the database $NAME is not listed after create"
  local sed_cfg="s/^name = \"jellymoon\"/name = \"$NAME\"/; s/^database_name = \"jellymoon\"/database_name = \"$NAME\"/; s/^database_id = .*/database_id = \"$DBID\"/"
  sed "$sed_cfg" wrangler.toml > "$CFG"
  [ "$(grep -c "$NAME" "$CFG")" = 2 ] && grep -q "$DBID" "$CFG" || die "could not point the config at $NAME"
  wr d1 execute "$NAME" --remote --yes -c "$CFG" --file=schema.sql || die "cannot apply schema.sql"
  BEARER=$(openssl rand -hex 24)
  [ -z "${GITHUB_ACTIONS:-}" ] || echo "::add-mask::$BEARER"
  wr deploy -c "$CFG" || die "cannot deploy"
  printf '%s' "$BEARER" | wr secret put TEST_BEARER -c "$CFG" || die "cannot set the test bearer"
  SUB=$(cf "/accounts/$ACCT/workers/subdomain" | jq -r '.result.subdomain')
  [ -n "$SUB" ] && [ "$SUB" != null ] || die "the account has no workers.dev subdomain"
  [ -z "${GITHUB_ACTIONS:-}" ] || echo "::add-mask::$SUB"
  JM_E2E_URL="https://$NAME.$SUB.workers.dev"
  export JM_E2E_URL JM_E2E_TEST_BEARER=$BEARER
  [ -z "${GITHUB_ENV:-}" ] || printf 'JM_E2E_URL=%s\nJM_E2E_TEST_BEARER=%s\n' "$JM_E2E_URL" "$BEARER" >> "$GITHUB_ENV"
  for _ in $(seq 60); do curl -sf -o /dev/null "$JM_E2E_URL/healthz" && return 0; sleep 2; done
  die "$NAME did not answer /healthz with 200 within 2 minutes"
}

run() {
  [ -n "${JM_E2E_URL:-}" ] && [ -n "${JM_E2E_TEST_BEARER:-}" ] || die "run needs JM_E2E_URL and JM_E2E_TEST_BEARER (set by up)"
  "$ROOT/e2e/judge.sh"
}

down_quiet() {
  wr delete "$NAME" --force > /dev/null 2>&1 || true
  wr d1 delete "$NAME" -y > /dev/null 2>&1 || true
  rm -f "$CFG"
}

down() {
  setup "$1"
  down_quiet
  cf "/accounts/$ACCT/workers/scripts" | jq -e --arg n "$NAME" '[.result[].id] | index($n) == null' > /dev/null || die "the Worker $NAME is still there after delete"
  cf "/accounts/$ACCT/d1/database?name=$NAME" | jq -e --arg n "$NAME" '[.result[].name] | index($n) == null' > /dev/null || die "the database $NAME is still there after delete"
  echo "preview: $NAME and its database are gone"
}

need_secrets() {
  [ -n "${JM_CF_PREVIEW_TOKEN:-}" ] && [ -n "${JM_CF_ACCOUNT_ID:-}" ] || die "JM_CF_PREVIEW_TOKEN and JM_CF_ACCOUNT_ID must both be set (repository secrets in CI); the preview cannot be tested without them"
}

case "${1:-}" in
  need-secrets) need_secrets ;;
  up) up "${2:-}" ;;
  run) run ;;
  down) down "${2:-}" ;;
  all)
    rc=0
    ( up "${2:-}" && run ) || rc=$?
    down "${2:-}" || rc=$?
    exit $rc ;;
  *) die "usage: preview.sh need-secrets|up|run|down|all <pr-N|main>" ;;
esac
