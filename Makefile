# Jellymoon commands. Rust is pinned in rust-toolchain.toml; ./init.sh installs it and gitleaks into .tools/.
# CI and Windows have no .tools/ and run the same cargo commands with the cargo on PATH.
RUST_HOME := $(or $(JM_RUST_HOME),$(CURDIR)/.tools)
ifneq ($(wildcard $(RUST_HOME)/cargo/bin/cargo),)
export RUSTUP_HOME := $(RUST_HOME)/rustup
export CARGO_HOME := $(RUST_HOME)/cargo
export PATH := $(RUST_HOME)/cargo/bin:$(PATH)
# Named by path: GNU Make 3.81 (macOS) looks a simple recipe up on the PATH make started with, not the exported one.
CARGO := $(RUST_HOME)/cargo/bin/cargo
else
CARGO := cargo
endif
GITLEAKS := $(or $(wildcard $(CURDIR)/.tools/bin/gitleaks),gitleaks)

# The draw tests render without a window, which GPUI can do on the Mac. GPUI compiles its Metal shaders at build time
# with Xcode's `metal`; a Mac with only the command-line tools has none, and compiles them when the app starts instead.
ifeq ($(shell uname -s),Darwin)
FEATURES := --features sticker-kit/headless
ifeq ($(shell xcrun -f metal 2>/dev/null),)
FEATURES := $(FEATURES),sticker-kit/runtime_shaders
endif
endif

.PHONY: preflight fmt fmt-check clippy check test secrets secrets-staged hooks

# What a pull request has to pass, and what CI runs.
preflight: fmt-check clippy check test secrets

fmt:
	$(CARGO) fmt --all

fmt-check:
	$(CARGO) fmt --all --check

clippy:
	$(CARGO) clippy --workspace --all-targets $(FEATURES) -- -D warnings

# The repo's own rules (entropy.toml): file size, LOC per crate, no machine-local paths, crate dependency direction.
check:
	$(CARGO) xtask check

test:
	$(CARGO) test --workspace $(FEATURES)

# Every commit reachable from HEAD, merges included. A scan can refuse; a clean one proves nothing absent.
secrets:
	$(GITLEAKS) git --log-opts="-m HEAD" --redact --no-banner .

secrets-staged:
	$(GITLEAKS) git --pre-commit --staged --redact --no-banner .

# Per worktree: the pre-commit runs fmt-check, check and secrets-staged. fence=1 also blocks pushes (worker worktrees).
hooks:
	git config extensions.worktreeConfig true
	git config --worktree core.hooksPath .githooks
	@if [ "$(fence)" = "1" ]; then git config --worktree jellymoon.pushFence 1; echo "push fence on"; fi
