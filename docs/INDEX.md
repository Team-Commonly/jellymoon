# Docs index

Every file under `docs/` is listed here with one line. `make entropy` doesn't check this yet. A
per-wave unit compares this list with `ls docs/**` and fixes the drift.

| File | What it is for |
|---|---|
| `GLOSSARY.md` | Terms used precisely across code, schema and docs |
| `development/workflow.md` | How Jellymoon is built: the long-running harness, stage skills, entropy gate, review and merge |
| `development/verifier.md` | The verifier protocol: what a fresh-context reviewer checks, and its verdict format |
| `adr/0001-dev-tooling.md` | Dev dependencies, the entropy budgets and the slice LOC caps |
| `skills/SOURCES.md` | Where the vendored stage skills in `.agents/skills/` come from, their pins and how to update them |
| `design/protocol.md` | Protocol review: what the schema decides, the open questions and recommendations |
| `design/connector-options.md` | Connectors options paper (OSS-first): Linear read through a pod grant, the write gate |
| `design/client-options.md` | Client strategy options paper: web-first slice, four screens |
| `design/ux-concepts.html` | Six interaction paradigms (Round table included) as clickable prototypes on one scenario, with verdicts |
| `design/generative-ui.md` | Generative UI options paper: a fixed, schema-declared card catalogue, with every number cited |
