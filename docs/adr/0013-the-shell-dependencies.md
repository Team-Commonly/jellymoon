# 0013 · What the kit and the shell depend on

Status: Accepted with F28, 2026-11-10. [ADR 0011](0011-the-journal-app.md), section 4, chose GPUI for the shell. This
note lists what that choice and the `layout` command brought into the workspace, and how each is moved.

## Decisions

1. **GPUI comes from the Zed repository at one commit**: `gpui`, `gpui_platform` and, on the Mac, `gpui_apple`, all at
   the `rev` in the root `Cargo.toml`. The crates.io release of GPUI lags the repository by months and lacks the
   headless renderer the draw tests use. To move the pin, change the one `rev` on all three lines, run
   `cargo update -p gpui`, run `make preflight` on a Mac, look at the pictures the snapshot example makes, and compare
   the Linux package list in `.github/workflows/ci.yml` with Zed's `script/linux` at the new commit. GPUI is not
   patched; what it cannot do is worked around in the kit or left out.
2. **Only the kit and the shell see GPUI.** Core and the `jellymoon` binary never depend on it (`entropy.toml` holds
   the direction, and its `[forbidden]` table bars every package whose name starts with `gpui`). In the kit it is the
   default feature `gpui`, in the shell the default feature `window`; without them both crates build in seconds and
   the `layout` command still works. The lock file grew from 50 packages to 691, nearly all of it GPUI's.
3. **`ttf-parser` reads advance widths from the bundled font files** for `BundledMeasure`, so `layout` prints the same
   numbers on every system. It has no dependencies, no `unsafe`, and GPUI's own graph already held it, so it adds no
   package to the lock. Rejected: `skrifa` with `read-fonts` (a second font stack for one table lookup), `fontdue`
   (a rasteriser; only widths are needed), `rustybuzz` (full shaping would close most of the gap to the window, at
   the cost of a shaper that must then agree with the one in each operating system, which it cannot for a system
   font). The window keeps GPUI's own measuring; `docs/development/app.md` has the numbers.
4. **The kit's `toml` gains the `display` feature**, to write a theme back out in its tests. No new package.
5. **Features for where a build runs.** `runtime_shaders` compiles the Metal shaders when the app starts, for a Mac
   without Xcode's `metal`; `make` turns it on by itself when `xcrun -f metal` finds nothing. `headless` renders
   without a window for the draw tests and the snapshot example; `make` and CI turn it on for the Mac only, where it
   has been seen to work. On Linux the kit's `gpui` feature also turns on GPUI's Wayland and X11 backends, since a
   Linux build without one can open no window; on other systems those two add nothing.
6. **GPUI's features reach core in a workspace build.** Cargo builds each package once for the whole workspace, so
   `cargo test --workspace` gives core and the binary a `serde_json` with GPUI's `preserve_order` on, and
   `cargo test -p jellymoon` one without. Nothing in core or the binary may depend on which: they use their own types,
   not `serde_json::Value`, and a test that writes JSON with `json!` lists its keys in alphabetical order.
7. **One test runs `sh`, on the Mac only.** The repo keeps `sh` out of tests so they pass on Windows. Installing on a
   Mac is a shell script by its nature (`install.sh`, and `app/macos/bundle.sh`, which makes the app bundle), so
   F33's tests of them are compiled for macOS only, with one more for other Unix systems that checks the script
   refuses them. `jellymoon uninstall` is Rust and its test runs everywhere. This is the only exception; a Windows
   installer gets its own test in its own language. F33 adds no package: the release is built by a workflow, and
   the script uses only what every Mac has (`curl`, `shasum`, `tar`, `xattr`).

## Not decided here

Whether Linux is a supported system: ADR 0011 names the Mac first and Windows after. CI builds and tests all three so
that a break is seen the day it happens; nobody has opened the window on Linux or Windows yet.
