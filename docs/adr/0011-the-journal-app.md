# 0011 · The journal app: a local app that reads your agent sessions into a page

Status: Accepted, by the owner's ruling, 2026-11-09. Sections 4 (the shell) and 7 (working copies) were decided on
2026-11-10 from the stack evaluation the owner asked for; the owner can overrule either, and section 4 says what would
change it. This note decides the shape of a new local app so the features that follow (F26 to F31) have one design to
build. The measurements below come from throwaway tests kept outside the repo; "Still open" names what they did not cover.
Reworded on 2026-11-10 by [ADR 0012](0012-restart-as-the-journal-app-in-rust.md), which made the repo Rust only: names of
removed code changed, and three facts about starting an agent moved into section 7. No measurement or decision changed.

All logic lives in core (`crates/jellymoon-core`) and is checked at the commands of the `jellymoon` binary. The shell only
draws page data, so it can be replaced without touching a rule.

## Decision

Jellymoon gains a desktop app, for the Mac first and for Windows after it. It reads the Claude Code and Codex session logs already on the computer, works out what the
person asked their agents for, and shows one **page** per local day: each **wish** as a **little one** carrying a **star**
for every finished job, and a note wherever two things the person said cannot both stand (a **clash**). A tap on the page
settles a clash, and the answer reaches the agents that are running.

It needs no account and no server for one person's page. The person's own harness sign-in does the reading; the app holds
no credential of its own.

## The parts

| Part | Where | What it does |
|---|---|---|
| **Reader** | core | Collects one local day of what the person typed, with the agent's last reply to each turn |
| **Reading call** | core, through the person's harness | One headless model call, with no tools, that turns the day into page data |
| **Page data** | typed shapes in core, with example files that tests round-trip | The contract: wishes, stars, clashes. The reading call may return nothing else |
| **Shell** | `app/` on `crates/sticker-kit` (Rust and GPUI) | Draws a page from page data using a fixed **kit**; places and animates everything itself |
| **Delivery** | a hook the app installs in the harness | Hands a ruling to a running session at its next step |

### 1. The reader takes only what the person typed

A session log mixes the person's prompts with tool results, task notices, compaction summaries and prompts sent by other
programs. The reader keeps a turn only when the log marks it as typed by a person, and attaches the agent's last reply in
that turn, cut short. It redacts anything shaped like a credential before the text leaves the process, and it skips any
folder the person has switched off.

Measured on one real day (Claude Code 2.1.322): 75 typed turns across two projects came to about 44,000 characters. The
same logs held several thousand entries that were not typed by a person.

### 2. The model returns data, and code checks it

The reading call is one prompt to the person's harness, run headless with every tool, MCP server and saved session turned
off, so the model can return text and do nothing else. A session log is untrusted input: it can hold any text an agent ever
printed. Three rules follow.

- **The output is page data and nothing else.** It is parsed into core's page types; anything that fails is dropped.
- **Every quote is checked by code.** A wish carries the person's own words. A quote that is not an exact substring of
  what they typed that day is rejected, so the model cannot put words in the person's mouth.
- **A star needs a report.** A star is a job an agent reported as finished. The prompt asks for that, and the page says the
  reading is the model's.

Measured on the same day: one call (Sonnet, 9 seconds) returned 8 wishes, 9 stars and one clash; all 10 quotes were exact.
Nobody has yet scored the wishes against the person's own judgement.

### 3. The page is arranged from a fixed kit, never generated as code

The shell is a compiled program and runs no generated code. Every page is composed from a fixed kit of drawable pieces
(the little one, a star, a note with choices, a list note, tape). The model decides what is on the page; the shell decides
where it goes and how it moves. A day that needs a piece the kit lacks falls back to a plain note until the piece is drawn.

This keeps every page in one look, keeps frame time in our hands, and means nothing in a log can change what the app does.

The look is a **theme**, kept apart from the kit's rules. Page data says what is on a page, layout says where, and a
theme says how each piece looks: its colours, fonts, paper, the art for each piece and how much things move. The sticker
journal is the first theme; a plain one, level and quiet for people who want something more serious, is built beside it
so the seam is proved by two users. Extra pieces (a pack) are added the same way. A theme or a pack is data and art
only. It carries no code, so installing one cannot change what the app does.

### 4. The shell is Rust and GPUI, one shell for Mac and Windows

The owner's ruling (2026-11-10): the app supports Windows, and Windows need not be first class. So the Mac is where the
app is polished first, and one shell serves both. The shell is a Rust program on GPUI, the UI framework of the Zed
editor, which ships on Mac, Windows and Linux.

GPUI and a native Swift shell were each built as a 50-sticker test page on the same Mac (60 Hz, Command Line Tools only).

| | GPUI (Rust, pinned commit) | Swift (AppKit, Core Animation) |
|---|---|---|
| Runs on | Mac and Windows | Mac only |
| 50 animated stickers | 60.0 frames a second, worst frame 17.1 ms, 5 to 10% of a core, 74 MB | 0.0% of a core in the app process, 43 MB; frame pacing not measured |
| Binary | 12.7 MB | 141 KB |
| Tilted text and pictures | no: only single-colour vector art and drawn paths take a transform at that commit | yes |
| Tests with no display | layout, a simulated tap and a picture, 0.95 s; the best of any candidate | layout assertions and an offscreen picture, 0.5 s |
| How it is consumed | a pinned commit; no versioned release | the system SDK |
| Menu-bar item, held key | separate crates, both registered inside GPUI's loop in the test | first-party |

Swift is the better Mac app and was the pick for one day, until Windows became a requirement. A Swift shell now and a
second shell for Windows later would mean two shells to keep in step; one shell that trails on Windows is the cheaper
promise to keep.

What this costs, and how each cost is held:

- **No tilted text.** On the page, paper and tape tilt (they are drawn paths); the words on them stay level. Fixed
  titles can be shipped as vector art, which does tilt. The kit is designed inside that limit.
- **A pinned commit, not a release.** `app/` pins one GPUI commit and moves it on purpose, with the layout and picture
  tests as the check. Rust is pinned in `rust-toolchain.toml` and installed into `.tools/` by `init.sh`.
- **Packaging is ours.** The app bundle, signing and updates are built by script on each system; GPUI provides none.
- **Speech is a helper per system.** On-device speech that covers Chinese on the Mac is Apple's framework, reached
  through a small Swift helper when F30 needs it. Windows starts with a typed wish.
- **Windows has not been run.** Nothing here was measured on Windows. From its first commit the shell builds and passes
  its layout tests on a Windows CI runner; a person on Windows tries a real build before Windows is announced.
- **Whether a held key reports its release has not been proved** on any candidate. A person holds the key on a real
  build before F30 passes.

Layout is a pure function from page data to placed pieces, asserted as data (F28). The kit (`crates/sticker-kit`) knows
nothing of Jellymoon, and it and `app/` each get a budget in `entropy.toml`. The shell links core as a library, in one
process (ADR 0012), and takes only page data and core's calls from it; core and the `jellymoon` binary never depend on
GPUI. Because every rule lives in core, a different shell can replace this one.

What would change this: the Windows CI check failing in a way a pin cannot fix, in which case Windows gets a web shell
drawing the same page data. Considered and not taken: a web view shell for both systems (Electron, Tauri), which is
quicker to draw in and can run generated pages, but is either a far larger download (Electron) or held to 60 frames a
second by the Mac's own web view (Tauri), and gives up rule 3; and game engines, which have no held global key or speech without native code.

### 5. A ruling reaches a running session through a hook

The app installs one hook in the harness's settings and removes it on uninstall. When the person settles a clash, the
ruling is written to a file; the hook hands it to each running session once, at that session's next step.

Measured (Claude Code 2.1.322, headless): a ruling written 12 seconds into a running turn arrived at the next tool call,
5 seconds later, and the agent quoted it exactly. In three further runs the agent was told to do one thing, received a
ruling to do another mid-turn, and changed course all three times.

A hook fires only when something happens, so a session sitting idle hears nothing until its next prompt. That is enough
for a ruling, which only has to arrive before the agent acts again. A new wish needs an agent to start now; that is a run
Jellymoon starts itself (section 7), which it can reach at any time. The app never types into a terminal.

### 6. One person's page is local

One person's page is local and needs no server. A clash is an **ask** (options and an open decision) and the tap is a
**ruling**; the app uses those words. A friend's page, where two people see each other's pages, is the first thing that
would need a server. That slice is not designed here; ADR 0012 says where a server would go.

Jellymoon holds no credential of its own or anyone else's. The reading call runs under whatever sign-in the person's
harness already has.

### 7. A run the app starts works in its own copy of the repo

When a wish starts a run (F30), core gives that run its own working copy, so two wishes in one repo never write over
each other: a git worktree of the person's repo on a branch of its own, removed with its branch when the run completes.
A failed run's copy is kept for a look, the newest five only. Cleanup removes only a folder directly inside its own
root, runs no hook, and reads no file from the run's folder, since the agent could have rewritten them. The work stays
in the person's repo, where they already look for it.

How the agent is started and stopped, carried over from the Go program this repo used to hold (tag `harness-final`):

- **A child over ACP, with an allow-listed environment.** The agent is a child process speaking ACP on stdio. It gets a
  short list of variables (`PATH`, `HOME` and the like) and never inherits the parent's environment.
- **Cancel, then signal, then kill.** A stop sends `session/cancel` first and waits; an agent deaf to that is signalled,
  and one deaf to the signal is killed.
- **A permission request is refused by default.** Where the person has turned allowing on, the answer is for that one
  call. It is never "always": a remembered allow could not be taken back.

LayerFS, a filesystem that gives each agent a writable copy over shared committed history, was evaluated for the same
seam. Its macOS support ran here from an unreleased branch with no installer and no extra permission, which is
promising, but it has no macOS release yet and no path from its copy back into the person's git history. It goes behind
the same seam when it ships for macOS and that path exists; nothing above the seam changes.

## Still open

- **The reading has been tried on one person's one day.** Whether it holds on other people's sessions, on a week, or on a
  day with many small sessions is unknown.
- **Codex.** The measured day had no typed Codex sessions. The reader's Codex path and a Codex hook are untested.
- **Delivery in an interactive session.** The delivery tests ran headless. The hook path is the same, but an open terminal
  session has not been tried, nor the hook that fires with the person's next prompt.
- **Cost to the person.** The reading call spends the person's own plan. How often it runs, and on which model, is not set.
- **A 120 Hz display.** Both test pages ran at 60 Hz.
- **Windows.** Nothing was run on Windows. Where the reader finds session logs there, the hook's script, and the
  floating window all need their own check.
- **The oldest macOS the app supports.** On-device speech as chosen needs macOS 26.
- **Packaging.** The app bundle, signing and updates are assembled by script; that has not been tried on either system.
- **A second person.** The friend's page, and whether a page ever leaves the computer, need their own decision.
