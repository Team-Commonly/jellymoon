# Recording a real Claude Code or Codex session

`daemon/testdata/acp/claude-slice.jsonl` and `codex-slice.jsonl` are each one real run through the `claude-agent-acp` or `codex-acp` adapter. `TestReplayClaude` and `TestReplayCodex`
replay them through the daemon on every `go test`, with no model, no network and no sign-in. You only need this page to make a new
recording: when the adapter version moves, when the slice's calls change, or when a second machine wants to confirm it.

## What you need

- A normal sign-in for the harness you record (Claude Code or Codex) on this machine. The adapter reads it; the scripts here never touch it and no API key goes in any file.
- Node and npm on PATH (for the adapter only), and the repo's pinned Go and Bun (`make tools`).
- Nothing global: the adapter is installed into `.tools/acp/` (ignored), at an exact version.

## Record

```
make acp-tools      # npm install of the pinned adapter into .tools/acp, scripts off, no lockfile outside it
make record-acp     # one real run, then the scrub; writes daemon/testdata/acp/claude-slice.jsonl
```

`make record-acp` is `bun scripts/record-acp.mjs [out.jsonl]`. It starts a local server (`worker/dev.sh`, loopback only), makes a user
and a pod, starts the daemon with an empty working directory, registers one `claude-code` agent, and assigns it one task ("Say hello
to the pod"). The daemon wakes the agent and starts the adapter the way it starts any agent, except that the program it runs is
`e2e/testdata/acprec`, a tee. The tee copies every ACP line in both directions and points the MCP server `session/new` names at a local
proxy, so the agent's calls to the pod are logged in the same order. One run takes about a minute.

Keep it short: one task, one reply. A run that goes wrong is not worth fixing by hand-editing the file; record again.

## Codex

`make record-acp-codex` (`bun scripts/record-acp.mjs --harness codex`) does the same with a real Codex run through the pinned
`@agentclientprotocol/codex-acp` adapter and writes `daemon/testdata/acp/codex-slice.jsonl`; `TestReplayCodex` replays it against
`codex-slice.golden`. It needs this machine's normal Codex sign-in (the adapter's bundled Codex reads it; nothing here does). `make acp-tools`
installs both adapters. The scrub's existing account-line filter covers Codex's own account line too, its vocabulary names the Guardian Review step, and it replaces the model, sandbox and build settings in Codex's turn metadata with placeholders.

## Isolation

A recording must not carry this machine's own configuration. `scripts/record-acp-env.mjs` makes a fresh config directory for each recording inside the run's scratch directory and points the agent at it: `CODEX_HOME` for Codex, `CLAUDE_CONFIG_DIR` for Claude Code. So the model sees no personal settings and no global instructions file. The one thing carried over is the sign-in file (`auth.json` for Codex, `.credentials.json` for Claude Code), copied from the real config directory (or the one `CODEX_HOME` / `CLAUDE_CONFIG_DIR` already names) when it exists; nothing else is copied. `HOME` itself stays real, so a sign-in held in the operating system's keychain still works. If your Claude Code sign-in lives only in the keychain and the adapter reports no sign-in under the new directory, sign in once for that directory's variable or record on a machine with a credentials file. The scratch directory goes with the run, so the copied sign-in file does not outlive it.

## Scrub

The script scrubs the raw log before anything is kept (`scripts/acp-scrub.mjs`): pod, user, agent, run, task and message ids become
numbered placeholders, tool-use and session ids become numbered placeholders, home and working paths and ports are replaced, any URL
becomes `[url]`, tokens become `[token]`, and the adapter's sign-in status and the machine's skill list are dropped. Then it refuses
the file if anything token-, home-path-, email- or non-loopback-host-shaped is left. A refused scrub leaves the raw log in its scratch
directory (the script says where); fix the rule and run `bun scripts/record-acp.mjs --scrub <raw.jsonl>` instead of recording again. On success the scratch directory is removed; on a failure it is kept.

Streamed model text (`agent_message_chunk` and the like) is replaced by `[text]` with the line kept, because an id can be split across chunks and the replay never reads it. After the replacements the scrub allowlists what is left: every `word_xxxxxxxx` name must be a numbered placeholder or one of a short list of vocabulary names in the script, every absolute path must sit under a placeholder root, and no host except loopback may remain. The planted-line test is `bun run test:scrub`.

Before committing a recording, read it once for model-written text that quotes something it should not (the agent gets a fresh config directory, see Isolation, but the working machine and the prompt are still real), and run `make private secrets`.

## After a new recording

1. The first line names the adapter and the date. If you changed a pin, change `ACP_ADAPTERS` in the Makefile and the matching entry in
   `HARNESSES` in the script together.
2. Run `go test ./daemon/internal/daemon -run 'TestReplay(Claude|Codex)'`. It fails if the calls differ from `claude-slice.golden` or `codex-slice.golden`. Read the
   printed list against the recording's `"t":"mcp"` lines, and only then update the golden file by hand.
3. Note in `progress.md` what the run showed.
