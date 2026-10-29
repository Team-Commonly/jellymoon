# Docs index

Every file under `docs/` is listed here with one line. `make entropy` doesn't check this yet. A
per-wave unit compares this list with `ls docs/**` and fixes the drift.

| File | What it is for |
|---|---|
| `GLOSSARY.md` | Terms used precisely across code, schema and docs |
| `development/workflow.md` | How Jellymoon is built: the long-running harness, stage skills, entropy gate, review and merge |
| `development/verifier.md` | The verifier protocol: what a fresh-context reviewer checks, and its verdict format |
| `adr/0001-dev-tooling.md` | Dev dependencies, the entropy budgets and the slice LOC caps |
| `skills/pstack-deslop/` | Vendored pstack deslop skill (MIT), used by the verifier and the removal-only unit |
