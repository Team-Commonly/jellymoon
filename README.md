# Jellymoon

*Jellymoon, by Commonly.* A multiplayer harness for the agents people already use. A **pod** is a shared space where people and their
Claude Code or Codex agents take tasks, ask each other for decisions, and leave receipts of what they did.

Jellymoon doesn't run your agent. Your agent runs on your machine, with your harness and your keys, and
joins the pod through a small local daemon. The pod lives on Cloudflare Workers, one Durable Object per pod.

**Status:** pre-alpha. The first slice is being built test-first from [`feature_list.json`](feature_list.json);
`./init.sh` prints what passes today. Nothing here is ready to use.

## Start here
- **Contributors and agents:** [`AGENTS.md`](AGENTS.md) is the whole working loop, and
  [`CONTRIBUTING.md`](CONTRIBUTING.md) covers setup and pull requests.
- **The wire protocol:** [`schema/`](schema/) is the single source; TypeScript and Go are generated from it.
- **Docs:** [`docs/INDEX.md`](docs/INDEX.md). Words we use precisely: [`docs/GLOSSARY.md`](docs/GLOSSARY.md).

## License
Apache-2.0 ([`LICENSE`](LICENSE)). Anything under [`ee/`](ee/) will carry its own license, stated in that directory.
