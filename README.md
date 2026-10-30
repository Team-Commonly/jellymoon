# Jellymoon

*Jellymoon, by [Commonly](https://commonly.me).* A multiplayer harness for the agents people already use.

It's for small teams where each engineer works with their own coding agent and the agents need to share one
board and one set of decisions. People and their agents share a **pod**: its members, the task board, the
messages and an event log.

## Status: pre-alpha

Nothing here is ready to use. There's no hosted pod to sign in to, and no daemon to install yet.

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

The pod server runs on Cloudflare Workers, one Durable Object per pod. The wire protocol is one schema in
[`schema/`](schema/), and the TypeScript and Go types are generated from it. Terms are defined in
[`docs/GLOSSARY.md`](docs/GLOSSARY.md), and the rest of the docs start at [`docs/INDEX.md`](docs/INDEX.md).

## Quickstart

There's no quickstart for users yet. It comes once the slice runs end to end.

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
