# Jellymoon commands. Go and Bun are pinned in .tool-versions; `make tools` installs them into
# .tools/ (repo-local). CI installs the same versions with setup-go and setup-bun.
export PATH := $(CURDIR)/.tools/bin:$(PATH)

.PHONY: tools gen check-gen test test-ts test-go entropy portable secrets hooks preflight

tools:
	scripts/tools.sh

node_modules: package.json bun.lock
	bun install --frozen-lockfile
	@touch node_modules

# Regenerate every contract from schema/. Never hand-edit gen/ or protocol/gen.go.
gen: node_modules
	bun scripts/gen.mjs
	gofmt -w daemon/internal/protocol/gen.go

# Fails if schema/ changed without `make gen`, or gen/ was edited by hand.
check-gen: gen
	git diff --exit-code -- gen daemon/internal/protocol/gen.go
	@test -z "$$(git ls-files --others --exclude-standard gen daemon/internal/protocol)" || (echo "untracked generated files; commit them"; exit 1)

test-ts: node_modules
	bun scripts/check-fixtures.mjs
	bun run check:types

test-go:
	go vet ./...
	go test -race ./...

test: test-ts test-go

# Entropy gate (docs/development/workflow.md): budgets and ratchets in entropy.json,
# dead code (knip, staticcheck, deadcode), TS module boundaries (dependency-cruiser).
STATICCHECK := honnef.co/go/tools/cmd/staticcheck@v0.8.2
DEADCODE := golang.org/x/tools/cmd/deadcode@v0.51.0
entropy: node_modules
	bun run check:entropy
	bun run check:dead
	bun run check:boundaries
	go run $(STATICCHECK) ./...
	@if [ -n "$$(go list -f '{{if eq .Name "main"}}{{.ImportPath}}{{end}}' ./...)" ]; then go run $(DEADCODE) -test ./...; fi

# No secrets or infrastructure ids, from the first commit (gitleaks over the full history).
GITLEAKS := github.com/zricethezav/gitleaks/v8@v8.30.1
secrets:
	go run $(GITLEAKS) git --log-opts=HEAD --redact --no-banner .

# Per worktree: pre-commit runs the fast gates; fence=1 also blocks pushes (worker worktrees).
hooks:
	git config extensions.worktreeConfig true
	git config --worktree core.hooksPath .githooks
	@if [ "$(fence)" = "1" ]; then git config --worktree commonly.pushFence 1; echo "push fence on"; fi

# Works from a fresh clone anywhere: no symlink out of the repo, no machine-local path.
portable:
	scripts/check-portable.sh

preflight: portable check-gen test entropy secrets
