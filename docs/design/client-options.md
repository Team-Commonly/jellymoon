# Client strategy: options paper

2026-10-29. Research only. "Local" means earlier research notes and the repo's own docs. Everything about another product carries a URL; where I could not confirm a claim, it says so.

**Read with:** `ux-concepts.html`, the five-paradigm prototype on the same scenario. It recommends Inbox as home and Chat as the pod surface, which matches screens 2 and 3 below.

## 1. Recommendation

**Web-first (option 1) for the slice and the first release. Treat Tauri as a packaging step later (option 2), and the TUI as an optional second view on the same API (option 3), not the first client.**

1. The owners' latest call already says web first with a strong API. The evidence backs it for one reason: the people who must approve a card are not all terminal people. Every multi-user comparable we studied that has humans as first-class members ships a web client. The TUI-first products we studied (herdr) are single-user.
2. The engine is HTTP plus a typed WebSocket (`docs/protocol.md`), so **a web client is a thin view**. The e2e test stays API-level, with no UI (`docs/development/workflow.md`). The browser is checked only on a few routes.
3. Web gives the one thing a TUI cannot: a teammate joins from a link. OAuth for connectors is a browser redirect there. In a TUI it needs a browser handoff anyway.
4. Don't take on Tauri in the 48-72 h slice. It adds a Rust toolchain, per-target sidecar binaries, a signing key, and an Apple Developer account ($99/yr) before anyone has used the product. The web app wrapped later is the cheap way to reach "desktop". Revisit once someone asks for tray notifications or one-click daemon install.
5. Keep the daemon headless (login via device flow, `status`, `join`). Its local unix socket and the shared envelope (`docs/protocol.md`, "Socket events") make a Bubble Tea view a later 1-2 day add, and herdr shows a real audience for it (see section 3).

## 2. Slice client scope (web)

**Screens (4, one layout shell):**
1. `/device` — device-code approval for the daemon (the one browser check the contract already names).
2. `/pod/:id` — stream: headline cards (summary line, status, `receipt` chip), folded body, composer.
3. Needs-you queue (a filter of the same stream) with keyboard approve.
4. `/invite/:token` — join, so a non-terminal teammate reaches the pod with no install.

Not in the slice: board view, agent rail, settings, desktop shell, mobile, connector management UI (the Linear read is a card in the stream plus one "Connect" button).

**Flows and their check tier:**

| Flow | Check |
|---|---|
| Create task, agent claims, posts card, human rules, agent wakes (the slice) | API e2e (`make e2e`, no UI) — the gate |
| Card reducer, receipt render, key map (`a`, `1..n`, `j/k`) | Component tests (vitest + Testing Library), no browser |
| Sign in with GitHub, `/device` approval | **Playwright**, 390 and 1440, 1 spec |
| Open pod, see card, press `a`, ruled state appears (WS live) | **Playwright**, 1 spec against `podfake`/preview |
| Invite link join | **Playwright**, 1 spec |
| Layout invariants (no h-scroll, no clipped text) | Playwright + `impeccable detect --cdp` per route |
| Visual baselines | `toHaveScreenshot` at 2 viewports, same container as CI only |

Budget: roughly 5 browser specs, under 90 s. Estimated 2 days for a writer on React + Vite + generated types for 4 screens; that is my estimate, not measured.

**Connecting a third-party tool:** a "Connect Linear" button opens the vendor's remote-MCP OAuth in a normal browser tab and returns to the pod; the server vault holds the token. Daemon-side login uses the device flow, which the daemon already needs.

## 3. Trade-offs

| | 1. Web-first (React + Vite on Worker assets) | 2. Web + Tauri, Go daemon as sidecar | 3. TUI-first (Go + Bubble Tea) |
|---|---|---|---|
| **UX for the slice** | Best for cards, receipts, links, mouse and keys. Same surface for non-terminal teammates and phones. | Same UI plus tray, native notifications, daemon lifecycle owned by the app. | Fast keyboard approve; folds and receipts work. Weak at rich bodies, images, and sharing a link. Teammates need a terminal. |
| **Third-party OAuth** | Native browser redirect. | Same; deep link back needs extra config. | Always a browser handoff. Standard fix is the device flow (RFC 8628), made for input-constrained clients: [rfc8628](https://datatracker.ietf.org/doc/html/rfc8628). `gh auth login` uses a one-time code in the browser: [gh manual](https://cli.github.com/manual/gh_auth_login). |
| **Non-terminal teammate** | Opens an invite link. | Downloads an app. | Cannot, without install and shell. |
| **Verification** | API e2e main. Playwright for ~5 routes; screenshot baselines are per OS/browser, so run them in one pinned environment: [Playwright snapshots](https://playwright.dev/docs/test-snapshots). impeccable: 62 deterministic rules, CLI, can run over `--cdp` on a rendered page ([repo](https://github.com/pbakaus/impeccable)). | Same as 1, plus an untested shell layer (sidecar spawn/kill, updater). Tauri's sidecar page does not describe termination or orphan handling: [sidecar docs](https://v2.tauri.app/develop/sidecar/). | Fastest and most deterministic: teatest golden frames, byte-stable with a fixed color profile. teatest is experimental (`x/exp`, untagged): [pkg.go.dev](https://pkg.go.dev/github.com/charmbracelet/x/exp/teatest). impeccable does not cover terminals. |
| **Flake risk** | Medium (timing, fonts, baselines); mitigated by few specs. | Medium-high (adds OS dialogs, signing). | Low. |
| **Time to first usable (slice)** | ~2 days for 4 screens (estimate). | 1 + ~1-2 days for scaffolding, sidecar triples, signing (estimate). | ~1.5 days, but not usable by non-terminal people. |
| **Added to the Go daemon** | Nothing beyond headless + device login. | Run as a child; must handle parent exit, port/socket choice, and a stable status API. Sidecar binaries need a target-triple suffix per platform: [docs](https://v2.tauri.app/develop/sidecar/). | A local unix-socket protocol and a TUI package; Bubble Tea dependency. |
| **Distribution / update** | A URL. Deploy = update. Daemon installs with `install.sh` or Homebrew (herdr's pattern). | Signed installer. Updates must be signed with a Tauri key; losing the key strands installs: [updater](https://v2.tauri.app/plugin/updater/). macOS needs Developer ID + notarization ($99/yr): [signing](https://v2.tauri.app/distribute/sign/macos/). | One static binary, same as the daemon. No GUI-signing cost. |

## 4. How comparable products chose

Verified:
- **Multica** (closest to us): web (Next.js 16), Electron desktop sharing web UI packages, Expo mobile, CLI/API, local daemon; no TUI listed. Desktop sets up the daemon automatically: [repo](https://github.com/multica-ai/multica).
- **Tutti:** Electron desktop; its open edition is one person with several agents; group rooms are in the closed VM edition ([repo](https://github.com/tutti-os/tutti)).
- **Raft/Slock:** React 19 + Vite web client, Socket.IO, local daemon ([repo](https://github.com/botiverse/raft-source)).
- **herdr:** a Rust TUI over a headless daemon, no multi-user or socket auth; Apache-2.0 now ([repo](https://github.com/herdrdev/herdr)). The proof that a TUI audience exists, and that it is a single-user one.
- **Claude Code desktop:** a Code tab inside the Claude desktop app on macOS, Windows, and a Linux beta; same settings files as the CLI, with diff review and approve/reject UI the terminal lacks: [docs](https://code.claude.com/docs/en/desktop).
- **Zed:** native Rust with GPUI, macOS/Linux/Windows: [repo](https://github.com/zed-industries/zed). **Warp:** native Rust UI crates (MIT) with AGPL app: [repo](https://github.com/warpdotdev/warp). Both are native because they are editors/terminals; neither is a model for a thin collaboration client.
- **Linear:** desktop for macOS and Windows plus web, iOS, Android: [download](https://linear.app/download). The page does not say Electron.

Not confirmed (don't cite as fact): whether Conductor is Tauri (a review says Mac-native, not Electron; [source](https://rywalker.com/research/conductor)); Codex app framework (coverage says native macOS, no toolkit named; [docs](https://developers.openai.com/codex/app)); Sculptor is Electron (single HN comment only; [thread](https://hn.svelte.dev/item/45427697)); Claude Code desktop's framework.

Read: the pattern is **web UI + a local daemon + a desktop wrapper later**, with the wrapper usually Electron where known (Multica, Tutti). No studied product started TUI-first and got a team audience.

## 5. Risks

- **The earlier plan was TUI-first.** Its TUI lane and the e2e steps that drive it need rewriting to API/WS calls, or the e2e keeps a hidden TUI dependency.
- **"Keyboard approve" must be real on web**: focus handling, shortcuts in text inputs, and screenshot proof. Test with key scripts in Playwright, not mouse clicks.
- **Web cannot start the daemon.** A non-terminal teammate with a local agent still needs one install. Mitigation: a copy-paste one-liner on `/device` now; Tauri wrapper later (Multica's desktop does this automatically).
- **Visual baselines flake across machines.** Pin one container; use `threshold`/`maxDiffPixels`; keep specs few.
- **Tauri adds a Rust toolchain and two secrets** (updater key, Apple notarization). Deferring avoids both until the UI is stable. Sidecar kill-on-exit is undocumented on the page I read; test it before committing.
- **teatest is experimental**; if TUI returns later, pin the commit.
- **Unverified claims above** (Conductor, Codex, Sculptor, Linear framework) should not appear in public text.
- **Estimates** (2 days, 1-2 days) are mine and unmeasured.
