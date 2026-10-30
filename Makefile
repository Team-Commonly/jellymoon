# Jellymoon commands. Go and Bun are pinned in .tool-versions; `make tools` installs them into
# .tools/ (repo-local). CI installs the same versions with setup-go and setup-bun.
export PATH := $(CURDIR)/.tools/bin:$(PATH)
# GNU Make 3.81 (macOS) execs a simple recipe line directly, searching the PATH make started with,
# not the exported one, so recipes name the repo-local tools by path. CI has no .tools/ and uses PATH.
BUN := $(or $(wildcard $(CURDIR)/.tools/bin/bun),bun)
GO := $(or $(wildcard $(CURDIR)/.tools/bin/go),go)

.PHONY: e2e e2e-a worker-dev land verify-preflight tools gen check-gen test test-ts test-go entropy portable secrets secrets-range hooks preflight private secrets-all

tools:
	scripts/tools.sh

node_modules: package.json bun.lock
	$(BUN) install --frozen-lockfile
	@touch node_modules

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
	$(BUN) scripts/check-land.mjs
	$(BUN) scripts/check-land-e2e.mjs
	$(BUN) run check:types
	$(BUN) test podcore worker

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

# land only: secrets in a commit range, ignoring gitleaks:allow markers (RANGE=<main tip>..<head>). It can only refuse. A finding makes
# gitleaks exit 3, which `go run` reports on stderr as "exit status 3"; any other failure exits 1, so land can name which it was.
secrets-range:
	$(GO) run $(GITLEAKS) git --log-opts="-m $(RANGE)" --ignore-gitleaks-allow --exit-code 3 --redact --no-banner .

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


# The black-box contract suite (docs/development/e2e-contract.md). Needs JM_E2E_SERVER; ends with E2E_MS=<n> STEPS=<k>/<k>.
e2e:
	$(GO) test ./e2e -count=1 -v

# The Cloudflare adapter (worker/) under `wrangler dev --local`. Loopback only.
worker-dev: node_modules
	ENV=local PORT=$${PORT:-8787} JM_DATA_DIR=$${JM_DATA_DIR:-$(CURDIR)/.wrangler-data} worker/dev.sh

# The contract suite against the Cloudflare adapter. It persists every command, heartbeats included (staleMs is 0).
# It also judges the closing line, which no test can see: the last line before go test's own ok/FAIL/PASS must be
# E2E_MS=<n> STEPS=<k>/<n>, with k = n when the suite passed and k < n when it did not.
e2e-a: node_modules
	@out=$$(mktemp); \
	JM_E2E_SERVER='$(CURDIR)/worker/dev.sh' $(GO) test ./e2e -count=1 -v > "$$out" 2>&1; rc=$$?; \
	cat "$$out"; \
	last=$$(grep -v -E '^(ok|FAIL|PASS)([[:space:]]|$$)|^$$' "$$out" | tail -1); \
	rm -f "$$out"; \
	echo "$$last" | grep -E -q '^E2E_MS=[0-9]+ STEPS=[0-9]+/[0-9]+$$' || { echo "e2e-a: the closing line is not E2E_MS=<n> STEPS=<k>/<n>: $$last"; exit 1; }; \
	k=$$(echo "$$last" | sed -E 's/.*STEPS=([0-9]+)\/([0-9]+)/\1/'); n=$$(echo "$$last" | sed -E 's/.*STEPS=([0-9]+)\/([0-9]+)/\2/'); \
	if [ $$rc -eq 0 ] && [ "$$k" != "$$n" ]; then echo "e2e-a: green run but STEPS=$$k/$$n"; exit 1; fi; \
	if [ $$rc -ne 0 ] && [ "$$k" = "$$n" ]; then echo "e2e-a: red run but STEPS=$$k/$$n"; exit 1; fi; \
	exit $$rc

preflight: portable check-gen test entropy secrets private e2e-a
