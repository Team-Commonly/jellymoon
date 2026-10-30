# Jellymoon commands. Go and Bun are pinned in .tool-versions; `make tools` installs them into
# .tools/ (repo-local). CI installs the same versions with setup-go and setup-bun.
export PATH := $(CURDIR)/.tools/bin:$(PATH)
# GNU Make 3.81 (macOS) execs a simple recipe line directly, searching the PATH make started with,
# not the exported one, so recipes name the repo-local tools by path. CI has no .tools/ and uses PATH.
BUN := $(or $(wildcard $(CURDIR)/.tools/bin/bun),bun)
GO := $(or $(wildcard $(CURDIR)/.tools/bin/go),go)

.PHONY: land verify-preflight tools gen check-gen test test-ts test-go entropy portable secrets secrets-range secrets-dir hooks preflight private secrets-all

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
	$(BUN) test podcore

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
	$(GO) run $(STATICCHECK) ./...
	@if [ -n "$$($(GO) list -f '{{if eq .Name "main"}}{{.ImportPath}}{{end}}' ./...)" ]; then $(GO) run $(DEADCODE) -test ./...; fi

# No secrets or infrastructure ids, from the first commit (gitleaks over the full history).
GITLEAKS := github.com/zricethezav/gitleaks/v8@v8.30.1
secrets:
	$(GO) run $(GITLEAKS) git --log-opts="-m HEAD" --redact --no-banner .

# land only: secrets in a commit range, ignoring gitleaks:allow markers (RANGE=<main tip>..<head>).
secrets-range:
	$(GO) run $(GITLEAKS) git --log-opts="-m $(RANGE)" --ignore-gitleaks-allow --redact --no-banner .

# land only: a directory of inert files copied under neutral names (DIR=<dir>); no config from the PR, markers ignored.
secrets-dir:
	$(GO) run $(GITLEAKS) dir --ignore-gitleaks-allow --redact --no-banner $(DIR)

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
private: node_modules
	$(BUN) scripts/check-private.mjs

# Before the repo goes public, and in CI: every ref, every commit (-m so merges count), plus the tracked tree (one scan per top-level entry, so untracked tools are not read). A clone that carries
# other repositories' refs narrows it: make secrets-all SCAN_REFS="HEAD --remotes=<remote>/".
SCAN_REFS ?= --all
secrets-all:
	$(GO) run $(GITLEAKS) git --log-opts="-m $(SCAN_REFS)" --redact --no-banner .
	@for p in $$(git ls-files | cut -d/ -f1 | sort -u); do $(GO) run $(GITLEAKS) dir --redact --no-banner "$$p" || exit 1; done

preflight: portable check-gen test entropy secrets private
