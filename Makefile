# Jellymoon commands. Go and Bun are pinned in .tool-versions; `make tools` installs them into
# .tools/ (repo-local). CI installs the same versions with setup-go and setup-bun.
export PATH := $(CURDIR)/.tools/bin:$(PATH)
# GNU Make 3.81 (macOS) execs a simple recipe line directly, searching the PATH make started with,
# not the exported one, so recipes name the repo-local tools by path. CI has no .tools/ and uses PATH.
BUN := $(or $(wildcard $(CURDIR)/.tools/bin/bun),bun)
GO := $(or $(wildcard $(CURDIR)/.tools/bin/go),go)

.PHONY: acp-tools record-acp record-acp-codex e2e e2e-a e2e-a-migrated e2e-c e2e-any e2e-deployed e2e-web deployed-boot server worker-dev land verify-preflight tools tools-cache gen check-gen test test-ts test-worker test-go entropy portable secrets secrets-range hooks preflight private secrets-all

tools:
	scripts/tools.sh

# The shared toolchain cache refuses a root or entry anyone but you could have written (fixtures only; no download).
tools-cache:
	scripts/check-tools-cache.sh

node_modules: package.json bun.lock
	$(BUN) install --frozen-lockfile
	@touch node_modules

# The real Claude Code and Codex adapters for scripts/record-acp.mjs, pinned exactly, in this clone's own ignored prefix. Never global.
ACP_ADAPTERS := @agentclientprotocol/claude-agent-acp@0.99.1 @agentclientprotocol/codex-acp@2.6.0
acp-tools:
	npm install --prefix .tools/acp --ignore-scripts --save-exact --no-audit --no-fund $(ACP_ADAPTERS)

# Records a fresh daemon/testdata/acp/claude-slice.jsonl with a real Claude Code run (docs/development/record-acp.md).
record-acp: acp-tools
	$(BUN) scripts/record-acp.mjs

# The same for daemon/testdata/acp/codex-slice.jsonl, with a real Codex run.
record-acp-codex: acp-tools
	$(BUN) scripts/record-acp.mjs --harness codex

# Regenerate every contract from schema/. Never hand-edit gen/, protocol/gen.go or docs/protocol.md.
gen: node_modules
	$(BUN) scripts/gen.mjs

# Fails if schema/ changed without `make gen`, or gen/ was edited by hand.
check-gen: gen
	git diff --exit-code -- gen daemon/internal/protocol/gen.go docs/protocol.md
	@test -z "$$(git ls-files --others --exclude-standard gen daemon/internal/protocol docs/protocol.md)" || (echo "untracked generated files; commit them"; exit 1)

test-ts: node_modules
	$(BUN) scripts/check-fixtures.mjs
	$(BUN) scripts/check-review-gate.mjs
	$(BUN) scripts/check-scan-history.mjs
	$(BUN) scripts/check-scan-checkout.mjs
	$(BUN) scripts/check-land.mjs
	$(BUN) scripts/check-land-scan.mjs
	$(BUN) scripts/check-land-e2e.mjs
	$(BUN) scripts/check-land-batch.mjs
	$(BUN) scripts/check-land-batch-e2e.mjs
	$(BUN) run check:types
	$(BUN) test podcore worker server
	$(BUN) run test:scrub
	$(BUN) run test:web

# The Cloudflare adapter's unit tests alone (CI's `worker` job). bun test exits 1 when it finds no test, so an empty run is red.
test-worker: node_modules
	$(BUN) test worker

test-go:
	$(GO) vet ./...
	$(GO) test -race ./...

test: test-ts test-go

# Entropy gate (docs/development/workflow.md): budgets and ratchets in entropy.json,
# dead code (knip, staticcheck, deadcode), TS module boundaries (dependency-cruiser).
STATICCHECK := honnef.co/go/tools/cmd/staticcheck@v0.8.2
DEADCODE := golang.org/x/tools/cmd/deadcode@v0.51.0
entropy: node_modules
	$(BUN) run check:entropy
	$(BUN) run check:dead
	$(BUN) run check:boundaries
	$(BUN) run check:boundary-rule
	$(GO) run $(STATICCHECK) ./...
	@if [ -n "$$($(GO) list -f '{{if eq .Name "main"}}{{.ImportPath}}{{end}}' ./...)" ]; then $(GO) run $(DEADCODE) -test ./...; fi

# No secrets or infrastructure ids, from the first commit (gitleaks over the full history).
GITLEAKS := github.com/zricethezav/gitleaks/v8@v8.30.1
secrets:
	$(GO) run $(GITLEAKS) git --log-opts="-m HEAD" --redact --no-banner .

# For people: the command land's range scan runs (scripts/land-scan.mjs), over RANGE=<main tip>..<head>, ignoring gitleaks:allow markers.
# Land does not run this target; check-review-gate checks the two are the same command. Like land, it passes its own config (gitleaks'
# built-in rules, written to a fresh directory under TMPDIR) and an empty ignore directory, so no .gitleaks.* in the checkout is read.
# A finding makes gitleaks exit 3, which `go run` reports on stderr as "exit status 3"; any other failure exits 1.
secrets-range:
	d=$$(mktemp -d) || exit 1; printf '[extend]\nuseDefault = true\n' > "$$d/gitleaks.toml" && mkdir "$$d/empty" && $(GO) run $(GITLEAKS) git --log-opts="-m $(RANGE)" --ignore-gitleaks-allow --exit-code 3 --redact --no-banner --config "$$d/gitleaks.toml" --gitleaks-ignore-path "$$d/empty" .; rc=$$?; rm -f "$$d/gitleaks.toml"; rmdir "$$d/empty" "$$d"; exit $$rc

# Per worktree: pre-commit runs the fast gates; fence=1 also blocks pushes (worker worktrees).
hooks:
	git config extensions.worktreeConfig true
	git config --worktree core.hooksPath .githooks
	@if [ "$(fence)" = "1" ]; then git config --worktree commonly.pushFence 1; echo "push fence on"; fi

# Works from a fresh clone anywhere: no symlink out of the repo, no machine-local path.
portable:
	scripts/check-portable.sh

# The only merge path (docs/development/landing.md): make land PR=<n> [ARGS=--dry-run]. Needs the JM_* env.
land:
	node scripts/land.mjs $(PR) $(ARGS)

# Verifiers: the trust gate, then the merge-result preflight of PR=<n> and nothing else (no ledger, no pod).
# Needs JM_REPO and JM_TRUSTED_LOGINS. Run it from a clean checkout of main, never the PR's tree (its Makefile and scripts
# would run before the trust gate decides). An untrusted PR is refused before any of its code is fetched.
verify-preflight:
	node scripts/land.mjs $(PR) --preflight-only $(ARGS)

# Tracked files carry nothing private (scripts/check-private.mjs holds patterns only).
private:
	$(BUN) scripts/check-private.mjs
	@# an unmutated judge of the exit path: the check must refuse a planted hit
	@printf 'TASK-%s\n' 5 > .planted-hit.tmp; if $(BUN) scripts/check-private.mjs .planted-hit.tmp >/dev/null 2>&1; then rm -f .planted-hit.tmp; echo 'private: a planted hit was not refused'; exit 1; fi; rm -f .planted-hit.tmp

# Before the repo goes public, and in CI: every ref, every commit (-m so merges count), plus the tracked tree (one scan per top-level entry, so untracked tools are not read). A clone that carries
# other repositories' refs narrows it: make secrets-all SCAN_REFS="HEAD --remotes=<remote>/".
SCAN_REFS ?= --all
secrets-all:
	$(GO) run $(GITLEAKS) git --log-opts="-m $(SCAN_REFS)" --redact --no-banner .
	@git -c core.quotepath=off ls-files | cut -d/ -f1 | sort -u | while IFS= read -r p; do $(GO) run $(GITLEAKS) dir --redact --no-banner "$$p" </dev/null || exit 1; done


# The black-box contract suite against any server: `JM_E2E_SERVER='<shell command>' make e2e-any` (docs/development/e2e-contract.md).
# With JM_E2E_SERVER unset every test skips.
e2e-any:
	$(GO) test ./e2e -count=1 -v

# The pod-state server (ADR 0006): one Bun process, Hono, Postgres. `make server` serves on $PORT (default 8787) with the
# in-process Postgres kept in .tools/server-data; set DATABASE_URL to use a real one.
server: node_modules
	$(if $(DATABASE_URL),,ENV=local) PORT=$${PORT:-8787} $(BUN) server/src/main.ts $(if $(DATABASE_URL),,--persist-to $(CURDIR)/.tools/server-data)

# A fresh PGlite data directory as one tar, so each e2e server skips initdb. Rebuilt when the lockfile changes.
PGLITE_TPL := $(CURDIR)/.tools/pglite-template.tar
$(PGLITE_TPL): bun.lock | node_modules
	mkdir -p $(dir $@)
	$(BUN) server/src/pglite-template.ts $@

# The suite starts it with PORT, ENV=local and JM_DATA_DIR. The state dir sits under JM_DATA_DIR (TestF02 reads the flag from
# the process table). Against a real Postgres, each server gets its own schema, named from its data dir, so every test starts fresh.
SERVER_E2E := if [ -n "$$DATABASE_URL" ]; then export JM_PG_SCHEMA="e2e_$$(printf %s "$$JM_DATA_DIR" | cksum | tr -d -c 0-9)"; fi; exec $(BUN) server/src/main.ts --persist-to "$$JM_DATA_DIR/state"

# Runs the contract suite with the given server environment and judges the closing line, which no test can see: the last line
# before go test's own ok/FAIL/PASS must be E2E_MS=<n> STEPS=<k>/<n>, with k = n when the suite passed and k < n when it did not.
# $(1) names the target in messages. $(2) is the environment that picks the server.
define judge_e2e
	@out=$$(mktemp); \
	env $(2) $(GO) test ./e2e -count=1 -v > "$$out" 2>&1; rc=$$?; \
	cat "$$out"; \
	last=$$(grep -v -E '^(ok|FAIL|PASS)([[:space:]]|$$)|^$$' "$$out" | tail -1); \
	rm -f "$$out"; \
	echo "$$last" | grep -E -q '^E2E_MS=[0-9]+ STEPS=[0-9]+/[0-9]+$$' || { echo "$(1): the closing line is not E2E_MS=<n> STEPS=<k>/<n>: $$last"; exit 1; }; \
	k=$$(echo "$$last" | sed -E 's/.*STEPS=([0-9]+)\/([0-9]+)/\1/'); n=$$(echo "$$last" | sed -E 's/.*STEPS=([0-9]+)\/([0-9]+)/\2/'); \
	if [ $$rc -eq 0 ] && [ "$$k" != "$$n" ]; then echo "$(1): green run but STEPS=$$k/$$n"; exit 1; fi; \
	if [ $$rc -ne 0 ] && [ "$$k" = "$$n" ]; then echo "$(1): red run but STEPS=$$k/$$n"; exit 1; fi; \
	exit $$rc
endef

# The contract suite against the pod-state server. JM_E2E_SKIP is set from E2E_C_SKIP below, so an inherited value never applies. It persists every command, heartbeats included (staleMs is 0). The tests that
# need the Cloudflare adapter's object routes are skipped, each with its reason (docs/development/e2e-contract.md, "Skipped on the server").
E2E_C_SKIP := TestEvictMidClaim,TestEvictAlarmWake,TestEvictPastDeadline
e2e-c: node_modules $(if $(DATABASE_URL),,$(PGLITE_TPL))
	$(call judge_e2e,e2e-c,JM_E2E_SERVER='$(SERVER_E2E)' JM_E2E_SKIP='$(E2E_C_SKIP)' $(if $(DATABASE_URL),JM_E2E_STATE_IN_DB=1,JM_PGLITE_TEMPLATE='$(PGLITE_TPL)'))

# `make e2e` is the contract suite against the server that ships.
e2e: e2e-c

# The contract suite against the Cloudflare adapter (worker/), which stays in the tree as a second adapter.
# JM_E2E_SKIP is cleared here, so an inherited skip list never drops tests without a word. The skips below are
# deliberate: features built on the pod-state server only (ADR 0006), each with its reason in
# docs/development/e2e-contract.md, "Skipped on the Worker". Override with `make e2e-a E2E_A_SKIP=<names>`.
E2E_A_SKIP := TestF19,TestF21,TestF22,TestF25
e2e-a: node_modules
	$(call judge_e2e,e2e-a,JM_E2E_SERVER='$(CURDIR)/worker/dev.sh' JM_E2E_SKIP='$(E2E_A_SKIP)')

# The sign-in suites against a Worker whose D1 was created from the schema before the sign-in hardening and then migrated
# (worker/migrations, `wrangler d1 migrations apply --local` on the suite's own state dirs). Loopback only.
e2e-a-migrated: node_modules
	JM_E2E_SERVER='JM_D1_SEED=migrations/0000_initial.sql $(CURDIR)/worker/dev.sh' JM_E2E_SKIP= $(GO) test ./e2e -count=1 -v -run '^(TestF15|TestBSESS|TestAUTHH)$$'

# The Cloudflare adapter (worker/) under `wrangler dev --local`. Loopback only.
worker-dev: node_modules
	ENV=local PORT=$${PORT:-8787} JM_DATA_DIR=$${JM_DATA_DIR:-$(CURDIR)/.wrangler-data} worker/dev.sh

# The server the way a self-hoster runs it (ADR 0006, "Deployed form"): the release bundle, one Bun process, config from env.
# `make e2e-deployed` builds the bundle, boots it as its own process (deployed-boot, below), then runs the contract suite with the
# bundle as the server under test. With DATABASE_URL set it is the hosted shape: Postgres, ENV unset, test routes behind a bearer.
# Unset, it falls back to the in-process Postgres (ENV=local) so a laptop with no Postgres can still run it. pglite stays external to
# the bundle: it is the local fallback only, and its wasm files are not part of a bundle.
DEPLOYED := $(CURDIR)/.tools/deployed
DEPLOYED_BUNDLE := $(DEPLOYED)/server.js
SERVER_DEPLOYED := if [ -n "$$DATABASE_URL" ]; then export JM_PG_SCHEMA="e2e_$$(printf %s "$$JM_DATA_DIR" | cksum | tr -d -c 0-9)"; fi; exec $(BUN) $(DEPLOYED_BUNDLE) --persist-to "$$JM_DATA_DIR/state"

e2e-deployed: node_modules $(if $(DATABASE_URL),,$(PGLITE_TPL))
	mkdir -p $(DEPLOYED)
	$(BUN) build server/src/main.ts --target=bun --external @electric-sql/pglite --outfile $(DEPLOYED_BUNDLE)
	@$(MAKE) --no-print-directory deployed-boot
	$(call judge_e2e,e2e-deployed,JM_E2E_SERVER='$(SERVER_DEPLOYED)' JM_E2E_SKIP='$(E2E_C_SKIP)' $(if $(DATABASE_URL),JM_E2E_STATE_IN_DB=1,JM_PGLITE_TEMPLATE='$(PGLITE_TPL)'))

# The browser specs (docs/development/browser-specs.md): builds the web client into web/dist, then runs web-e2e/ under Playwright.
# Its global setup starts the pod-state server itself (ENV=local, a fresh PGlite dir, a free port), serving that build. It needs Node
# and the pinned Chromium; CI's browser job installs the browser in the job. Never install a browser on a laptop for this.
e2e-web: node_modules $(PGLITE_TPL)
	$(BUN) run build:web
	BUN=$(BUN) JM_PGLITE_TEMPLATE=$(PGLITE_TPL) node_modules/.bin/playwright test -c web-e2e

# Boots the bundle as its own process and drives it by URL only: it must refuse to start with no database, answer /healthz, keep the
# test routes closed in the hosted shape (no bearer is 401, no clock route), serve a user's pod through the real routes, and keep that
# pod across kill -9 and a second start. Any failed check, or a server that never answers, fails the target; nothing skips.
# A failing Postgres (a wrong DATABASE_URL) makes the server exit at boot, and that fails this target too.
deployed-boot:
	@set -u; \
	bun="$(BUN)"; bundle="$(DEPLOYED_BUNDLE)"; dir=$$(mktemp -d); log="$$dir/server.log"; pid=0; \
	port=$$($(BUN) -e 'const s = Bun.serve({ port: 0, fetch() { return new Response(); } }); console.log(s.port); s.stop(true);'); \
	code() { curl -s -o /dev/null -w '%{http_code}' "$$@"; }; \
	want() { [ "$$2" = "$$3" ] || { echo "deployed-boot: $$1: got $$2, want $$3"; tail -20 "$$log"; exit 1; }; echo "deployed-boot: ok $$1 -> $$2"; }; \
	trap 'if [ "$$pid" -gt 0 ]; then kill $$pid 2>/dev/null; wait $$pid 2>/dev/null; fi; tmp="$${TMPDIR:-/tmp}"; tmp="$${tmp%/}"; case "$$dir" in "$$tmp"/?*|/tmp/?*) [ -d "$$dir" ] && rm -rf -- "$$dir";; esac' EXIT; \
	out=$$(env -u ENV -u DATABASE_URL PORT=$$port "$$bun" "$$bundle" 2>&1); rc=$$?; \
	if [ $$rc -eq 0 ] || ! echo "$$out" | grep -q 'DATABASE_URL is required'; then echo "deployed-boot: with no ENV and no DATABASE_URL the server must refuse to start (rc=$$rc)"; exit 1; fi; \
	echo "deployed-boot: ok the server refuses to start with no database (rc=$$rc)"; \
	if [ -n "$${DATABASE_URL:-}" ]; then mode=hosted; bearer=$$(od -An -N24 -tx1 /dev/urandom | tr -d ' \n'); auth="authorization: Bearer $$bearer"; \
	  start() { env -u ENV -u JM_PG_SCHEMA PORT=$$port TEST_BEARER=$$bearer "$$bun" "$$bundle" >> "$$log" 2>&1 & pid=$$!; }; \
	else mode=local; bearer=""; auth="x-smoke: local"; \
	  start() { env ENV=local PORT=$$port "$$bun" "$$bundle" --persist-to "$$dir/state" >> "$$log" 2>&1 & pid=$$!; }; fi; \
	up() { i=0; until curl -fsS -o /dev/null "http://127.0.0.1:$$port/healthz" 2>/dev/null; do i=$$((i+1)); \
	  if ! kill -0 $$pid 2>/dev/null; then echo "deployed-boot: the server exited before /healthz answered; log tail:"; tail -20 "$$log"; exit 1; fi; \
	  if [ $$i -gt 600 ]; then echo "deployed-boot: no /healthz 200 within 60 s; log tail:"; tail -20 "$$log"; exit 1; fi; sleep 0.1; done; }; \
	base="http://127.0.0.1:$$port"; json='content-type: application/json'; \
	start; up; echo "deployed-boot: the server is its own process: pid=$$pid port=$$port mode=$$mode"; \
	want "GET /healthz" "$$(code $$base/healthz)" 200; \
	if [ $$mode = hosted ]; then \
	  want "POST /__test/session without the bearer" "$$(code -X POST -H "$$json" -d '{"name":"x"}' $$base/__test/session)" 401; \
	  want "GET /__test/clock with the bearer (the hosted shape has no clock route)" "$$(code -H "$$auth" $$base/__test/clock)" 404; \
	else \
	  want "GET /__test/clock (local shape)" "$$(code $$base/__test/clock)" 200; \
	fi; \
	tok=$$(curl -s -X POST -H "$$auth" -H "$$json" -d '{"name":"smoke"}' $$base/__test/session | sed -n 's/.*"token":"\(jmu_[^"]*\)".*/\1/p'); \
	if [ -z "$$tok" ]; then echo "deployed-boot: /__test/session gave no jmu_ token"; tail -20 "$$log"; exit 1; fi; \
	podid=$$(curl -s -X POST -H "authorization: Bearer $$tok" -H "$$json" -d '{"name":"deployed-smoke"}' $$base/v1/pods | sed -n 's/.*"id":"\(pod_[^"]*\)".*/\1/p'); \
	if [ -z "$$podid" ]; then echo "deployed-boot: POST /v1/pods gave no pod id"; tail -20 "$$log"; exit 1; fi; \
	want "GET /v1/pods/<pod>/me as its owner" "$$(code -H "authorization: Bearer $$tok" $$base/v1/pods/$$podid/me)" 200; \
	want "GET /v1/pods/<pod>/me with no token" "$$(code $$base/v1/pods/$$podid/me)" 401; \
	kill -9 $$pid; wait $$pid 2>/dev/null; start; up; echo "deployed-boot: restarted after kill -9: pid=$$pid port=$$port"; \
	want "GET /v1/pods/<pod>/me after the restart" "$$(code -H "authorization: Bearer $$tok" $$base/v1/pods/$$podid/me)" 200; \
	echo "deployed-boot: all checks passed"

preflight: portable tools-cache check-gen test entropy secrets private e2e-c
