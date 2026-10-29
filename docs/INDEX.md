# Docs index

Every file under `docs/` is listed here with one line. `make entropy` doesn't check this yet. A
per-wave unit compares this list with `ls docs/**` and fixes the drift.

| File | What it is for |
|---|---|
| `GLOSSARY.md` | Terms used precisely across code, schema and docs |
| `protocol.md` | The wire protocol in plain words: ids, tokens, HTTP routes, events, messages, tasks, operations (MCP is an adapter over them) and errors. Generated from `schema/` by `make gen`; never edit by hand |
| `development/workflow.md` | How Jellymoon is built: the long-running harness, stage skills, entropy gate, review and merge |
| `development/verifier.md` | The verifier protocol: what a fresh-context reviewer checks, and its verdict format |
| `adr/0001-dev-tooling.md` | Dev dependencies, the entropy budgets and the slice LOC caps |
| `adr/0002-claims-and-leases.md` | When a task claim's lease renews, expires and is released; the unknowns left open |
| `skills/SOURCES.md` | Where the vendored stage skills in `.agents/skills/` come from, their pins and how to update them |
| `design/protocol.md` | Protocol review: what the schema decides, the open questions and recommendations |
| `design/connector-options.md` | Connectors options paper (OSS-first): Linear read through a pod grant, the write gate |
| `design/client-options.md` | Client strategy options paper: web-first slice, four screens |
| `design/ux-concepts.html` | Six interaction paradigms (Round table included) as clickable prototypes on one scenario, with verdicts. Exploration only: the client direction is open until the review with Sam (F18) |
| `design/generative-ui.md` | Generative UI options paper: a fixed, schema-declared card catalogue, with every number cited |
