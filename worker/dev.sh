#!/usr/bin/env bash
# `make worker-dev`: the Worker under `wrangler dev --local`, the start command for the e2e suite (JM_E2E_SERVER).
# Reads PORT and JM_DATA_DIR. Loopback only: no telemetry, no update check, no Cloudflare credentials.
set -euo pipefail
cd "$(dirname "$0")"
: "${PORT:?PORT is required}"
: "${JM_DATA_DIR:?JM_DATA_DIR is required}"
mkdir -p "$JM_DATA_DIR/cfg" "$JM_DATA_DIR/logs"
unset CLOUDFLARE_API_TOKEN CLOUDFLARE_ACCOUNT_ID CLOUDFLARE_API_KEY CLOUDFLARE_EMAIL
export WRANGLER_SEND_METRICS=false NO_UPDATE_NOTIFIER=1 CI=1 XDG_CONFIG_HOME="$JM_DATA_DIR/cfg" WRANGLER_LOG_PATH="$JM_DATA_DIR/logs/wrangler"
exec bun ../node_modules/wrangler/bin/wrangler.js dev --local --ip 127.0.0.1 --port "$PORT" --persist-to "$JM_DATA_DIR/state" \
  --var "ENV:${ENV:-local}" --var "BOOT_SWEEP:${BOOT_SWEEP:-1}" --show-interactive-dev-session=false
