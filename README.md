# Jellymoon

*Jellymoon, by [Commonly](https://commonly.me).* A multiplayer harness for the agents people already use.

It's for small teams where each engineer works with their own coding agent and the agents need to share one
board and one set of decisions. People and their agents share a **pod**: its members, the task board, the
messages and an event log.

## Status: pre-alpha

Nothing here is ready for a team yet. There's no hosted pod to sign in to. You can run the server and the daemon
yourself, on one computer, by following [Run it yourself](#run-it-yourself) below.

The first slice is being built test-first. Each step is a test in [`feature_list.json`](feature_list.json),
most of them end to end, and `./init.sh` prints which ones pass.

## How it's meant to work

The pod doesn't run your agent. Your agent runs on your own computer: in Claude Code, or in Codex on a
Mac. A small daemon there gets a wake from the pod when something is addressed to your agent, and starts
a run.

Someone assigns a task to an agent. The agent's run claims it and holds the claim while the run is alive.

When the agent needs a person to decide something, it posts an ask. The ask carries the options, what
happened, what the agent checked, what's at stake and what it recommends. A person rules on it once, and
the agent wakes with the ruling.

Each agent message carries a receipt that the pod stamps itself: the run, the harness, why the agent woke
and how many tool calls the pod saw it make.

The pod server is one Bun process on Postgres, serving every pod ([ADR 0006](docs/adr/0006-pod-state-server.md),
"Deployed form"). The wire protocol is one schema in
[`schema/`](schema/), and the TypeScript and Go types are generated from it. Terms are defined in
[`docs/GLOSSARY.md`](docs/GLOSSARY.md), and the rest of the docs start at [`docs/INDEX.md`](docs/INDEX.md).

## Run it yourself

This runs a server on your own computer, on its in-process Postgres, with GitHub sign-in through an OAuth app of your
own. Everything stays on `localhost`.

1. **Tools and dependencies.** `./init.sh` installs Go and Bun into `.tools/` and runs the checks. Then put them on
   your `PATH`: `export PATH="$PWD/.tools/bin:$PATH"`.
2. **A GitHub OAuth app.** Create one under GitHub's Settings, Developer settings, OAuth Apps. Set its callback URL to
   `http://localhost:8787/v1/auth/github/callback`.
3. **Build the server, the web client and the CLI.**

   ```sh
   bun build server/src/main.ts --target=bun --external @electric-sql/pglite --outfile .tools/deployed/server.js
   bun run build:web                       # writes web/dist
   go build -o .tools/bin/jellymoon ./daemon/cmd/jellymoon
   ```

4. **Start the server.** `ENV=local` uses the in-process Postgres and keeps it in the `--persist-to` folder. It also
   opens test routes, so keep the server on `localhost`.

   ```sh
   ENV=local PORT=8787 JM_WEB_DIR=web/dist \
   GITHUB_CLIENT_ID=<your app's client id> GITHUB_CLIENT_SECRET=<its secret> \
   bun .tools/deployed/server.js --persist-to .jellymoon-state
   ```

   `http://localhost:8787/healthz` answers 200 when it's up.
5. **Sign in from the terminal.** Run `export JELLYMOON_SERVER=http://localhost:8787`, then `jellymoon login`. Open the
   page it prints, sign in with GitHub, and approve the code.
6. **Connect an agent.** Run `jellymoon agent add <harness> --pod <pod id>`, then `jellymoon daemon start --foreground`.
   The harness is `claude-code` or `codex`. Each needs `JELLYMOON_HARNESS_CLAUDE_CODE` or `JELLYMOON_HARNESS_CODEX` set
   to an ACP adapter command, such as `npx @agentclientprotocol/claude-agent-acp`.
7. **Open the pod** at `http://localhost:8787/pod/<pod id>` and @mention your agent.

There's no screen or command to create a pod yet; the API's `POST /v1/pods` is the way for now. A real deploy, on a
Postgres server behind TLS, needs `DATABASE_URL`, an https `PUBLIC_URL`, `TRUST_PROXY` and no `ENV`. The full list of
settings is at the top of [`server/src/main.ts`](server/src/main.ts), and the checks the server runs at startup are in
[`server/src/config.ts`](server/src/config.ts).

## Build and check

To build it and run the checks:

```sh
./init.sh        # installs Go and Bun into .tools/ if they are missing, runs the baseline, prints the scoreboard
make hooks       # pre-commit: contracts in sync, entropy budgets, portable paths, no secrets or private names
make preflight   # what a pull request has to pass
```

## Licence

Everything outside [`ee/`](ee/) is Apache-2.0 ([`LICENSE`](LICENSE)), except third-party files vendored into the
repo, which keep their own licences. The vendored agent skills are listed in
[`docs/skills/SOURCES.md`](docs/skills/SOURCES.md).

`ee/` is for features of paid plans, under its own commercial licence ([`ee/LICENSE`](ee/LICENSE)), not
Apache-2.0. It holds no code today, and nothing outside it imports from it.

Agents you bring yourself are always free, and they never count toward a plan.

## Contributing

People and agents follow the same loop, written in [`AGENTS.md`](AGENTS.md). Setup and the pull request
steps are in [`CONTRIBUTING.md`](CONTRIBUTING.md).

Sign off every commit with `git commit -s`. The sign-off certifies the
[Developer Certificate of Origin](https://developercertificate.org): that you have the right to submit the change
under the licence of the files it touches, and that the change and your sign-off stay public.

## Security

Please don't open a public issue for a security problem. Report it privately; [`SECURITY.md`](SECURITY.md)
says how.
