# 0011 · The journal app: a local app that reads your agent sessions into a page

Status: Proposed, 2026-11-09. Waiting on the owner's ruling. It adds no code. It decides the shape of a new local app so the
features that follow have one design to build. The measurements below come from throwaway tests kept outside the repo;
"Still open" names what they did not cover.

## Decision

Jellymoon gains a Mac app. It reads the Claude Code and Codex session logs already on the computer, works out what the
person asked their agents for, and shows one **page** per local day: each **wish** as a **little one** carrying a **star**
for every finished job, and a note wherever two things the person said cannot both stand (a **clash**). A tap on the page
settles a clash, and the answer reaches the agents that are running.

It needs no account and no server for one person's page. The person's own harness sign-in does the reading; the app holds
no credential of its own.

## The parts

| Part | Where | What it does |
|---|---|---|
| **Reader** | the daemon (`daemon/`) | Collects one local day of what the person typed, with the agent's last reply to each turn |
| **Reading call** | the daemon, through the person's harness | One headless model call, with no tools, that turns the day into page data |
| **Page data** | `schema/` | The contract: wishes, stars, clashes. The reading call may return nothing else |
| **Shell** | `app/` (new, Rust and GPUI) | Draws a page from page data using a fixed **kit**; places and animates everything itself |
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

- **The output is page data and nothing else.** It is validated against the schema; anything that fails is dropped.
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

### 4. The shell is Rust and GPUI

A throwaway GPUI window (transparent, always on top, one character with five simulated ropes) held 60.0 frames a second
for 14 seconds on a 60 Hz display, with the worst frame at 17.1 ms, 6 to 15% of one core and about 65 MB of memory. The
binary was 8.9 MB.

Known gaps, read from the GPUI source at the pinned commit: no menu-bar item, no radial gradient or blur on drawn paths,
and no option for a click-through window. The flat sticker look needs none of the three. A menu-bar item, if wanted, is a
small piece of native code beside GPUI.

This adds a language and a toolchain to the repo. Rust is pinned in `.tool-versions` and installed into `.tools/` by
`init.sh`, like Go and Bun; GPUI is pinned to one commit. `app/` gets its own budget in `entropy.json` and may import
nothing from `daemon/`, `server/` or `web/`: it talks to the daemon only through page data and the daemon's local commands.

Considered and not taken: a web view shell. It is quicker to draw in and can run generated pages, but it gives up the
frame budget and rule 3.

### 5. A ruling reaches a running session through a hook

The app installs one hook in the harness's settings and removes it on uninstall. When the person settles a clash, the
ruling is written to a file; the hook hands it to each running session once, at that session's next step.

Measured (Claude Code 2.1.322, headless): a ruling written 12 seconds into a running turn arrived at the next tool call,
5 seconds later, and the agent quoted it exactly. In three further runs the agent was told to do one thing, received a
ruling to do another mid-turn, and changed course all three times.

A hook fires only when something happens, so a session sitting idle hears nothing until its next prompt. That is enough
for a ruling, which only has to arrive before the agent acts again. A new wish needs an agent to start now; that is a run
the daemon starts itself (ADR 0005), which it can reach at any time. The app never types into a terminal.

### 6. How it sits beside the pod

One person's page is local and needs no pod. A clash is an **ask** in the pod's sense (options and an open decision) and
the tap is a **ruling**; the app uses those words and, where a pod is present, the same records. A friend's page is where
the pod comes in: two people in one pod can see each other's pages. That slice is not designed here.

The token rule holds: the app and the reading call hold no `jmc_`, `jmu_` or third-party credential. The reading call
runs under whatever sign-in the person's harness already has.

## Still open

- **The reading has been tried on one person's one day.** Whether it holds on other people's sessions, on a week, or on a
  day with many small sessions is unknown.
- **Codex.** The measured day had no typed Codex sessions. The reader's Codex path and a Codex hook are untested.
- **Delivery in an interactive session.** The delivery tests ran headless. The hook path is the same, but an open terminal
  session has not been tried, nor the hook that fires with the person's next prompt.
- **Cost to the person.** The reading call spends the person's own plan. How often it runs, and on which model, is not set.
- **A 120 Hz display.** The frame test ran at 60 Hz.
- **A second person.** The friend's page, and whether a page ever leaves the computer, need their own decision.
